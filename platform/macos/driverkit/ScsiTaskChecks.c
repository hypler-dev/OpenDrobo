#include "ScsiTaskTable.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct { ODScsiTaskTable table; unsigned retains, releases, completions; } Fixture;
static void retain(void *ctx, void *action) { assert(action); ++((Fixture *)ctx)->retains; }
static void release(void *ctx, void *action) { assert(action); ++((Fixture *)ctx)->releases; }
static void complete(void *ctx, void *action, const ODTBTControllerCompletion *c) {
    Fixture *f = ctx; assert(action && c);
    ++f->completions;
    /* The entry is removed before callbacks, preventing duplicate completion
     * even when the framework callback reenters task processing. */
    assert(od_scsi_tasks_finish(&f->table, c) == OD_SCSI_TASK_MISMATCH);
}
typedef struct { ODScsiTaskTable *table; unsigned retains, releases, completions; } Destructive;
static void destructive_retain(void *ctx, void *action) { assert(action); ++((Destructive *)ctx)->retains; }
static void destructive_release(void *ctx, void *action) { assert(action); ++((Destructive *)ctx)->releases; }
static void destructive_complete(void *ctx, void *action, const ODTBTControllerCompletion *c) {
    Destructive *d = ctx; assert(action && c); ++d->completions;
    free(d->table); d->table = NULL;
}
int main(void) {
    assert(!od_scsi_service_hardware_enabled());
    Fixture f = {0}; ODScsiTaskCallbacks cb = {&f, retain, release, complete};
    assert(od_scsi_tasks_init(&f.table, &cb) == OD_SCSI_TASK_OK);
    ODDmaLease request = OD_DMA_LEASE_INITIALIZER, payload = OD_DMA_LEASE_INITIALIZER;
    uint32_t slot = 99; int action = 1;
    assert(od_scsi_tasks_reserve(&f.table, 7, &action, &request, &payload, &slot) == OD_SCSI_TASK_OK);
    assert(od_scsi_tasks_reserve(&f.table, 7, &action, 0, 0, &slot) == OD_SCSI_TASK_BUSY);
    assert(od_scsi_tasks_init(&f.table, &cb) == OD_SCSI_TASK_BUSY);
    assert(od_scsi_tasks_bind(&f.table, slot, 3) == OD_SCSI_TASK_OK);
    assert(od_scsi_tasks_bind(&f.table, slot, 4) == OD_SCSI_TASK_MISMATCH);
    ODTBTControllerCompletion c = {.task_id = 7, .transaction = 4, .cancelled = 1};
    assert(od_scsi_tasks_finish(&f.table, &c) == OD_SCSI_TASK_MISMATCH);
    c.transaction = 3;
    const ODDmaLeaseState live[] = {OD_DMA_LEASE_PREPARED, OD_DMA_LEASE_PUBLISHED, OD_DMA_LEASE_QUARANTINED};
    for (unsigned i = 0; i < sizeof(live) / sizeof(*live); ++i) {
        request.state = live[i]; payload.state = OD_DMA_LEASE_COMPLETED;
        assert(od_scsi_tasks_finish(&f.table, &c) == OD_SCSI_TASK_DMA_LIVE);
        payload.state = live[i]; request.state = OD_DMA_LEASE_COMPLETED;
        assert(od_scsi_tasks_finish(&f.table, &c) == OD_SCSI_TASK_DMA_LIVE);
        assert(!f.completions && !f.releases && od_scsi_tasks_pending(&f.table) == 1);
    }
    request.state = payload.state = OD_DMA_LEASE_COMPLETED;
    assert(od_scsi_tasks_finish(&f.table, &c) == OD_SCSI_TASK_OK);
    assert(f.completions == 1 && f.retains == 1 && f.releases == 1);
    assert(od_scsi_tasks_finish(&f.table, &c) == OD_SCSI_TASK_MISMATCH);
    assert(od_scsi_tasks_reserve(&f.table, 7, &action, 0, 0, &slot) == OD_SCSI_TASK_OK);
    c.transaction = 0; c.cancelled = 0; c.succeeded = 1;
    assert(od_scsi_tasks_finish(&f.table, &c) == OD_SCSI_TASK_OK);
    assert(f.completions == 2 && f.retains == f.releases);
    for (unsigned i = 0; i < 32; ++i)
        assert(od_scsi_tasks_reserve(&f.table, i, &action, 0, 0, &slot) == OD_SCSI_TASK_OK);
    assert(od_scsi_tasks_reserve(&f.table, 100, &action, 0, 0, &slot) == OD_SCSI_TASK_BUSY);
    for (unsigned i = 0; i < 32; ++i) {
        c.task_id = i;
        assert(od_scsi_tasks_finish(&f.table, &c) == OD_SCSI_TASK_OK);
    }
    assert(!od_scsi_tasks_pending(&f.table) && f.retains == f.releases && f.completions == f.releases);
    Destructive d = {0}; d.table = calloc(1, sizeof(*d.table)); assert(d.table);
    ODScsiTaskCallbacks destructive = {&d, destructive_retain, destructive_release, destructive_complete};
    assert(od_scsi_tasks_init(d.table, &destructive) == OD_SCSI_TASK_OK);
    assert(od_scsi_tasks_reserve(d.table, 42, &action, NULL, NULL, &slot) == OD_SCSI_TASK_OK);
    c.task_id = 42;
    assert(od_scsi_tasks_finish(d.table, &c) == OD_SCSI_TASK_OK);
    assert(!d.table && d.retains == 1 && d.releases == 1 && d.completions == 1);
    puts("PASS: SCSI action ownership, completion association, reentrancy, DMA gates and lab disable");
    return 0;
}
