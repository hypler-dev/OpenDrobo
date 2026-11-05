#ifndef OPENDROBO_READ_ONLY_TASK_POLICY_H
#define OPENDROBO_READ_ONLY_TASK_POLICY_H
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    OD_TBT_PLAN_OK = 0,
    OD_TBT_PLAN_INVALID_ARGUMENT = 1,
    OD_TBT_PLAN_UNSUPPORTED_SCOPE = 2,
    OD_TBT_PLAN_UNSUPPORTED_COMMAND = 3,
    OD_TBT_PLAN_INVALID_TRANSFER = 4,
    OD_TBT_PLAN_DMA_RANGE = 5
} ODTBTPlanStatus;

typedef enum {
    OD_TBT_CHANNEL_STORAGE = 1,
    OD_TBT_CHANNEL_MANAGEMENT = 3,
    OD_TBT_CHANNEL_INLINE = 4
} ODTBTChannel;

/* A projection of the SDK task, not a copy of its binary layout. */
typedef struct {
    uint64_t version;
    uint64_t target_id;
    uint64_t controller_task_id;
    uint64_t requested_bytes;
    uint64_t buffer_iovm_address;
    uint8_t lun[8];
    uint8_t cdb[16];
    uint8_t cdb_length;
    uint8_t direction; /* 0: no data, 1: host-to-device, 2: device-to-host. */
} ODTBTTaskInput;

typedef struct {
    uint64_t controller_task_id;
    uint64_t buffer_iovm_address;
    uint64_t transfer_bytes;
    uint64_t logical_block;
    uint32_t block_count;
    ODTBTChannel channel;
    uint8_t reports_virtual_geometry;
    uint8_t cdb_length;
    uint8_t cdb[16];
} ODTBTReadPlan;

/* Pure planning only: no register, DMA, queue, device, or filesystem access.
 * This initial policy supports target 0/LUN 0 and 512-byte block reads only.
 * The caller owns the mapping; an address-range check does not establish DMA
 * ownership or make an arbitrary numeric address safe to submit.
 * Pointers must be valid/aligned for their structs. No pointer is retained.
 * A failed call clears a valid output. Input/output overlap is supported. */
ODTBTPlanStatus od_tbt_plan_read_only(const ODTBTTaskInput *input,
                                    uint32_t block_size, uint32_t max_transfer_bytes,
                                    ODTBTReadPlan *output);

#ifdef __cplusplus
}
struct SCSIUserParallelTask;
ODTBTPlanStatus od_tbt_plan_driverkit_task(const SCSIUserParallelTask *task,
                                        uint32_t block_size, uint32_t max_transfer_bytes,
                                        ODTBTReadPlan *output);
#endif
#endif
