#include "ScsiTaskTable.h"
#include <string.h>

int od_scsi_service_hardware_enabled(void) { return 0; }
static int valid(const ODScsiTaskTable *t) {
    return t && t->callbacks.retain && t->callbacks.release && t->callbacks.complete;
}
ODScsiTaskStatus od_scsi_tasks_init(ODScsiTaskTable *t, const ODScsiTaskCallbacks *callbacks) {
    if (!t || !callbacks || !callbacks->retain || !callbacks->release || !callbacks->complete)
        return OD_SCSI_TASK_ARGUMENT;
    for (unsigned i = 0; i < OD_TBT_CONTROLLER_SLOTS; ++i)
        if (t->entries[i].action) return OD_SCSI_TASK_BUSY;
    ODScsiTaskCallbacks copied = *callbacks;
    memset(t, 0, sizeof(*t)); t->callbacks = copied;
    return OD_SCSI_TASK_OK;
}
uint32_t od_scsi_tasks_pending(const ODScsiTaskTable *t) {
    if (!valid(t)) return 0;
    uint32_t count = 0;
    for (unsigned i = 0; i < OD_TBT_CONTROLLER_SLOTS; ++i) if (t->entries[i].action) ++count;
    return count;
}
ODScsiTaskStatus od_scsi_tasks_reserve(ODScsiTaskTable *t, uint64_t id, void *action,
                                     ODDmaLease *request, ODDmaLease *payload, uint32_t *slot) {
    if (!valid(t) || !action || !slot || (!!request != !!payload) || (request && request == payload))
        return OD_SCSI_TASK_ARGUMENT;
    unsigned free_slot = OD_TBT_CONTROLLER_SLOTS;
    for (unsigned i = 0; i < OD_TBT_CONTROLLER_SLOTS; ++i) {
        ODScsiTaskEntry *e = t->entries + i;
        if (e->action && e->task_id == id) return OD_SCSI_TASK_BUSY;
        if (!e->action && free_slot == OD_TBT_CONTROLLER_SLOTS) free_slot = i;
    }
    if (free_slot == OD_TBT_CONTROLLER_SLOTS) return OD_SCSI_TASK_BUSY;
    t->callbacks.retain(t->callbacks.context, action);
    t->entries[free_slot] = (ODScsiTaskEntry){action, id, 0, request, payload};
    *slot = free_slot;
    return OD_SCSI_TASK_OK;
}
ODScsiTaskStatus od_scsi_tasks_bind(ODScsiTaskTable *t, uint32_t slot, uint32_t tx) {
    if (!valid(t) || slot >= OD_TBT_CONTROLLER_SLOTS || !tx) return OD_SCSI_TASK_ARGUMENT;
    if (!t->entries[slot].action || t->entries[slot].transaction) return OD_SCSI_TASK_MISMATCH;
    t->entries[slot].transaction = tx;
    return OD_SCSI_TASK_OK;
}
static int safe(const ODDmaLease *lease) {
    return !lease || (lease->magic == OD_DMA_LEASE_MAGIC &&
                     (lease->state == OD_DMA_LEASE_COMPLETED || lease->state == OD_DMA_LEASE_EMPTY));
}
ODScsiTaskStatus od_scsi_tasks_finish(ODScsiTaskTable *t, const ODTBTControllerCompletion *result) {
    if (!valid(t) || !result) return OD_SCSI_TASK_ARGUMENT;
    ODTBTControllerCompletion copied = *result;
    for (unsigned i = 0; i < OD_TBT_CONTROLLER_SLOTS; ++i) {
        ODScsiTaskEntry e = t->entries[i];
        if (!e.action || e.task_id != copied.task_id) continue;
        if (e.transaction != copied.transaction) return OD_SCSI_TASK_MISMATCH;
        if (!safe(e.request) || !safe(e.payload)) return OD_SCSI_TASK_DMA_LIVE;
        ODScsiTaskCallbacks callbacks = t->callbacks;
        memset(t->entries + i, 0, sizeof(*t->entries));
        callbacks.complete(callbacks.context, e.action, &copied);
        callbacks.release(callbacks.context, e.action);
        return OD_SCSI_TASK_OK;
    }
    return OD_SCSI_TASK_MISMATCH;
}
