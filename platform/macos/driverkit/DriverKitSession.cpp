#include "DriverKitSession.h"
#include "DriverKitDma.h"
#include <DriverKit/IODispatchQueue.h>
#include <DriverKit/IOInterruptDispatchSource.h>
#include <DriverKit/IOLib.h>
#include <DriverKit/IOService.h>
#include <PCIDriverKit/IOPCIDevice.h>

static constexpr uint64_t kCommandRegister = 4;
static constexpr uint16_t kMemorySpace = 2;
static constexpr uint16_t kBusMaster = 4;

void ODTBTDriverKitSession::ReleaseOpenFailure() {
    if (pci) {
        pci->Close(client);
        pci->release();
        pci = nullptr;
    }
    if (serialQueue) { serialQueue->release(); serialQueue = nullptr; }
    if (client) { client->release(); client = nullptr; }
    state = State::Closed;
}

kern_return_t ODTBTDriverKitSession::Open(IOPCIDevice *provider, IOService *owner,
                                         IODispatchQueue *queue, OSAction *action,
                                         uint32_t interruptIndex) {
    if (state != State::Empty || !provider || !owner || !queue || !action)
        return kIOReturnBadArgument;
    auto result = provider->Open(owner, 0);
    if (result != kIOReturnSuccess) return result;
    pci = provider; pci->retain();
    client = owner; client->retain();
    serialQueue = queue; serialQueue->retain();
    uint16_t command = UINT16_MAX;
    pci->ConfigurationRead16(kCommandRegister, &command);
    if (command == UINT16_MAX) { ReleaseOpenFailure(); return kIOReturnNoDevice; }
    /* Leave bus mastering disabled until the caller has established ownership
     * and the queue handshake. This is not a device-reset or DMA-drain proof. */
    pci->ConfigurationWrite16(kCommandRegister, (command & ~kBusMaster) | kMemorySpace);
    uint16_t observed = UINT16_MAX;
    pci->ConfigurationRead16(kCommandRegister, &observed);
    if (observed == UINT16_MAX || (observed & (kMemorySpace | kBusMaster)) != kMemorySpace) {
        ReleaseOpenFailure(); return kIOReturnNotReady;
    }
    for (uint8_t bar = 0; bar < 2; ++bar) {
        uint8_t type = kPCIBARTypeIO;
        result = pci->GetBARInfo(bar, &indices[bar], &sizes[bar], &type);
        const bool memoryType = type == kPCIBARTypeM32 || type == kPCIBARTypeM64 ||
                                type == kPCIBARTypeM32PF || type == kPCIBARTypeM64PF;
        if (result != kIOReturnSuccess || !memoryType || sizes[bar] < 4 ||
            sizes[bar] > UINT32_MAX) {
            ReleaseOpenFailure();
            return result == kIOReturnSuccess ? kIOReturnUnsupported : result;
        }
    }
    if (indices[0] == indices[1]) { ReleaseOpenFailure(); return kIOReturnUnsupported; }
    result = IOInterruptDispatchSource::Create(pci, interruptIndex, serialQueue, &interrupt);
    if (result != kIOReturnSuccess) {
        if (interrupt) state = State::FailedClose;
        else ReleaseOpenFailure();
        return result;
    }
    result = interrupt->SetHandler(action);
    state = result == kIOReturnSuccess ? State::Open : State::FailedClose;
    if (result != kIOReturnSuccess) {
        /* A created source is cancelled asynchronously even on setup failure.
         * The caller must await BeginClose rather than free this object. */
        return result;
    }
    return kIOReturnSuccess;
}

kern_return_t ODTBTDriverKitSession::EnableInterrupts() {
    if (state != State::Open || !interrupt) return kIOReturnNotReady;
    auto result = interrupt->SetEnableWithCompletion(true, nullptr);
    if (result == kIOReturnSuccess) interruptEnabled = true;
    return result;
}

kern_return_t ODTBTDriverKitSession::EnableBusMaster(const ODTBTController *controller) {
    if (state != State::Open || !pci || !interruptEnabled ||
        !od_tbt_controller_running_for(controller, this)) return kIOReturnNotReady;
    uint16_t command = UINT16_MAX;
    pci->ConfigurationRead16(kCommandRegister, &command);
    if (command == UINT16_MAX) return kIOReturnNoDevice;
    pci->ConfigurationWrite16(kCommandRegister, command | kMemorySpace | kBusMaster);
    uint16_t observed = UINT16_MAX;
    pci->ConfigurationRead16(kCommandRegister, &observed);
    if (observed == UINT16_MAX) return kIOReturnNoDevice;
    return (observed & (kMemorySpace | kBusMaster)) == (kMemorySpace | kBusMaster)
        ? kIOReturnSuccess : kIOReturnNotReady;
}

int ODTBTDriverKitSession::Read32(uint8_t bar, uint32_t offset, uint32_t *value) {
    if (state != State::Open || !value || bar > 1 || (offset & 3) ||
        offset > sizes[bar] || sizes[bar] - offset < 4) return -1;
    *value = UINT32_MAX;
    pci->MemoryRead32(indices[bar], offset, value);
    return 0; // UINT32_MAX is valid copied data; protocol status reads classify it.
}

int ODTBTDriverKitSession::Write32(uint8_t bar, uint32_t offset, uint32_t value) {
    if (state != State::Open || bar > 1 || (offset & 3) ||
        offset > sizes[bar] || sizes[bar] - offset < 4) return -1;
    pci->MemoryWrite32(indices[bar], offset, value);
    return 0; // The SDK accessor is void; protocol readback detects removal.
}

int ODTBTDriverKitSession::Order() {
    if (state != State::Open) return -1;
    OSSynchronizeIO();
    return 0;
}

void ODTBTDriverKitSession::FinishClose() {
    interrupt->release(); interrupt = nullptr;
    /* PCI Close disables bus mastering and memory-space access. Outstanding
     * DMA leases still require an independent completion/quiescence proof. */
    ReleaseOpenFailure();
}

kern_return_t ODTBTDriverKitSession::BeginClose(void (^completion)(kern_return_t)) {
    if ((state != State::Open && state != State::FailedClose) || !interrupt || !completion)
        return kIOReturnBadArgument;
    state = State::Closing; // Block new MMIO and submissions before callbacks drain.
    interruptEnabled = false;
    auto result = interrupt->SetEnableWithCompletion(false, ^{
        auto cancelled = interrupt->Cancel(^{
            FinishClose();
            completion(kIOReturnSuccess);
        });
        if (cancelled != kIOReturnSuccess) {
            state = State::FailedClose;
            completion(cancelled);
        }
    });
    if (result != kIOReturnSuccess) state = State::FailedClose;
    return result;
}

static int controller_read(void *context, uint8_t bar, uint32_t offset, uint32_t *value) {
    return static_cast<ODTBTDriverKitSession *>(context)->Read32(bar, offset, value);
}
static int controller_write(void *context, uint8_t bar, uint32_t offset, uint32_t value) {
    return static_cast<ODTBTDriverKitSession *>(context)->Write32(bar, offset, value);
}
static int controller_order(void *context) {
    return static_cast<ODTBTDriverKitSession *>(context)->Order();
}
static int controller_write_request(void *context, ODDmaLease *lease, const void *bytes, uint32_t length) {
    auto session = static_cast<ODTBTDriverKitSession *>(context);
    if (!session->Provider() || !lease || lease->backend.context != session->Provider()) return -1;
    return od_driverkit_dma_write_request(lease, bytes, length);
}
ODTBTControllerIO od_tbt_driverkit_controller_io(ODTBTDriverKitSession *session) {
    if (!session) return {};
    return {session, controller_read, controller_write, controller_order, controller_write_request};
}
