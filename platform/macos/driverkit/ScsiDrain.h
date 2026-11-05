#ifndef OPENDROBO_SCSI_DRAIN_H
#define OPENDROBO_SCSI_DRAIN_H

#include <stdint.h>
#include "ControllerTransport.h"

#ifdef __cplusplus
extern "C" {
#endif

enum {
    OD_SCSI_DRAIN_MAX_PENDING_ACTIONS = 32U,
    OD_SCSI_DRAIN_MAX_LIVE_OWNED_LEASES = 64U,
    OD_SCSI_DRAIN_MAGIC = UINT32_C(0x4f445244)
};

typedef enum {
    OD_SCSI_DRAIN_RUNNING = 0,
    OD_SCSI_DRAIN_DRAINING = 1,
    OD_SCSI_DRAIN_CLOSE_SOURCES = 2,
    OD_SCSI_DRAIN_QUARANTINED = 3
} ODScsiDrainState;

typedef enum {
    OD_SCSI_DRAIN_WAIT = 0,
    OD_SCSI_DRAIN_CLOSE = 1,
    OD_SCSI_DRAIN_ERROR = 2
} ODScsiDrainStatus;

typedef enum {
    OD_SCSI_DRAIN_FAILURE_TIMEOUT = 1,
    OD_SCSI_DRAIN_FAILURE_PROTOCOL = 2,
    OD_SCSI_DRAIN_FAILURE_REMOVAL = 3
} ODScsiDrainFailure;

/*
 * Caller-owned, serialized drain state. It retains no action, DMA mapping,
 * buffer, or callback pointer and never releases any of them. A quarantined
 * instance is terminal; reinitialize separate new storage only after a caller
 * has established an independently safe lifecycle.
 */
typedef struct {
    uint32_t magic;
    ODScsiDrainState state;
    uint64_t deadline_ms;
    uint64_t last_observed_ms;
    uint32_t clock_seen;
    ODScsiDrainFailure failure;
} ODScsiDrain;

#define OD_SCSI_DRAIN_INITIALIZER { 0U, OD_SCSI_DRAIN_RUNNING, 0U, 0U, 0U, 0 }

/*
 * Initialize only new zeroed storage or OD_SCSI_DRAIN_INITIALIZER storage.
 * Calls must be serialized by caller. A later init refuses initialized storage,
 * including QUARANTINED storage, so this instance cannot restart.
 */
ODScsiDrainStatus od_scsi_drain_init(ODScsiDrain *drain);

/*
 * Stop new work and continue completion polling. deadline_ms is an absolute,
 * checked monotonic-clock deadline. Repeating begin cannot extend it. A
 * deadline before now, or a clock rollback while draining, quarantines.
 */
ODScsiDrainStatus od_scsi_drain_begin(ODScsiDrain *drain, uint64_t now_ms,
                                      uint64_t deadline_ms);

/*
 * Feed one serialized snapshot of outstanding caller-owned resources. WAIT
 * means caller keeps polling completions and retains all resources. CLOSE is
 * returned only when both counts are zero. Counts above their fixed bounds,
 * clock rollback, or a deadline with resources still live quarantine instead.
 * After CLOSE, another observation must keep both counts zero and the clock
 * monotonic; a contradictory snapshot quarantines rather than reusing old zero
 * counts as proof. No close result attests physical PCI/DMA quiescence.
 * At/after deadline with both counts zero may close because no caller-owned
 * request or mapping remains.
 */
ODScsiDrainStatus od_scsi_drain_observe(ODScsiDrain *drain, uint64_t now_ms,
                                        uint32_t pending_actions,
                                        uint32_t live_owned_leases);

/* Terminal caller-reported failure. No drain API attests hardware quiescence. */
ODScsiDrainStatus od_scsi_drain_fail(ODScsiDrain *drain,
                                     ODScsiDrainFailure failure);

/* Shared service fault classification. Removal takes precedence over timeout;
 * every other failed controller result is a protocol failure. An OK result
 * without removal is refused without changing state. No resource is released. */
ODScsiDrainStatus od_scsi_drain_fail_controller(ODScsiDrain *drain,
                                               ODTBTControllerStatus status,
                                               int removed);

/* Admission closes at begin and stays closed through close/quarantine. */
int od_scsi_drain_accepts_work(const ODScsiDrain *drain);

/* Completion polling continues only while RUNNING or DRAINING. */
int od_scsi_drain_polls_completions(const ODScsiDrain *drain);

#ifdef __cplusplus
}
#endif

#endif
