#ifndef OPENDROBO_DRIVERKIT_SESSION_H
#define OPENDROBO_DRIVERKIT_SESSION_H

#include <DriverKit/IOReturn.h>
#include <stdint.h>
#include "ControllerTransport.h"

class IOService;
class IOPCIDevice;
class IODispatchQueue;
class IOInterruptDispatchSource;
class OSAction;

/* Compile-only SDK resource adapter. The eventual IOService must keep this
 * object and supplied queue alive through every asynchronous close callback;
 * do not cancel the queue before close completes. All calls, including
 * controller work, belong to the supplied serial queue. No method is called by
 * the application or a static initializer. There is no implicit destructor
 * cleanup: a failed cancellation must retain its live callback resources.
 * Closing a PCI session is NOT proof that earlier DMA transactions drained. */
class ODTBTDriverKitSession {
public:
    enum class State { Empty, Open, Closing, FailedClose, Closed };
    kern_return_t Open(IOPCIDevice *provider, IOService *owner,
                       IODispatchQueue *queue, OSAction *interruptAction,
                       uint32_t interruptIndex);
    kern_return_t EnableInterrupts();
    kern_return_t EnableBusMaster(const ODTBTController *controller);
    kern_return_t BeginClose(void (^completion)(kern_return_t));
    int Read32(uint8_t bar, uint32_t offset, uint32_t *value);
    int Write32(uint8_t bar, uint32_t offset, uint32_t value);
    int Order();
    uint64_t BarSize(uint8_t bar) const { return bar < 2 ? sizes[bar] : 0; }
    State GetState() const { return state; }
    bool InterruptsDrained() const { return state == State::Closed; }
    IOPCIDevice *Provider() const { return state == State::Open ? pci : nullptr; }

    ODTBTDriverKitSession() = default;
    ODTBTDriverKitSession(const ODTBTDriverKitSession &) = delete;
    ODTBTDriverKitSession &operator=(const ODTBTDriverKitSession &) = delete;
private:
    void FinishClose();
    void ReleaseOpenFailure();
    State state = State::Empty;
    IOPCIDevice *pci = nullptr;
    IOService *client = nullptr;
    IODispatchQueue *serialQueue = nullptr;
    IOInterruptDispatchSource *interrupt = nullptr;
    bool interruptEnabled = false;
    uint8_t indices[2] = {};
    uint64_t sizes[2] = {};
};
ODTBTControllerIO od_tbt_driverkit_controller_io(ODTBTDriverKitSession *session);
#endif
