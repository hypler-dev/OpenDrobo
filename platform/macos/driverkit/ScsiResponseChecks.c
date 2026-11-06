#include "ScsiResponses.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

static ODScsiGeometry geometry(uint64_t logical_blocks) {
    ODScsiGeometry result = {512, logical_blocks};
    return result;
}

static ODScsiIdentity identity(void) {
    ODScsiIdentity result = {0};
    static const uint8_t vendor[8] = {'O', 'P', 'E', 'N', 'D', 'R', 'O', ' '};
    static const uint8_t product[16] = {'B', 'R', 'I', 'D', 'G', 'E', ' ', 'S', 'C', 'S', 'I', ' ', ' ', ' ', ' ', ' '};
    static const uint8_t revision[4] = {'0', '0', '0', '1'};
    static const uint8_t serial[] = "BRIDGE-0001";
    memcpy(result.vendor, vendor, sizeof(vendor));
    memcpy(result.product, product, sizeof(product));
    memcpy(result.revision, revision, sizeof(revision));
    memcpy(result.serial, serial, sizeof(serial) - 1);
    result.serial_length = sizeof(serial) - 1;
    return result;
}

static ODTBTTaskInput task_for(const uint8_t *cdb, uint8_t cdb_length,
                                uint8_t direction, uint64_t requested_bytes) {
    ODTBTTaskInput task = {0};
    task.version = 1;
    task.cdb_length = cdb_length;
    task.direction = direction;
    task.requested_bytes = requested_bytes;
    memcpy(task.cdb, cdb, cdb_length);
    return task;
}

static void expect_error(ODTBTTaskInput task, ODScsiGeometry geometry_value,
                         ODScsiIdentity identity_value, ODScsiResponseStatus expected) {
    uint8_t output[36];
    uint32_t realized = 99;
    memset(output, 0xa5, sizeof(output));
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         output, sizeof(output), &realized) == expected);
    assert(realized == 0);
}

int main(void) {
    const uint8_t tur[6] = {0x00};
    const uint8_t capacity10[10] = {0x25};
    const uint8_t capacity16[16] = {0x9e, 0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32};
    const uint8_t report_luns[12] = {0xa0, 0, 0, 0, 0, 0, 0, 0, 0, 16};
    const uint8_t inquiry[6] = {0x12, 0, 0, 0, 36, 0};
    const uint8_t vpd00[6] = {0x12, 1, 0, 0, 6, 0};
    const uint8_t vpd80[6] = {0x12, 1, 0x80, 0, 36, 0};
    const uint64_t sixty_four_tib_blocks = UINT64_C(0x2000000000);
    ODScsiGeometry geometry_value = geometry(sixty_four_tib_blocks);
    ODScsiIdentity identity_value = identity();
    ODTBTTaskInput task;
    uint8_t output[36];
    uint32_t realized;

    task = task_for(tur, sizeof(tur), 0, 0);
    realized = 99;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         NULL, 0, &realized) == OD_SCSI_RESPONSE_HANDLED);
    assert(realized == 0);

    task = task_for(capacity10, sizeof(capacity10), 2, 8);
    memset(output, 0xa5, sizeof(output)); realized = 0;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         output, sizeof(output), &realized) == OD_SCSI_RESPONSE_HANDLED);
    assert(realized == 8);
    { const uint8_t expected[8] = {0xff, 0xff, 0xff, 0xff, 0, 0, 2, 0};
      assert(memcmp(output, expected, sizeof(expected)) == 0); }

    task = task_for(capacity16, sizeof(capacity16), 2, 32);
    memset(output, 0xa5, sizeof(output)); realized = 0;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         output, sizeof(output), &realized) == OD_SCSI_RESPONSE_HANDLED);
    assert(realized == 32);
    { const uint8_t expected[32] = {
          0, 0, 0, 0x1f, 0xff, 0xff, 0xff, 0xff, 0, 0, 2, 0,
          0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
      assert(memcmp(output, expected, sizeof(expected)) == 0); }

    task = task_for(report_luns, sizeof(report_luns), 2, 16);
    memset(output, 0xa5, sizeof(output)); realized = 0;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         output, sizeof(output), &realized) == OD_SCSI_RESPONSE_HANDLED);
    assert(realized == 16);
    { const uint8_t expected[16] = {0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
      assert(memcmp(output, expected, sizeof(expected)) == 0); }

    task = task_for(inquiry, sizeof(inquiry), 2, 36);
    memset(output, 0xa5, sizeof(output)); realized = 0;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         output, sizeof(output), &realized) == OD_SCSI_RESPONSE_HANDLED);
    assert(realized == 36);
    { const uint8_t expected[36] = {
          0, 0, 6, 2, 31, 0, 0, 0,
          'O', 'P', 'E', 'N', 'D', 'R', 'O', ' ',
          'B', 'R', 'I', 'D', 'G', 'E', ' ', 'S', 'C', 'S', 'I', ' ', ' ', ' ', ' ', ' ',
          '0', '0', '0', '1'};
      assert(memcmp(output, expected, sizeof(expected)) == 0); }

    task = task_for(vpd00, sizeof(vpd00), 2, 6);
    memset(output, 0xa5, sizeof(output)); realized = 0;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         output, sizeof(output), &realized) == OD_SCSI_RESPONSE_HANDLED);
    assert(realized == 6);
    { const uint8_t expected[6] = {0, 0, 0, 2, 0, 0x80};
      assert(memcmp(output, expected, sizeof(expected)) == 0); }

    task = task_for(vpd80, sizeof(vpd80), 2, 36);
    memset(output, 0xa5, sizeof(output)); realized = 0;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         output, sizeof(output), &realized) == OD_SCSI_RESPONSE_HANDLED);
    assert(realized == 15);
    { const uint8_t expected[15] = {0, 0x80, 0, 11, 'B', 'R', 'I', 'D', 'G', 'E', '-', '0', '0', '0', '1'};
      assert(memcmp(output, expected, sizeof(expected)) == 0); }

    task = task_for(inquiry, sizeof(inquiry), 2, 1);
    task.cdb[4] = 1;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         output, 1, &realized) == OD_SCSI_RESPONSE_HANDLED);
    assert(realized == 1 && output[0] == 0);
    task = task_for(inquiry, sizeof(inquiry), 2, 1);
    expect_error(task, geometry_value, identity_value, OD_SCSI_RESPONSE_INVALID);
    task = task_for(vpd80, sizeof(vpd80), 2, 15);
    realized = 99;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         output, 14, &realized) == OD_SCSI_RESPONSE_INVALID);
    assert(realized == 0);
    task = task_for(inquiry, sizeof(inquiry), 2, 0); task.cdb[4] = 0;
    realized = 99;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         NULL, 0, &realized) == OD_SCSI_RESPONSE_HANDLED);
    assert(realized == 0);

    task = task_for(capacity10, sizeof(capacity10), 2, 8); task.target_id = 1;
    expect_error(task, geometry_value, identity_value, OD_SCSI_RESPONSE_UNSUPPORTED);
    task = task_for(capacity10, sizeof(capacity10), 2, 8); task.lun[0] = 1;
    expect_error(task, geometry_value, identity_value, OD_SCSI_RESPONSE_UNSUPPORTED);
    task = task_for(capacity10, sizeof(capacity10), 1, 8);
    expect_error(task, geometry_value, identity_value, OD_SCSI_RESPONSE_UNSUPPORTED);
    task = task_for(capacity10, sizeof(capacity10), 2, 8); task.cdb[0] = 0x2a;
    expect_error(task, geometry_value, identity_value, OD_SCSI_RESPONSE_UNSUPPORTED);
    task = task_for(capacity10, sizeof(capacity10), 0, 8);
    expect_error(task, geometry_value, identity_value, OD_SCSI_RESPONSE_INVALID);
    task = task_for(capacity10, sizeof(capacity10), 2, 7);
    expect_error(task, geometry_value, identity_value, OD_SCSI_RESPONSE_INVALID);
    task = task_for(capacity10, sizeof(capacity10), 2, 8);
    realized = 99;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         output, 7, &realized) == OD_SCSI_RESPONSE_INVALID);
    assert(realized == 0);
    task = task_for(capacity10, sizeof(capacity10), 2, 8); task.buffer_iovm_address = UINT64_MAX;
    realized = 0;
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value,
                                         output, sizeof(output), &realized) == OD_SCSI_RESPONSE_HANDLED);
    assert(realized == 8);

    expect_error(task_for(capacity10, sizeof(capacity10), 2, 8), geometry(0), identity_value,
                 OD_SCSI_RESPONSE_INVALID);
    expect_error(task_for(capacity10, sizeof(capacity10), 2, 8),
                 (ODScsiGeometry){4096, 1}, identity_value, OD_SCSI_RESPONSE_INVALID);
    expect_error(task_for(capacity10, sizeof(capacity10), 2, 8),
                 geometry(UINT64_MAX / 512 + 1), identity_value, OD_SCSI_RESPONSE_INVALID);
    identity_value.serial_length = 33;
    expect_error(task_for(vpd80, sizeof(vpd80), 2, 36), geometry_value, identity_value,
                 OD_SCSI_RESPONSE_INVALID);
    identity_value = identity(); identity_value.serial[0] = 0x1f;
    expect_error(task_for(vpd80, sizeof(vpd80), 2, 36), geometry_value, identity_value,
                 OD_SCSI_RESPONSE_INVALID);

    od_scsi_bridge_identity(&identity_value);
    assert(identity_value.serial_length == 0);
    for (size_t i = 0; i < sizeof(identity_value.serial); ++i) assert(identity_value.serial[i] == 0);
    task = task_for(inquiry, sizeof(inquiry), 2, 36);
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value, output, sizeof(output), &realized) == OD_SCSI_RESPONSE_HANDLED);
    task = task_for(vpd00, sizeof(vpd00), 2, 6);
    assert(od_scsi_write_inline_response(&task, &geometry_value, &identity_value, output, sizeof(output), &realized) == OD_SCSI_RESPONSE_HANDLED);
    assert(realized == 5 && output[3] == 1 && output[4] == 0);
    expect_error(task_for(vpd80, sizeof(vpd80), 2, 36), geometry_value, identity_value, OD_SCSI_RESPONSE_UNSUPPORTED);

    puts("PASS: bounded inline SCSI responses, exact CDB scope, geometry, and identity");
    return 0;
}
