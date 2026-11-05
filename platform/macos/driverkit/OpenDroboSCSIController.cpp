#include <new>
#include <initializer_list>
#include "OpenDroboSCSIController.h"
#include "DriverKitSession.h"
#include "DriverKitDma.h"
#include "ScsiResponses.h"
#include "ScsiTaskTable.h"
#include <DriverKit/DriverKit.h>
#include <DriverKit/IOBufferMemoryDescriptor.h>
#include <DriverKit/IODispatchQueue.h>
#include <DriverKit/IOTimerDispatchSource.h>
#include <DriverKit/IOLib.h>
#include <DriverKit/OSDictionary.h>
#include <PCIDriverKit/IOPCIDevice.h>
#include <stdio.h>
#include <string.h>

struct ServiceTask {
    uint64_t id = 0;
    ODTBTTaskInput input = {};
    bool used = false;
    IOBufferMemoryDescriptor *request_buffer = nullptr, *data_buffer = nullptr;
    ODDmaLease request = {}, payload = {};
    ServiceTask() { (void)od_dma_lease_init(&request); (void)od_dma_lease_init(&payload); }
};
struct OpenDroboSCSIController_IVars {
    ODTBTDriverKitSession session;
    ODTBTController storage = {}, management = {};
    ODScsiTaskTable tasks = {};
    ODScsiIdentity identity = {};
    ServiceTask slots[OD_TBT_CONTROLLER_SLOTS];
    IOPCIDevice *provider = nullptr; // Borrowed from Start until Stop's super dispatch.
    IODispatchQueue *queue = nullptr, *auxiliary = nullptr;
    IOTimerDispatchSource *timer = nullptr;
    OSAction *interrupt_action = nullptr, *timer_action = nullptr;
    IOService *stop_provider = nullptr;
    uint32_t next_hba_id = 1, auxiliary_work = 0, pending_power = 0;
    bool initialized = false, engines_initialized = false, running = false;
    bool stopping = false, sources_closed = false, stop_pin = false;
    bool target_present = false, target_destroying = false, power_pending = false;
    bool startup_failed = false, auxiliary_closing = false, auxiliary_closed = false;
};
struct ServicePin {
    OpenDroboSCSIController *service;
    explicit ServicePin(OpenDroboSCSIController *s) : service(s) { service->retain(); }
    ~ServicePin() { service->release(); }
};

static constexpr uint32_t kLocalIllegalCommand = 0x100;
static const OSDispatchMethod kSuperDispatch =
    (OSDispatchMethod)&IOUserSCSIParallelInterfaceController::_Dispatch;
static void maybe_finish_stop(OpenDroboSCSIController *);
static void begin_shutdown(OpenDroboSCSIController *);

static uint64_t uptime_ns() {
    mach_timebase_info_data_t timebase = {};
    if (mach_timebase_info(&timebase) != KERN_SUCCESS || !timebase.denom) return 0;
    const unsigned __int128 ns = (unsigned __int128)mach_absolute_time() * timebase.numer / timebase.denom;
    return ns <= UINT64_MAX ? (uint64_t)ns : 0;
}
static uint64_t uptime_ms() {
    uint64_t ns = uptime_ns();
    return ns ? (ns / 1000000 ? ns / 1000000 : 1) : 0;
}
static bool arm_timer(OpenDroboSCSIController *self) {
    uint64_t now = uptime_ns();
    return now && now <= UINT64_MAX - 50000000 && self->ivars->timer &&
        self->ivars->timer->WakeAtTime(kIOTimerClockUptimeRaw, now + 50000000, 1000000) == kIOReturnSuccess;
}
static void release_buffers(ServiceTask &task) {
    if (task.request_buffer) task.request_buffer->release();
    if (task.data_buffer) task.data_buffer->release();
    task = ServiceTask{};
}
static void retain_action(void *, void *action) { static_cast<OSAction *>(action)->retain(); }
static void release_action(void *, void *action) { static_cast<OSAction *>(action)->release(); }
static void complete_action(void *context, void *action, const ODTBTControllerCompletion *result) {
    auto self = static_cast<OpenDroboSCSIController *>(context);
    ServicePin pin(self);
    SCSIUserParallelResponse response = {};
    response.version = kScsiUserParallelTaskResponseCurrentVersion1;
    response.fControllerTaskIdentifier = result->task_id;
    response.fTargetID = 0;
    response.fBytesTransferred = result->realized_bytes;
    response.fServiceResponse = result->succeeded ? kSCSIServiceResponse_TASK_COMPLETE
        : kSCSIServiceResponse_SERVICE_DELIVERY_OR_TARGET_FAILURE;
    response.fCompletionStatus = result->succeeded ? kSCSITaskStatus_GOOD : kSCSITaskStatus_DeliveryFailure;
    for (auto &slot : self->ivars->slots) if (slot.used && slot.id == result->task_id) {
        if (result->srb_response == kLocalIllegalCommand) {
            response.fServiceResponse = kSCSIServiceResponse_TASK_COMPLETE;
            response.fCompletionStatus = kSCSITaskStatus_CHECK_CONDITION;
            response.fSenseLength = 18;
            response.fSenseBuffer[0] = 0x70;
            response.fSenseBuffer[2] = slot.input.direction == 1 ? 7 : 5;
            response.fSenseBuffer[7] = 10;
            response.fSenseBuffer[12] = slot.input.direction == 1 ? 0x27 : 0x20;
        } else if (!result->cancelled && (result->srb_response == 1 || result->srb_response == 0x12)) {
            response.fServiceResponse = kSCSIServiceResponse_TASK_COMPLETE;
            response.fCompletionStatus = (SCSITaskStatus)result->scsi_status;
        }
        /* The shared table checked both DMA leases before invoking this path.
         * Drop descriptor ownership before a framework callback can reenter. */
        release_buffers(slot);
        break;
    }
    self->ParallelTaskCompletion(static_cast<OSAction *>(action), response);
    maybe_finish_stop(self);
}
static void defer_result(OpenDroboSCSIController *self, ODTBTControllerCompletion result) {
    self->retain();
    self->ivars->queue->DispatchAsync(^{
        (void)od_scsi_tasks_finish(&self->ivars->tasks, &result);
        self->release();
    });
}
static void finish_engine(OpenDroboSCSIController *self, ODTBTController &engine) {
    ODTBTControllerCompletion results[OD_TBT_CONTROLLER_SLOTS]; size_t count = 0;
    /* There is deliberately no fabricated bus-master drain attestation here. */
    auto status = od_tbt_controller_finish_stop(&engine, 1, 0, results, OD_TBT_CONTROLLER_SLOTS, &count);
    if (status == OD_TBT_CTL_OK)
        for (size_t i = 0; i < count; ++i) (void)od_scsi_tasks_finish(&self->ivars->tasks, results + i);
}
static void session_closed(OpenDroboSCSIController *self) {
    ServicePin pin(self);
    self->ivars->sources_closed = true;
    if (self->ivars->interrupt_action) {
        self->ivars->interrupt_action->release(); self->ivars->interrupt_action = nullptr;
    }
    if (self->ivars->timer_action) {
        self->ivars->timer_action->release(); self->ivars->timer_action = nullptr;
    }
    if (self->ivars->engines_initialized) {
        finish_engine(self, self->ivars->storage);
        finish_engine(self, self->ivars->management);
    }
    maybe_finish_stop(self);
}
static void close_session(OpenDroboSCSIController *self) {
    auto state = self->ivars->session.GetState();
    if (state == ODTBTDriverKitSession::State::Empty || state == ODTBTDriverKitSession::State::Closed) {
        session_closed(self); return;
    }
    (void)self->ivars->session.BeginClose(^(kern_return_t result) {
        if (result == kIOReturnSuccess) session_closed(self);
        // Failed cancellation retains the source, owner, queue and stop pin.
    });
}
static void begin_shutdown(OpenDroboSCSIController *self) {
    auto v = self->ivars;
    if (v->stopping) { maybe_finish_stop(self); return; }
    v->stopping = true; v->running = false;
    self->retain(); v->stop_pin = true;
    if (v->engines_initialized) {
        if (v->storage.state == OD_TBT_CTL_RUNNING)
            (void)od_tbt_controller_summary_disable(&v->storage);
        (void)od_tbt_controller_begin_stop(&v->storage, 0);
        (void)od_tbt_controller_begin_stop(&v->management, 0);
    }
    if (!v->timer) { close_session(self); return; }
    (void)v->timer->SetEnableWithCompletion(false, ^{
        (void)v->timer->Cancel(^{
            v->timer->release(); v->timer = nullptr;
            close_session(self);
        });
    });
}
static void maybe_finish_stop(OpenDroboSCSIController *self) {
    auto v = self->ivars;
    if (!v || !v->stopping || !v->sources_closed || v->auxiliary_work ||
        od_scsi_tasks_pending(&v->tasks)) return;
    if (v->target_present && !v->target_destroying) {
        v->target_destroying = true; ++v->auxiliary_work;
        v->auxiliary->DispatchAsync(^{
            auto result = self->UserDestroyTargetForID(0);
            v->queue->DispatchAsync(^{
                --v->auxiliary_work;
                if (result == kIOReturnSuccess) { v->target_present = false; maybe_finish_stop(self); }
            });
        });
        return;
    }
    if (v->target_present) return;
    if (v->auxiliary && !v->auxiliary_closed) {
        if (!v->auxiliary_closing) {
            v->auxiliary_closing = true;
            auto result = v->auxiliary->Cancel(^{
                v->queue->DispatchAsync(^{
                    v->auxiliary_closed = true;
                    maybe_finish_stop(self);
                });
            });
            if (result != kIOReturnSuccess) v->auxiliary_closing = false;
        }
        return;
    }
    if (v->power_pending) {
        v->power_pending = false;
        (void)self->SetPowerState(v->pending_power, kSuperDispatch);
    }
    if (!v->stop_provider) {
        if (v->startup_failed && v->stop_pin) {
            v->stop_pin = false; self->release();
        }
        return; // A fault/power transition leaves a closed, nonrestartable service.
    }
    IOService *provider = v->stop_provider; v->stop_provider = nullptr;
    (void)self->Stop(provider, kSuperDispatch);
    provider->release();
    if (v->stop_pin) { v->stop_pin = false; self->release(); }
}
static void poll_queues(OpenDroboSCSIController *self) {
    auto v = self->ivars;
    if (!v->running || v->stopping) return;
    /* Acknowledge the shared cause BEFORE inspecting both queues. Individual
     * engines own only their channel response acknowledgments in this mode. */
    if (od_tbt_controller_summary_clear(&v->storage) != OD_TBT_CTL_OK) { begin_shutdown(self); return; }
    for (auto engine : {&v->storage, &v->management}) {
        ODTBTControllerCompletion results[OD_TBT_CONTROLLER_SLOTS]; size_t count = 0;
        if (od_tbt_controller_poll(engine, uptime_ms(), results, OD_TBT_CONTROLLER_SLOTS, &count) != OD_TBT_CTL_OK) {
            begin_shutdown(self); return;
        }
        for (size_t i = 0; i < count; ++i)
            if (od_scsi_tasks_finish(&v->tasks, results + i) != OD_SCSI_TASK_OK) { begin_shutdown(self); return; }
    }
}

bool OpenDroboSCSIController::init() {
    if (!super::init()) return false;
    ivars = new (std::nothrow) OpenDroboSCSIController_IVars;
    if (!ivars) return false;
    ODScsiTaskCallbacks callbacks = {this, retain_action, release_action, complete_action};
    return od_scsi_tasks_init(&ivars->tasks, &callbacks) == OD_SCSI_TASK_OK;
}
void OpenDroboSCSIController::free() {
    /* A session/stop retain pins the service while callbacks or DMA remain.
     * Never free caller-owned lease storage on an unexpected unsafe teardown. */
    if (ivars && (od_scsi_tasks_pending(&ivars->tasks) || ivars->stop_pin || ivars->auxiliary_work ||
                  ivars->timer || (ivars->session.GetState() != ODTBTDriverKitSession::State::Empty &&
                                  ivars->session.GetState() != ODTBTDriverKitSession::State::Closed))) return;
    if (ivars) {
        if (ivars->interrupt_action) ivars->interrupt_action->release();
        if (ivars->timer_action) ivars->timer_action->release();
        if (ivars->queue) ivars->queue->release();
        if (ivars->auxiliary) ivars->auxiliary->release();
        delete ivars; ivars = nullptr;
    }
    super::free();
}
kern_return_t IMPL(OpenDroboSCSIController, Start) {
    if (!od_scsi_service_hardware_enabled()) return kIOReturnUnsupported;
    ServicePin pin(this);
    ivars->provider = OSDynamicCast(IOPCIDevice, provider);
    if (!ivars->provider) return kIOReturnBadArgument;
    auto result = Start(provider, SUPERDISPATCH);
    if (result != kIOReturnSuccess) { ivars->startup_failed = true; begin_shutdown(this); }
    return result;
}
kern_return_t IMPL(OpenDroboSCSIController, UserInitializeController) {
    if (!od_scsi_service_hardware_enabled()) return kIOReturnUnsupported;
    ServicePin pin(this);
    if (ivars->initialized || !ivars->provider) return kIOReturnNotReady;
    auto fail = [this](kern_return_t error) {
        ivars->startup_failed = true;
        begin_shutdown(this);
        return error;
    };
    auto result = CopyDispatchQueue(kIOServiceDefaultQueueName, &ivars->queue);
    if (result != kIOReturnSuccess) return fail(result);
    result = IODispatchQueue::Create("OpenDrobo auxiliary", 0, 0, &ivars->auxiliary);
    if (result != kIOReturnSuccess) return fail(result);
    result = SetDispatchQueue("AuxiliaryQueue", ivars->auxiliary);
    if (result != kIOReturnSuccess) return fail(result);
    if ((result = CreateActionHandleInterrupt(0, &ivars->interrupt_action)) != kIOReturnSuccess) return fail(result);
    if ((result = ivars->session.Open(ivars->provider, this, ivars->queue, ivars->interrupt_action, 0)) != kIOReturnSuccess)
        return fail(result);
    auto io = od_tbt_driverkit_controller_io(&ivars->session);
    if (od_tbt_controller_init_channel(&ivars->storage, &io, (uint32_t)ivars->session.BarSize(0),
            (uint32_t)ivars->session.BarSize(1), OD_TBT_CHANNEL_STORAGE, 1) != OD_TBT_CTL_OK ||
        od_tbt_controller_init_channel(&ivars->management, &io, (uint32_t)ivars->session.BarSize(0),
            (uint32_t)ivars->session.BarSize(1), OD_TBT_CHANNEL_MANAGEMENT, 1) != OD_TBT_CTL_OK) return fail(kIOReturnUnsupported);
    ivars->engines_initialized = true;
    if ((result = CreateActionHandleTimer(0, &ivars->timer_action)) != kIOReturnSuccess ||
        (result = IOTimerDispatchSource::Create(ivars->queue, &ivars->timer)) != kIOReturnSuccess ||
        (result = ivars->timer->SetHandler(ivars->timer_action)) != kIOReturnSuccess ||
        (result = ivars->timer->SetEnableWithCompletion(true, nullptr)) != kIOReturnSuccess) return fail(result);
    memcpy(ivars->identity.vendor, "OpenDrobo", 8);
    memcpy(ivars->identity.product, "Read-only bridge", 16);
    memcpy(ivars->identity.revision, "0001", 4);
    uint64_t registry_id = 0;
    if ((result = ivars->provider->GetRegistryEntryID(&registry_id)) != kIOReturnSuccess) return fail(result);
    char serial[17]; snprintf(serial, sizeof(serial), "%016llx", (unsigned long long)registry_id);
    memcpy(ivars->identity.serial, serial, 16); ivars->identity.serial_length = 16;
    ivars->initialized = true;
    return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserStartController) {
    if (!od_scsi_service_hardware_enabled() || !ivars->initialized || ivars->stopping) return kIOReturnUnsupported;
    ServicePin pin(this);
    uint32_t summary = 0;
    if (od_tbt_controller_start(&ivars->storage) != OD_TBT_CTL_OK ||
        od_tbt_controller_start(&ivars->management) != OD_TBT_CTL_OK ||
        od_tbt_controller_summary_state(&ivars->storage, &summary) != OD_TBT_CTL_OK || summary ||
        ivars->session.EnableInterrupts() != kIOReturnSuccess ||
        od_tbt_controller_summary_enable(&ivars->storage) != OD_TBT_CTL_OK ||
        ivars->session.EnableBusMaster(&ivars->storage) != kIOReturnSuccess) {
        begin_shutdown(this); return kIOReturnNotReady;
    }
    ivars->running = true;
    if (!arm_timer(this)) { begin_shutdown(this); return kIOReturnNotReady; }
    ++ivars->auxiliary_work;
    ivars->auxiliary->DispatchAsync(^{
        auto properties = OSDictionary::withCapacity(0);
        auto result = properties ? UserCreateTargetForID(0, properties) : kIOReturnNoMemory;
        if (properties) properties->release();
        ivars->queue->DispatchAsync(^{
            --ivars->auxiliary_work;
            ivars->target_present = result == kIOReturnSuccess;
            if (result != kIOReturnSuccess) begin_shutdown(this);
            else maybe_finish_stop(this);
        });
    });
    return kIOReturnSuccess;
}
void IMPL(OpenDroboSCSIController, HandleInterrupt) {
    ServicePin pin(this);
    (void)action; (void)count; (void)time; poll_queues(this);
}
void IMPL(OpenDroboSCSIController, HandleTimer) {
    ServicePin pin(this);
    (void)action; (void)time; poll_queues(this);
    if (ivars->running && !arm_timer(this)) begin_shutdown(this);
}
kern_return_t IMPL(OpenDroboSCSIController, Stop) {
    ServicePin pin(this);
    if (!provider) return kIOReturnBadArgument;
    if (!ivars->stop_provider) { ivars->stop_provider = provider; provider->retain(); }
    begin_shutdown(this);
    return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, SetPowerState) {
    if (!od_scsi_service_hardware_enabled()) return kIOReturnUnsupported;
    ServicePin pin(this);
    if (powerFlags == kIOServicePowerCapabilityOn)
        return ivars->stopping ? kIOReturnNotReady : SetPowerState(powerFlags, SUPERDISPATCH);
    ivars->power_pending = true; ivars->pending_power = powerFlags;
    begin_shutdown(this);
    return kIOReturnSuccess;
}

static bool abort_unpublished(ServiceTask &slot) {
    bool ok = true;
    for (auto lease : {&slot.request, &slot.payload})
        if (lease->state == OD_DMA_LEASE_PREPARED && od_dma_lease_abort_unpublished(lease) != OD_DMA_STATUS_OK) ok = false;
    return ok;
}
kern_return_t IMPL(OpenDroboSCSIController, UserProcessParallelTask) {
    ServicePin pin(this);
    if (!response || !completion) return kIOReturnBadArgument;
    *response = kSCSIServiceResponse_SERVICE_DELIVERY_OR_TARGET_FAILURE;
    if (!ivars->running || ivars->stopping || parallelRequest.fSCSIParallelFeatureRequestCount ||
        parallelRequest.fTaskAttribute != kSCSITask_SIMPLE || parallelRequest.version != 1 ||
        parallelRequest.fTargetID != 0) return kIOReturnSuccess;
    for (auto byte : parallelRequest.fLogicalUnitBytes) if (byte) return kIOReturnSuccess;
    uint32_t index;
    for (index = 0; index < OD_TBT_CONTROLLER_SLOTS && ivars->slots[index].used; ++index) { }
    if (index == OD_TBT_CONTROLLER_SLOTS) return kIOReturnSuccess;
    auto &slot = ivars->slots[index];
    slot.id = parallelRequest.fControllerTaskIdentifier; slot.used = true;
    slot.input.version = parallelRequest.version; slot.input.target_id = parallelRequest.fTargetID;
    slot.input.controller_task_id = slot.id; slot.input.requested_bytes = parallelRequest.fRequestedTransferCount;
    slot.input.buffer_iovm_address = 1; // Syntax planning only; actual DMA is prepared below.
    slot.input.direction = parallelRequest.fTransferDirection; slot.input.cdb_length = parallelRequest.fCommandSize;
    memcpy(slot.input.lun, parallelRequest.fLogicalUnitBytes, 8);
    memcpy(slot.input.cdb, parallelRequest.fCommandDescriptorBlock, 16);
    ODScsiGeometry geometry = {512, ivars->storage.logical_blocks};
    uint8_t inline_bytes[256]; uint32_t realized = 0, table_slot = 0;
    auto inline_status = od_scsi_write_inline_response(&slot.input, &geometry, &ivars->identity,
                                                       inline_bytes, sizeof(inline_bytes), &realized);
    ODTBTReadPlan plan = {};
    bool queued = inline_status == OD_SCSI_RESPONSE_UNSUPPORTED &&
        od_tbt_plan_read_only(&slot.input, 512, 512 * 1024, &plan) == OD_TBT_PLAN_OK &&
        (plan.channel == OD_TBT_CHANNEL_STORAGE || plan.channel == OD_TBT_CHANNEL_MANAGEMENT);
    if (od_scsi_tasks_reserve(&ivars->tasks, slot.id, completion, queued ? &slot.request : nullptr,
                              queued ? &slot.payload : nullptr, &table_slot) != OD_SCSI_TASK_OK) {
        release_buffers(slot); return kIOReturnSuccess;
    }
    *response = kSCSIServiceResponse_Request_In_Process;
    if (!queued) {
        bool ok = inline_status == OD_SCSI_RESPONSE_HANDLED;
        if (ok && realized) {
            IOAddressSegment range = {};
            ok = UserGetDataBuffer(0, slot.id, &slot.data_buffer) == kIOReturnSuccess && slot.data_buffer &&
                 slot.data_buffer->GetAddressRange(&range) == kIOReturnSuccess && range.address &&
                 range.length >= realized && range.address <= UINTPTR_MAX - realized;
            if (ok) memcpy((void *)(uintptr_t)range.address, inline_bytes, realized);
        }
        const bool illegal = inline_status != OD_SCSI_RESPONSE_HANDLED;
        ODTBTControllerCompletion result = {slot.id, 0, ok ? realized : 0,
            ok ? 1U : illegal ? kLocalIllegalCommand : 0U, illegal ? 2U : 0U,
            (uint8_t)ok, (uint8_t)(!ok && !illegal)};
        defer_result(this, result); return kIOReturnSuccess;
    }
    auto engine = plan.channel == OD_TBT_CHANNEL_STORAGE ? &ivars->storage : &ivars->management;
    uint32_t request_bytes = plan.channel == OD_TBT_CHANNEL_STORAGE ? OD_TBT_STORAGE_REQUEST_BYTES : OD_TBT_TM_REQUEST_BYTES;
    auto backend = od_driverkit_dma_backend(ivars->session.Provider());
    bool mapped = IOBufferMemoryDescriptor::Create(kIOMemoryDirectionInOut, request_bytes, 0,
                       &slot.request_buffer) == kIOReturnSuccess && slot.request_buffer &&
                  slot.request_buffer->SetLength(request_bytes) == kIOReturnSuccess &&
                  UserGetDataBuffer(0, slot.id, &slot.data_buffer) == kIOReturnSuccess && slot.data_buffer;
    if (mapped) mapped = od_dma_lease_prepare(&slot.request, &backend, slot.request_buffer, 0, request_bytes) == OD_DMA_STATUS_OK;
    if (mapped) mapped = od_dma_lease_prepare(&slot.payload, &backend, slot.data_buffer, 0, slot.input.requested_bytes) == OD_DMA_STATUS_OK;
    uint64_t now = uptime_ms(); uint64_t timeout = parallelRequest.fTimeoutInMilliSec ? parallelRequest.fTimeoutInMilliSec : 30000;
    uint32_t transaction = 0;
    auto status = mapped && now && timeout <= UINT64_MAX - now
        ? od_tbt_controller_submit(engine, &slot.input, &slot.request, &slot.payload, now, now + timeout, &transaction)
        : OD_TBT_CTL_DMA;
    if (status == OD_TBT_CTL_OK) {
        if (od_scsi_tasks_bind(&ivars->tasks, table_slot, transaction) != OD_SCSI_TASK_OK) begin_shutdown(this);
        return kIOReturnSuccess;
    }
    /* A publication error may have reached hardware. Preserve its actual
     * transaction association and never complete that framework action early. */
    bool published = false;
    for (auto &entry : engine->slots) if (entry.request == &slot.request) {
        published = true;
        (void)od_scsi_tasks_bind(&ivars->tasks, table_slot, entry.transaction);
    }
    if (!published && abort_unpublished(slot)) {
        ODTBTControllerCompletion result = {slot.id, 0, 0, 0, 0, 0, 1};
        defer_result(this, result);
    }
    begin_shutdown(this);
    return kIOReturnSuccess;
}

kern_return_t IMPL(OpenDroboSCSIController, UserInitializeTargetForID) {
    return targetID == 0 && ivars->running && !ivars->stopping ? kIOReturnSuccess : kIOReturnNotReady;
}
kern_return_t IMPL(OpenDroboSCSIController, UserMapHBAData) {
    if (!uniqueTaskID || ivars->next_hba_id == UINT32_MAX) return kIOReturnNoResources;
    *uniqueTaskID = ivars->next_hba_id++; return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserGetDMASpecification) {
    if (!maxTransferSize || !alignment || !numAddressBits || !segmentType) return kIOReturnBadArgument;
    *maxTransferSize = 512 * 1024; *alignment = 1; *numAddressBits = 32; *segmentType = kDMAOutputSegmentLittle32;
    return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserReportHBAHighestLogicalUnitNumber) {
    if (!value) return kIOReturnBadArgument; *value = 0; return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserDoesHBASupportSCSIParallelFeature) {
    (void)theValue; if (!result) return kIOReturnBadArgument; *result = false; return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserDoesHBAPerformAutoSense) {
    if (!result) return kIOReturnBadArgument; *result = false; return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserDoesHBASupportMultiPathing) {
    if (!result) return kIOReturnBadArgument; *result = false; return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserReportInitiatorIdentifier) {
    if (!id) return kIOReturnBadArgument; *id = UINT64_MAX; return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserReportHighestSupportedDeviceID) {
    if (!id) return kIOReturnBadArgument; *id = 0; return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserReportMaximumTaskCount) {
    if (!count) return kIOReturnBadArgument; *count = OD_TBT_CONTROLLER_SLOTS; return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserDoesHBAPerformDeviceManagement) {
    if (!result) return kIOReturnBadArgument; *result = true; return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserMapBundledParallelTaskCommandAndResponseBuffers) {
    (void)parallelCommandIOMemoryDescriptor; (void)parallelResponseIOMemoryDescriptor;
    return kIOReturnUnsupported; // Select the normal, individually owned task API.
}
void IMPL(OpenDroboSCSIController, UserProcessBundledParallelTasks) {
    (void)parallelRequestSlotIndices; (void)parallelRequestSlotIndicesCount; (void)completion;
    // No shared slots were accepted by UserMapBundledParallelTaskCommandAndResponseBuffers.
    begin_shutdown(this);
}
static kern_return_t reject_management(uint32_t *response) {
    if (!response) return kIOReturnBadArgument;
    *response = kSCSIServiceResponse_FUNCTION_REJECTED; return kIOReturnSuccess;
}
kern_return_t IMPL(OpenDroboSCSIController, UserAbortTaskRequest) {
    (void)theT; (void)theL; (void)theQ; begin_shutdown(this); return reject_management(response);
}
kern_return_t IMPL(OpenDroboSCSIController, UserAbortTaskSetRequest) {
    (void)theT; (void)theL; begin_shutdown(this); return reject_management(response);
}
kern_return_t IMPL(OpenDroboSCSIController, UserClearACARequest) {
    (void)theT; (void)theL; return reject_management(response);
}
kern_return_t IMPL(OpenDroboSCSIController, UserClearTaskSetRequest) {
    (void)theT; (void)theL; return reject_management(response);
}
kern_return_t IMPL(OpenDroboSCSIController, UserLogicalUnitResetRequest) {
    (void)theT; (void)theL; return reject_management(response);
}
kern_return_t IMPL(OpenDroboSCSIController, UserTargetResetRequest) {
    (void)theT; return reject_management(response);
}
