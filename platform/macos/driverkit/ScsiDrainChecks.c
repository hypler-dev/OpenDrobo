#include "ScsiDrain.h"

#include <assert.h>
#include <stdio.h>

static void expect_running(ODScsiDrain *drain) {
    assert(drain->state == OD_SCSI_DRAIN_RUNNING);
    assert(od_scsi_drain_accepts_work(drain));
    assert(od_scsi_drain_polls_completions(drain));
}

static void expect_quarantined(ODScsiDrain *drain, ODScsiDrainFailure failure) {
    assert(drain->state == OD_SCSI_DRAIN_QUARANTINED);
    assert(drain->failure == failure);
    assert(!od_scsi_drain_accepts_work(drain));
    assert(!od_scsi_drain_polls_completions(drain));
}

int main(void) {
    ODScsiDrain drain = OD_SCSI_DRAIN_INITIALIZER;
    ODScsiDrain deadline_drain = OD_SCSI_DRAIN_INITIALIZER;
    ODScsiDrain empty_deadline_drain = OD_SCSI_DRAIN_INITIALIZER;
    ODScsiDrain invalid_deadline_drain = OD_SCSI_DRAIN_INITIALIZER;
    ODScsiDrain rollback_drain = OD_SCSI_DRAIN_INITIALIZER;
    ODScsiDrain action_limit_drain = OD_SCSI_DRAIN_INITIALIZER;
    ODScsiDrain lease_limit_drain = OD_SCSI_DRAIN_INITIALIZER;
    ODScsiDrain removal_drain = OD_SCSI_DRAIN_INITIALIZER;
    ODScsiDrain wrong_state_drain = OD_SCSI_DRAIN_INITIALIZER;
    ODScsiDrain close_rollback_drain = OD_SCSI_DRAIN_INITIALIZER;

    assert(od_scsi_drain_init(&drain) == OD_SCSI_DRAIN_WAIT);
    expect_running(&drain);
    assert(od_scsi_drain_begin(&drain, 100, 200) == OD_SCSI_DRAIN_WAIT);
    assert(drain.state == OD_SCSI_DRAIN_DRAINING);
    assert(!od_scsi_drain_accepts_work(&drain));
    assert(od_scsi_drain_polls_completions(&drain));
    assert(od_scsi_drain_observe(&drain, 101, 1, 0) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_observe(&drain, 102, 0, 1) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_observe(&drain, 103, 1, 1) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_observe(&drain, 104, 0, 0) == OD_SCSI_DRAIN_CLOSE);
    assert(drain.state == OD_SCSI_DRAIN_CLOSE_SOURCES);
    assert(!od_scsi_drain_polls_completions(&drain));
    assert(od_scsi_drain_observe(&drain, 105, 0, 0) == OD_SCSI_DRAIN_CLOSE);
    assert(od_scsi_drain_observe(&drain, 106, 1, 0) == OD_SCSI_DRAIN_ERROR);
    expect_quarantined(&drain, OD_SCSI_DRAIN_FAILURE_PROTOCOL);

    assert(od_scsi_drain_init(&deadline_drain) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_begin(&deadline_drain, 10, 50) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_begin(&deadline_drain, 20, 999) == OD_SCSI_DRAIN_WAIT);
    assert(deadline_drain.deadline_ms == 50);
    assert(od_scsi_drain_observe(&deadline_drain, 49, 0, 1) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_observe(&deadline_drain, 50, 0, 1) == OD_SCSI_DRAIN_ERROR);
    expect_quarantined(&deadline_drain, OD_SCSI_DRAIN_FAILURE_TIMEOUT);

    assert(od_scsi_drain_init(&empty_deadline_drain) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_begin(&empty_deadline_drain, 10, 10) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_observe(&empty_deadline_drain, 11, 0, 0) == OD_SCSI_DRAIN_CLOSE);
    assert(empty_deadline_drain.state == OD_SCSI_DRAIN_CLOSE_SOURCES);

    assert(od_scsi_drain_init(&invalid_deadline_drain) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_begin(&invalid_deadline_drain, 50, 49) == OD_SCSI_DRAIN_ERROR);
    expect_quarantined(&invalid_deadline_drain, OD_SCSI_DRAIN_FAILURE_PROTOCOL);

    assert(od_scsi_drain_init(&rollback_drain) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_begin(&rollback_drain, 20, 100) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_observe(&rollback_drain, 19, 0, 0) == OD_SCSI_DRAIN_ERROR);
    expect_quarantined(&rollback_drain, OD_SCSI_DRAIN_FAILURE_PROTOCOL);

    assert(od_scsi_drain_init(&action_limit_drain) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_begin(&action_limit_drain, 1, 10) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_observe(&action_limit_drain, 2, OD_SCSI_DRAIN_MAX_PENDING_ACTIONS + 1U, 0) ==
           OD_SCSI_DRAIN_ERROR);
    expect_quarantined(&action_limit_drain, OD_SCSI_DRAIN_FAILURE_PROTOCOL);

    assert(od_scsi_drain_init(&lease_limit_drain) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_begin(&lease_limit_drain, 1, 10) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_observe(&lease_limit_drain, 2, 0,
                                 OD_SCSI_DRAIN_MAX_LIVE_OWNED_LEASES + 1U) ==
           OD_SCSI_DRAIN_ERROR);
    expect_quarantined(&lease_limit_drain, OD_SCSI_DRAIN_FAILURE_PROTOCOL);

    assert(od_scsi_drain_init(&removal_drain) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_fail(&removal_drain, OD_SCSI_DRAIN_FAILURE_REMOVAL) == OD_SCSI_DRAIN_ERROR);
    expect_quarantined(&removal_drain, OD_SCSI_DRAIN_FAILURE_REMOVAL);
    assert(od_scsi_drain_begin(&removal_drain, 2, 20) == OD_SCSI_DRAIN_ERROR);
    assert(od_scsi_drain_observe(&removal_drain, 2, 0, 0) == OD_SCSI_DRAIN_ERROR);
    assert(od_scsi_drain_fail(&removal_drain, OD_SCSI_DRAIN_FAILURE_TIMEOUT) == OD_SCSI_DRAIN_ERROR);
    assert(od_scsi_drain_init(&removal_drain) == OD_SCSI_DRAIN_ERROR);
    expect_quarantined(&removal_drain, OD_SCSI_DRAIN_FAILURE_REMOVAL);

    assert(od_scsi_drain_init(&wrong_state_drain) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_observe(&wrong_state_drain, 1, 0, 0) == OD_SCSI_DRAIN_ERROR);
    expect_quarantined(&wrong_state_drain, OD_SCSI_DRAIN_FAILURE_PROTOCOL);

    assert(od_scsi_drain_init(&close_rollback_drain) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_begin(&close_rollback_drain, 10, 20) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_drain_observe(&close_rollback_drain, 11, 0, 0) == OD_SCSI_DRAIN_CLOSE);
    assert(od_scsi_drain_observe(&close_rollback_drain, 10, 0, 0) == OD_SCSI_DRAIN_ERROR);
    expect_quarantined(&close_rollback_drain, OD_SCSI_DRAIN_FAILURE_PROTOCOL);
    assert(od_scsi_drain_init(NULL) == OD_SCSI_DRAIN_ERROR);
    assert(od_scsi_drain_begin(NULL, 0, 0) == OD_SCSI_DRAIN_ERROR);
    assert(od_scsi_drain_fail(NULL, OD_SCSI_DRAIN_FAILURE_TIMEOUT) == OD_SCSI_DRAIN_ERROR);

    puts("PASS: caller-owned graceful drain state, deadline, quarantine, and no restart");
    return 0;
}
