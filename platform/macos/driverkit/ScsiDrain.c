#include "ScsiDrain.h"

static int valid_failure(ODScsiDrainFailure failure) {
    return failure == OD_SCSI_DRAIN_FAILURE_TIMEOUT ||
           failure == OD_SCSI_DRAIN_FAILURE_PROTOCOL ||
           failure == OD_SCSI_DRAIN_FAILURE_REMOVAL;
}

static ODScsiDrainStatus quarantine(ODScsiDrain *drain,
                                    ODScsiDrainFailure failure) {
    drain->state = OD_SCSI_DRAIN_QUARANTINED;
    drain->failure = failure;
    return OD_SCSI_DRAIN_ERROR;
}

static int valid(const ODScsiDrain *drain) {
    return drain && drain->magic == OD_SCSI_DRAIN_MAGIC;
}

ODScsiDrainStatus od_scsi_drain_init(ODScsiDrain *drain) {
    if (!drain || drain->magic != 0) return OD_SCSI_DRAIN_ERROR;
    drain->magic = OD_SCSI_DRAIN_MAGIC;
    drain->state = OD_SCSI_DRAIN_RUNNING;
    drain->deadline_ms = 0;
    drain->last_observed_ms = 0;
    drain->clock_seen = 0;
    drain->failure = 0;
    return OD_SCSI_DRAIN_WAIT;
}

ODScsiDrainStatus od_scsi_drain_begin(ODScsiDrain *drain, uint64_t now_ms,
                                      uint64_t deadline_ms) {
    if (!valid(drain)) return OD_SCSI_DRAIN_ERROR;
    if (drain->state == OD_SCSI_DRAIN_QUARANTINED) return OD_SCSI_DRAIN_ERROR;
    if (drain->state == OD_SCSI_DRAIN_CLOSE_SOURCES) return OD_SCSI_DRAIN_CLOSE;
    if (drain->state == OD_SCSI_DRAIN_DRAINING) {
        if (drain->clock_seen && now_ms < drain->last_observed_ms)
            return quarantine(drain, OD_SCSI_DRAIN_FAILURE_PROTOCOL);
        drain->last_observed_ms = now_ms;
        drain->clock_seen = 1;
        return OD_SCSI_DRAIN_WAIT;
    }
    if (drain->state != OD_SCSI_DRAIN_RUNNING || deadline_ms < now_ms)
        return quarantine(drain, OD_SCSI_DRAIN_FAILURE_PROTOCOL);

    drain->deadline_ms = deadline_ms;
    drain->last_observed_ms = now_ms;
    drain->clock_seen = 1;
    drain->state = OD_SCSI_DRAIN_DRAINING;
    return OD_SCSI_DRAIN_WAIT;
}

ODScsiDrainStatus od_scsi_drain_observe(ODScsiDrain *drain, uint64_t now_ms,
                                        uint32_t pending_actions,
                                        uint32_t live_owned_leases) {
    if (!valid(drain) || drain->state == OD_SCSI_DRAIN_QUARANTINED)
        return OD_SCSI_DRAIN_ERROR;
    if (drain->state == OD_SCSI_DRAIN_CLOSE_SOURCES) {
        if (pending_actions != 0 || live_owned_leases != 0 ||
            pending_actions > OD_SCSI_DRAIN_MAX_PENDING_ACTIONS ||
            live_owned_leases > OD_SCSI_DRAIN_MAX_LIVE_OWNED_LEASES ||
            (drain->clock_seen && now_ms < drain->last_observed_ms))
            return quarantine(drain, OD_SCSI_DRAIN_FAILURE_PROTOCOL);
        drain->last_observed_ms = now_ms;
        drain->clock_seen = 1;
        return OD_SCSI_DRAIN_CLOSE;
    }
    if (drain->state != OD_SCSI_DRAIN_DRAINING ||
        pending_actions > OD_SCSI_DRAIN_MAX_PENDING_ACTIONS ||
        live_owned_leases > OD_SCSI_DRAIN_MAX_LIVE_OWNED_LEASES ||
        (drain->clock_seen && now_ms < drain->last_observed_ms))
        return quarantine(drain, OD_SCSI_DRAIN_FAILURE_PROTOCOL);

    drain->last_observed_ms = now_ms;
    drain->clock_seen = 1;
    if (pending_actions == 0 && live_owned_leases == 0) {
        drain->state = OD_SCSI_DRAIN_CLOSE_SOURCES;
        return OD_SCSI_DRAIN_CLOSE;
    }
    if (now_ms >= drain->deadline_ms)
        return quarantine(drain, OD_SCSI_DRAIN_FAILURE_TIMEOUT);
    return OD_SCSI_DRAIN_WAIT;
}

ODScsiDrainStatus od_scsi_drain_fail(ODScsiDrain *drain,
                                     ODScsiDrainFailure failure) {
    if (!valid(drain) || !valid_failure(failure)) return OD_SCSI_DRAIN_ERROR;
    if (drain->state == OD_SCSI_DRAIN_QUARANTINED) return OD_SCSI_DRAIN_ERROR;
    return quarantine(drain, failure);
}

ODScsiDrainStatus od_scsi_drain_fail_controller(ODScsiDrain *drain,
                                               ODTBTControllerStatus status,
                                               int removed) {
    if (status == OD_TBT_CTL_OK && !removed) return OD_SCSI_DRAIN_ERROR;
    return od_scsi_drain_fail(drain, removed ? OD_SCSI_DRAIN_FAILURE_REMOVAL :
        status == OD_TBT_CTL_TIMEOUT ? OD_SCSI_DRAIN_FAILURE_TIMEOUT :
        OD_SCSI_DRAIN_FAILURE_PROTOCOL);
}

int od_scsi_drain_accepts_work(const ODScsiDrain *drain) {
    return valid(drain) && drain->state == OD_SCSI_DRAIN_RUNNING;
}

int od_scsi_drain_polls_completions(const ODScsiDrain *drain) {
    return valid(drain) && (drain->state == OD_SCSI_DRAIN_RUNNING ||
                            drain->state == OD_SCSI_DRAIN_DRAINING);
}
