#ifndef OPENDROBO_SCSI_RESPONSES_H
#define OPENDROBO_SCSI_RESPONSES_H

#include "ReadOnlyTaskPolicy.h"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    OD_SCSI_RESPONSE_HANDLED = 0,
    OD_SCSI_RESPONSE_UNSUPPORTED = 1,
    OD_SCSI_RESPONSE_INVALID = 2
} ODScsiResponseStatus;

typedef struct {
    uint32_t block_size;
    uint64_t logical_blocks;
} ODScsiGeometry;

/*
 * This is bridge identity supplied by the project/provider registry. It is not
 * discovered hardware identity. Fixed-width fields are copied exactly; serial
 * is optional explicit printable ASCII and is never synthesized by this component.
 */
typedef struct {
    uint8_t vendor[8];
    uint8_t product[16];
    uint8_t revision[4];
    uint8_t serial[32];
    uint8_t serial_length;
} ODScsiIdentity;

/* Initialize public bridge identity without any serial or registry identifier. */
void od_scsi_bridge_identity(ODScsiIdentity *identity);

/*
 * Writes only bounded CPU-side response bytes. The task, geometry, and identity
 * are copied before output is written and no input pointer is retained. This is
 * not a DMA operation; fBufferIOVMAddr neither authorizes nor affects output.
 * On every returned error, *realized_bytes is zero. A nonzero response requires
 * output_bytes and output_capacity sufficient for the CDB-limited transfer.
 */
ODScsiResponseStatus od_scsi_write_inline_response(
    const ODTBTTaskInput *task, const ODScsiGeometry *geometry,
    const ODScsiIdentity *identity, uint8_t *output_bytes,
    uint32_t output_capacity, uint32_t *realized_bytes);

#ifdef __cplusplus
}
#endif

#endif
