#include "ReadOnlyTaskPolicy.h"
#include <string.h>

static uint32_t be32(const uint8_t *bytes) {
    return (uint32_t)bytes[0] << 24 | (uint32_t)bytes[1] << 16 |
           (uint32_t)bytes[2] << 8 | bytes[3];
}

static uint64_t be64(const uint8_t *bytes) {
    return (uint64_t)be32(bytes) << 32 | be32(bytes + 4);
}

static int zero(const uint8_t *bytes, unsigned length) {
    for (unsigned i = 0; i < length; ++i) if (bytes[i]) return 0;
    return 1;
}

ODTBTPlanStatus od_tbt_plan_read_only(const ODTBTTaskInput *input,
                                    uint32_t block_size, uint32_t max_transfer_bytes,
                                    ODTBTReadPlan *output) {
    if (!output) return OD_TBT_PLAN_INVALID_ARGUMENT;
    if (!input) { memset(output, 0, sizeof(*output)); return OD_TBT_PLAN_INVALID_ARGUMENT; }
    ODTBTTaskInput task = *input;
    memset(output, 0, sizeof(*output));
    if (!max_transfer_bytes) return OD_TBT_PLAN_INVALID_ARGUMENT;
    if (task.version != 1 || task.target_id || !zero(task.lun, sizeof(task.lun)))
        return OD_TBT_PLAN_UNSUPPORTED_SCOPE;
    if (task.cdb_length > sizeof(task.cdb) || !task.cdb_length)
        return OD_TBT_PLAN_UNSUPPORTED_COMMAND;
    if (task.direction != 0 && task.direction != 2)
        return OD_TBT_PLAN_UNSUPPORTED_COMMAND;

    ODTBTReadPlan plan = {0};
    uint8_t length;
    const uint8_t *cdb = task.cdb;
    switch (cdb[0]) {
        case 0x00: /* TEST UNIT READY; no synthetic response is produced here. */
            length = 6;
            if (!zero(cdb + 1, 5)) return OD_TBT_PLAN_UNSUPPORTED_COMMAND;
            plan.channel = OD_TBT_CHANNEL_INLINE;
            break;
        case 0x12: /* Standard inquiry and the two observed VPD pages. */
            length = 6;
            if ((cdb[1] & ~1U) || cdb[3] || cdb[5] ||
                (!cdb[1] && cdb[2]) || (cdb[1] && cdb[2] != 0 && cdb[2] != 0x80))
                return OD_TBT_PLAN_UNSUPPORTED_COMMAND;
            plan.channel = OD_TBT_CHANNEL_MANAGEMENT;
            plan.transfer_bytes = cdb[4];
            break;
        case 0x25: /* Existing logical geometry, never protected pack capacity. */
            length = 10;
            if (!zero(cdb + 1, 9)) return OD_TBT_PLAN_UNSUPPORTED_COMMAND;
            plan.channel = OD_TBT_CHANNEL_INLINE;
            plan.transfer_bytes = 8;
            plan.reports_virtual_geometry = 1;
            break;
        case 0x9e: /* READ CAPACITY(16), service action 0x10. */
            length = 16;
            if (cdb[1] != 0x10 || !zero(cdb + 2, 8) || cdb[14] || cdb[15] || be32(cdb + 10) < 32)
                return OD_TBT_PLAN_UNSUPPORTED_COMMAND;
            plan.channel = OD_TBT_CHANNEL_INLINE;
            plan.transfer_bytes = 32;
            plan.reports_virtual_geometry = 1;
            break;
        case 0xa0: /* REPORT LUNS, select-report zero only. */
            length = 12;
            if (!zero(cdb + 1, 5) || cdb[10] || cdb[11] || be32(cdb + 6) < 16)
                return OD_TBT_PLAN_UNSUPPORTED_COMMAND;
            plan.channel = OD_TBT_CHANNEL_INLINE;
            plan.transfer_bytes = be32(cdb + 6);
            break;
        case 0x5a: /* The two fixed vendor MODE SENSE pages, data-in only. */
            length = 10;
            if (cdb[1] != 8 || cdb[2] != 0x3a || (cdb[3] != 2 && cdb[3] != 4) ||
                !zero(cdb + 4, 3) || cdb[7] != 2 || cdb[8] || cdb[9])
                return OD_TBT_PLAN_UNSUPPORTED_COMMAND;
            plan.channel = OD_TBT_CHANNEL_MANAGEMENT;
            plan.transfer_bytes = 512;
            break;
        case 0x28: /* READ(10), with no optional flags or group/control bits. */
            length = 10;
            if (block_size != 512 || cdb[1] || cdb[6] || cdb[9])
                return OD_TBT_PLAN_UNSUPPORTED_COMMAND;
            plan.channel = OD_TBT_CHANNEL_STORAGE;
            plan.logical_block = be32(cdb + 2);
            plan.block_count = (uint32_t)cdb[7] << 8 | cdb[8];
            plan.transfer_bytes = (uint64_t)plan.block_count * 512;
            break;
        case 0x88: /* READ(16). */
            length = 16;
            if (block_size != 512 || cdb[1] || cdb[14] || cdb[15])
                return OD_TBT_PLAN_UNSUPPORTED_COMMAND;
            plan.channel = OD_TBT_CHANNEL_STORAGE;
            plan.logical_block = be64(cdb + 2);
            plan.block_count = be32(cdb + 10);
            plan.transfer_bytes = (uint64_t)plan.block_count * 512;
            break;
        default:
            return OD_TBT_PLAN_UNSUPPORTED_COMMAND;
    }
    if (task.cdb_length != length) return OD_TBT_PLAN_UNSUPPORTED_COMMAND;
    if (plan.block_count && plan.logical_block > UINT64_MAX - (plan.block_count - 1U))
        return OD_TBT_PLAN_INVALID_TRANSFER;
    if (cdb[0] == 0x00) {
        if (task.direction || task.requested_bytes) return OD_TBT_PLAN_INVALID_TRANSFER;
    } else {
        if (task.direction != 2 || !plan.transfer_bytes ||
            task.requested_bytes < plan.transfer_bytes || task.requested_bytes > max_transfer_bytes ||
            plan.transfer_bytes > max_transfer_bytes)
            return OD_TBT_PLAN_INVALID_TRANSFER;
        /* The original driver requests 32-bit mappings. This initial policy
         * rejects a range that crosses 4 GiB instead of truncating its address. */
        if (!task.buffer_iovm_address || task.buffer_iovm_address > UINT32_MAX ||
            plan.transfer_bytes - 1 > UINT32_MAX - task.buffer_iovm_address)
            return OD_TBT_PLAN_DMA_RANGE;
    }
    plan.controller_task_id = task.controller_task_id;
    plan.buffer_iovm_address = task.buffer_iovm_address;
    plan.cdb_length = length;
    memcpy(plan.cdb, task.cdb, length);
    *output = plan;
    return OD_TBT_PLAN_OK;
}
