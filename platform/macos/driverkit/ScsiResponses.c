#include "ScsiResponses.h"

#include <limits.h>
#include <string.h>

static int all_zero(const uint8_t *bytes, uint32_t length) {
    uint32_t i;
    for (i = 0; i < length; ++i) {
        if (bytes[i] != 0) return 0;
    }
    return 1;
}

static uint32_t read_be32(const uint8_t *bytes) {
    return (uint32_t)bytes[0] << 24 | (uint32_t)bytes[1] << 16 |
           (uint32_t)bytes[2] << 8 | (uint32_t)bytes[3];
}

static void write_be32(uint8_t *bytes, uint32_t value) {
    bytes[0] = (uint8_t)(value >> 24);
    bytes[1] = (uint8_t)(value >> 16);
    bytes[2] = (uint8_t)(value >> 8);
    bytes[3] = (uint8_t)value;
}

static void write_be64(uint8_t *bytes, uint64_t value) {
    write_be32(bytes, (uint32_t)(value >> 32));
    write_be32(bytes + 4, (uint32_t)value);
}

void od_scsi_bridge_identity(ODScsiIdentity *identity) {
    if (!identity) return;
    memset(identity, 0, sizeof(*identity));
    memcpy(identity->vendor, "OpenDrobo", 8);
    memcpy(identity->product, "Read-only bridge", 16);
    memcpy(identity->revision, "0001", 4);
}

static int serial_is_printable_ascii(const ODScsiIdentity *identity) {
    uint8_t i;
    if (identity->serial_length > sizeof(identity->serial))
        return 0;
    for (i = 0; i < identity->serial_length; ++i) {
        if (identity->serial[i] < 0x20 || identity->serial[i] > 0x7e) return 0;
    }
    return 1;
}

static uint32_t minimum(uint32_t left, uint32_t right) {
    return left < right ? left : right;
}

ODScsiResponseStatus od_scsi_write_inline_response(
    const ODTBTTaskInput *task_input, const ODScsiGeometry *geometry_input,
    const ODScsiIdentity *identity_input, uint8_t *output_bytes,
    uint32_t output_capacity, uint32_t *realized_bytes) {
    ODTBTTaskInput task;
    ODScsiGeometry geometry;
    ODScsiIdentity identity;
    uint8_t response[36] = {0};
    uint32_t response_length = 0;
    uint32_t allocation_length = 0;
    uint32_t transfer_length = 0;
    uint64_t last_lba;
    uint8_t opcode;

    if (!realized_bytes) return OD_SCSI_RESPONSE_INVALID;
    *realized_bytes = 0;
    if (!task_input || !geometry_input || !identity_input)
        return OD_SCSI_RESPONSE_INVALID;

    task = *task_input;
    geometry = *geometry_input;
    identity = *identity_input;
    if (geometry.block_size != 512 || geometry.logical_blocks == 0 ||
        geometry.logical_blocks > UINT64_MAX / geometry.block_size ||
        !serial_is_printable_ascii(&identity))
        return OD_SCSI_RESPONSE_INVALID;
    if (task.version != 1 || task.target_id != 0 || !all_zero(task.lun, sizeof(task.lun)))
        return OD_SCSI_RESPONSE_UNSUPPORTED;
    if (task.cdb_length == 0 || task.cdb_length > sizeof(task.cdb))
        return OD_SCSI_RESPONSE_UNSUPPORTED;

    opcode = task.cdb[0];
    switch (opcode) {
        case 0x00: /* TEST UNIT READY */
            if (task.cdb_length != 6 || !all_zero(task.cdb + 1, 5))
                return OD_SCSI_RESPONSE_UNSUPPORTED;
            if (task.direction != 0 || task.requested_bytes != 0)
                return OD_SCSI_RESPONSE_INVALID;
            return OD_SCSI_RESPONSE_HANDLED;

        case 0x25: /* READ CAPACITY(10) */
            if (task.cdb_length != 10 || !all_zero(task.cdb + 1, 9))
                return OD_SCSI_RESPONSE_UNSUPPORTED;
            response_length = 8;
            allocation_length = response_length;
            last_lba = geometry.logical_blocks - 1;
            write_be32(response, last_lba > UINT32_MAX ? UINT32_MAX : (uint32_t)last_lba);
            write_be32(response + 4, geometry.block_size);
            break;

        case 0x9e: /* READ CAPACITY(16), service action 0x10 */
            if (task.cdb_length != 16 || task.cdb[1] != 0x10 ||
                !all_zero(task.cdb + 2, 8) || task.cdb[14] != 0 || task.cdb[15] != 0)
                return OD_SCSI_RESPONSE_UNSUPPORTED;
            response_length = 32;
            allocation_length = read_be32(task.cdb + 10);
            write_be64(response, geometry.logical_blocks - 1);
            write_be32(response + 8, geometry.block_size);
            break;

        case 0xa0: /* REPORT LUNS, select-report 0 only */
            if (task.cdb_length != 12 || !all_zero(task.cdb + 1, 5) ||
                task.cdb[10] != 0 || task.cdb[11] != 0)
                return OD_SCSI_RESPONSE_UNSUPPORTED;
            response_length = 16;
            allocation_length = read_be32(task.cdb + 6);
            write_be32(response, 8); /* One eight-byte LUN, LUN 0. */
            break;

        case 0x12: /* INQUIRY: standard, VPD 00, and VPD 80 only */
            if (task.cdb_length != 6 || (task.cdb[1] & 0xfeU) != 0 ||
                task.cdb[3] != 0 || task.cdb[5] != 0)
                return OD_SCSI_RESPONSE_UNSUPPORTED;
            allocation_length = task.cdb[4];
            if (task.cdb[1] == 0) {
                if (task.cdb[2] != 0) return OD_SCSI_RESPONSE_UNSUPPORTED;
                response_length = 36;
                response[2] = 0x06;
                response[3] = 0x02;
                response[4] = 31;
                memcpy(response + 8, identity.vendor, sizeof(identity.vendor));
                memcpy(response + 16, identity.product, sizeof(identity.product));
                memcpy(response + 32, identity.revision, sizeof(identity.revision));
            } else if (task.cdb[2] == 0x00) {
                response_length = identity.serial_length ? 6 : 5;
                response[3] = identity.serial_length ? 2 : 1;
                response[4] = 0x00;
                if (identity.serial_length) response[5] = 0x80;
            } else if (task.cdb[2] == 0x80) {
                if (!identity.serial_length) return OD_SCSI_RESPONSE_UNSUPPORTED;
                response_length = 4U + identity.serial_length;
                response[1] = 0x80;
                response[3] = identity.serial_length;
                memcpy(response + 4, identity.serial, identity.serial_length);
            } else {
                return OD_SCSI_RESPONSE_UNSUPPORTED;
            }
            break;

        default:
            return OD_SCSI_RESPONSE_UNSUPPORTED;
    }

    if (task.direction == 1) return OD_SCSI_RESPONSE_UNSUPPORTED;
    if (task.direction != 2) return OD_SCSI_RESPONSE_INVALID;
    transfer_length = minimum(response_length, allocation_length);
    if (task.requested_bytes < transfer_length ||
        (transfer_length != 0 && (!output_bytes || output_capacity < transfer_length)))
        return OD_SCSI_RESPONSE_INVALID;
    if (transfer_length != 0) memcpy(output_bytes, response, transfer_length);
    *realized_bytes = transfer_length;
    return OD_SCSI_RESPONSE_HANDLED;
}
