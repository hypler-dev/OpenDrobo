#include "ReadOnlyTaskPolicy.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

static ODTBTTaskInput capacity_task(void) {
    ODTBTTaskInput task = {0};
    task.version = 1;
    task.controller_task_id = 42;
    task.requested_bytes = 512;
    task.buffer_iovm_address = 0x1000;
    task.direction = 2;
    task.cdb_length = 10;
    const uint8_t cdb[10] = {0x5a, 8, 0x3a, 2, 0, 0, 0, 2, 0, 0};
    memcpy(task.cdb, cdb, sizeof(cdb));
    return task;
}

static void reject(ODTBTTaskInput task, ODTBTPlanStatus expected) {
    ODTBTReadPlan output;
    memset(&output, 0xaa, sizeof(output));
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &output) == expected);
    ODTBTReadPlan empty = {0};
    assert(!memcmp(&output, &empty, sizeof(empty)));
}

int main(void) {
    ODTBTTaskInput task = capacity_task();
    const ODTBTTaskInput original = task;
    ODTBTReadPlan plan;
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &plan) == OD_TBT_PLAN_OK);
    assert(plan.channel == OD_TBT_CHANNEL_MANAGEMENT && plan.transfer_bytes == 512);
    assert(plan.controller_task_id == 42 && !plan.reports_virtual_geometry);
    assert(!memcmp(task.cdb, plan.cdb, 10) && !memcmp(&task, &original, sizeof(task)));
    task.cdb[3] = 4;
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &plan) == OD_TBT_PLAN_OK);

    task = capacity_task(); task.target_id = 1; reject(task, OD_TBT_PLAN_UNSUPPORTED_SCOPE);
    task = capacity_task(); task.lun[7] = 1; reject(task, OD_TBT_PLAN_UNSUPPORTED_SCOPE);
    task = capacity_task(); task.version = 2; reject(task, OD_TBT_PLAN_UNSUPPORTED_SCOPE);
    task = capacity_task(); task.direction = 1; reject(task, OD_TBT_PLAN_UNSUPPORTED_COMMAND);
    task = capacity_task(); task.cdb_length = 17; reject(task, OD_TBT_PLAN_UNSUPPORTED_COMMAND);
    task = capacity_task(); task.cdb_length = 6; reject(task, OD_TBT_PLAN_UNSUPPORTED_COMMAND);
    task = capacity_task(); task.cdb[3] = 3; reject(task, OD_TBT_PLAN_UNSUPPORTED_COMMAND);
    task = capacity_task(); task.cdb[7] = 1; reject(task, OD_TBT_PLAN_UNSUPPORTED_COMMAND);
    task = capacity_task(); task.requested_bytes = 511; reject(task, OD_TBT_PLAN_INVALID_TRANSFER);
    task = capacity_task(); task.requested_bytes = 1024 * 1024 + 1; reject(task, OD_TBT_PLAN_INVALID_TRANSFER);
    task = capacity_task(); task.buffer_iovm_address = UINT32_MAX - 510; reject(task, OD_TBT_PLAN_DMA_RANGE);
    task = capacity_task(); task.buffer_iovm_address = UINT64_MAX; reject(task, OD_TBT_PLAN_DMA_RANGE);
    task = capacity_task(); task.buffer_iovm_address = 0; reject(task, OD_TBT_PLAN_DMA_RANGE);
    task = capacity_task(); task.buffer_iovm_address = UINT32_MAX - 511;
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &plan) == OD_TBT_PLAN_OK);

    /* Mutation and vendor opcodes remain unavailable even with data-in set. */
    const uint8_t writes[] = {0x04, 0x15, 0x1b, 0x1e, 0x2a, 0x35, 0x3b, 0x42, 0x55, 0x8a, 0xaa, 0xea};
    for (unsigned i = 0; i < sizeof(writes); ++i) {
        task = capacity_task(); task.cdb[0] = writes[i];
        reject(task, OD_TBT_PLAN_UNSUPPORTED_COMMAND);
    }

    task = capacity_task(); memset(task.cdb, 0, sizeof(task.cdb));
    task.cdb[0] = 0x28; task.cdb[5] = 7; task.cdb[8] = 2; task.requested_bytes = 1024;
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &plan) == OD_TBT_PLAN_OK);
    assert(plan.channel == OD_TBT_CHANNEL_STORAGE && plan.logical_block == 7 && plan.block_count == 2);
    assert(od_tbt_plan_read_only(&task, 4096, 1024 * 1024, &plan) == OD_TBT_PLAN_UNSUPPORTED_COMMAND);
    task.cdb[8] = 0; reject(task, OD_TBT_PLAN_INVALID_TRANSFER);
    task.cdb[0] = 0x88; task.cdb_length = 16; task.cdb[5] = 0; task.cdb[13] = 2;
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &plan) == OD_TBT_PLAN_OK);
    memset(task.cdb + 2, 0xff, 8); reject(task, OD_TBT_PLAN_INVALID_TRANSFER);

    task = capacity_task(); memset(task.cdb, 0, sizeof(task.cdb)); task.cdb[0] = 0x25;
    task.requested_bytes = 8;
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &plan) == OD_TBT_PLAN_OK);
    assert(plan.reports_virtual_geometry && plan.channel == OD_TBT_CHANNEL_INLINE);
    task.cdb[0] = 0x9e; task.cdb[1] = 0x10; task.cdb[13] = 32; task.cdb_length = 16; task.requested_bytes = 32;
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &plan) == OD_TBT_PLAN_OK);
    assert(plan.reports_virtual_geometry && plan.transfer_bytes == 32);
    task.cdb[1] = 0x11; reject(task, OD_TBT_PLAN_UNSUPPORTED_COMMAND);

    task = capacity_task(); memset(task.cdb, 0, sizeof(task.cdb));
    task.cdb[0] = 0x12; task.cdb[4] = 36; task.cdb_length = 6; task.requested_bytes = 36;
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &plan) == OD_TBT_PLAN_OK);
    assert(plan.channel == OD_TBT_CHANNEL_MANAGEMENT && plan.transfer_bytes == 36);
    task.cdb[1] = 1; task.cdb[2] = 0x80;
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &plan) == OD_TBT_PLAN_OK);
    task.cdb[2] = 0x83; reject(task, OD_TBT_PLAN_UNSUPPORTED_COMMAND);
    memset(task.cdb, 0, sizeof(task.cdb)); task.cdb[0] = 0xa0; task.cdb[9] = 16;
    task.cdb_length = 12; task.requested_bytes = 16;
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &plan) == OD_TBT_PLAN_OK);
    task.cdb[2] = 1; reject(task, OD_TBT_PLAN_UNSUPPORTED_COMMAND);
    memset(task.cdb, 0, sizeof(task.cdb)); task.cdb_length = 6; task.direction = 0; task.requested_bytes = 0;
    assert(od_tbt_plan_read_only(&task, 512, 1024 * 1024, &plan) == OD_TBT_PLAN_OK);
    task.requested_bytes = 1; reject(task, OD_TBT_PLAN_INVALID_TRANSFER);

    memset(&plan, 0xaa, sizeof(plan));
    assert(od_tbt_plan_read_only(NULL, 512, 512, &plan) == OD_TBT_PLAN_INVALID_ARGUMENT);
    ODTBTReadPlan empty = {0}; assert(!memcmp(&plan, &empty, sizeof(plan)));
    assert(od_tbt_plan_read_only(&task, 512, 512, NULL) == OD_TBT_PLAN_INVALID_ARGUMENT);
    puts("PASS: bounded read-only task planning, mutation refusal, scope, DMA ranges, and virtual-geometry distinction");
}
