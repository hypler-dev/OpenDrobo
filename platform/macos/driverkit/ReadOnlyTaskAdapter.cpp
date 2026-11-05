#include "ReadOnlyTaskPolicy.h"
#include <SCSIControllerDriverKit/IOUserSCSIParallelInterfaceController.h>
#include <PCIDriverKit/IOPCIDevice.h>
#include <DriverKit/IODMACommand.h>
#include <string.h>

static_assert(kSCSIDataTransfer_NoDataTransfer == 0);
static_assert(kSCSIDataTransfer_FromInitiatorToTarget == 1);
static_assert(kSCSIDataTransfer_FromTargetToInitiator == 2);
static_assert(kScsiUserParallelTaskCurrentVersion1 == 1);
static_assert(sizeof(SCSIUserParallelTask::fCommandDescriptorBlock) == 16);
static_assert(sizeof(SCSIUserParallelTask::fLogicalUnitBytes) == 8);

ODTBTPlanStatus od_tbt_plan_driverkit_task(const SCSIUserParallelTask *task,
                                        uint32_t block_size, uint32_t max_transfer_bytes,
                                        ODTBTReadPlan *output) {
    if (!task) return od_tbt_plan_read_only(nullptr, block_size, max_transfer_bytes, output);
    ODTBTTaskInput input = {};
    input.version = task->version;
    input.target_id = task->fTargetID;
    input.controller_task_id = task->fControllerTaskIdentifier;
    input.requested_bytes = task->fRequestedTransferCount;
    input.buffer_iovm_address = task->fBufferIOVMAddr;
    input.cdb_length = task->fCommandSize;
    input.direction = task->fTransferDirection;
    memcpy(input.lun, task->fLogicalUnitBytes, sizeof(input.lun));
    memcpy(input.cdb, task->fCommandDescriptorBlock, sizeof(input.cdb));
    return od_tbt_plan_read_only(&input, block_size, max_transfer_bytes, output);
}
