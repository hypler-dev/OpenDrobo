#ifndef OPENDROBO_SCSI_TASK_TABLE_H
#define OPENDROBO_SCSI_TASK_TABLE_H
#include "ControllerTransport.h"
#ifdef __cplusplus
extern "C" {
#endif
typedef enum { OD_SCSI_TASK_OK = 0, OD_SCSI_TASK_ARGUMENT, OD_SCSI_TASK_BUSY,
               OD_SCSI_TASK_MISMATCH, OD_SCSI_TASK_DMA_LIVE } ODScsiTaskStatus;
typedef struct {
    void *context;
    void (*retain)(void *, void *action);
    void (*release)(void *, void *action);
    void (*complete)(void *, void *action, const ODTBTControllerCompletion *);
} ODScsiTaskCallbacks;
typedef struct {
    void *action;
    uint64_t task_id;
    uint32_t transaction;
    ODDmaLease *request, *payload;
} ODScsiTaskEntry;
typedef struct {
    ODScsiTaskCallbacks callbacks;
    ODScsiTaskEntry entries[OD_TBT_CONTROLLER_SLOTS];
} ODScsiTaskTable;

/* Hardware execution stays closed until a separate reviewed lab change proves
 * storage fields, DMA drain, reset and power lifecycles. No runtime preference
 * or Info.plist value can enable this source checkpoint. */
int od_scsi_service_hardware_enabled(void);

/* New zero-initialized storage only; calls and callbacks must be serialized.
 * reserve retains the action. finish removes the entry before its callback,
 * then drops the action retain using snapshotted callbacks, even if completion
 * replaces or destroys the table. Callback context must remain alive through
 * the release callback. Published/uncertain DMA cannot finish an OS
 * request until both leases are COMPLETED or EMPTY; a numeric completion or
 * timeout is not sufficient evidence. No callback pointer is retained beyond
 * the lifetime of the caller-owned table. */
ODScsiTaskStatus od_scsi_tasks_init(ODScsiTaskTable *, const ODScsiTaskCallbacks *);
ODScsiTaskStatus od_scsi_tasks_reserve(ODScsiTaskTable *, uint64_t task_id, void *action,
                                     ODDmaLease *request, ODDmaLease *payload, uint32_t *slot);
ODScsiTaskStatus od_scsi_tasks_bind(ODScsiTaskTable *, uint32_t slot, uint32_t transaction);
ODScsiTaskStatus od_scsi_tasks_finish(ODScsiTaskTable *, const ODTBTControllerCompletion *);
uint32_t od_scsi_tasks_pending(const ODScsiTaskTable *);
#ifdef __cplusplus
}
#endif
#endif
