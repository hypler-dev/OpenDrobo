#ifndef OPENDROBO_CONTROLLER_TRANSPORT_H
#define OPENDROBO_CONTROLLER_TRANSPORT_H
#include "DmaOwnership.h"
#include "InterruptProgram.h"
#include "ReadOnlyTaskPolicy.h"
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum { OD_TBT_CONTROLLER_SLOTS = 32, OD_TBT_TM_REQUEST_BYTES = 104 };
typedef enum {
    OD_TBT_CTL_OK = 0, OD_TBT_CTL_ARGUMENT, OD_TBT_CTL_STATE,
    OD_TBT_CTL_NOT_READY, OD_TBT_CTL_UNSUPPORTED, OD_TBT_CTL_FORMAT,
    OD_TBT_CTL_IO, OD_TBT_CTL_DMA, OD_TBT_CTL_TIMEOUT,
    OD_TBT_CTL_QUARANTINED, OD_TBT_CTL_EXHAUSTED
} ODTBTControllerStatus;
typedef enum {
    OD_TBT_CTL_EMPTY = 0, OD_TBT_CTL_RUNNING, OD_TBT_CTL_QUIESCING,
    OD_TBT_CTL_DMA_QUARANTINE, OD_TBT_CTL_STOPPED, OD_TBT_CTL_REMOVED
} ODTBTControllerState;

/* All callbacks return zero on success. The backend owns endian conversion,
 * PCI memory-index lookup, ordering and device lifetime. Calls must be
 * serialized on the same queue as interrupt, timeout and shutdown events. */
typedef struct {
    void *context;
    int (*read32)(void *, uint8_t bar, uint32_t offset, uint32_t *);
    int (*write32)(void *, uint8_t bar, uint32_t offset, uint32_t);
    int (*order)(void *);
    /* Writes through the PREPARED mapping, including any DMA bounce buffer.
     * No byte pointer is retained. Failure must not publish the mapping. */
    int (*write_request)(void *, ODDmaLease *, const void *, uint32_t length);
} ODTBTControllerIO;

typedef struct {
    uint64_t task_id;
    uint32_t transaction;
    uint64_t realized_bytes;
    uint32_t srb_response;
    uint32_t scsi_status;
    uint8_t succeeded;
    uint8_t cancelled;
} ODTBTControllerCompletion;

typedef struct {
    ODDmaLease *request;
    ODDmaLease *payload;
    uint64_t task_id;
    uint64_t deadline_ms;
    uint64_t expected_bytes;
    uint32_t transaction;
} ODTBTControllerSlot;

/* Caller-owned, zero-initialized storage. Never copy or reinitialize a live
 * controller or its leases. There is no restart/reset/sleep/wake path yet. */
typedef struct {
    uint32_t magic;
    ODTBTControllerState state;
    ODTBTControllerIO io;
    uint32_t bar0_bytes, bar1_bytes;
    uint32_t request_offset, response_offset, window, max_transfer;
    uint32_t next_transaction;
    uint64_t last_time_ms;
    uint8_t removed;
    uint8_t device_interrupts_touched;
    ODTBTInterruptProgram request_interrupt, response_interrupt, summary_interrupt;
    ODTBTControllerSlot slots[OD_TBT_CONTROLLER_SLOTS];
} ODTBTController;

/* A one-shot, version-4, target-0/LUN-0 management transport. It validates the
 * port and copied interrupt definitions through bounded BAR accessors, then
 * advertises host version 1. It never opens a provider or enables bus mastering.
 * No original vendor code is linked. Only mock accessors are executed in tests. */
ODTBTControllerStatus od_tbt_controller_init(ODTBTController *, const ODTBTControllerIO *,
                                            uint32_t bar0_bytes, uint32_t bar1_bytes);
ODTBTControllerStatus od_tbt_controller_start(ODTBTController *);
int od_tbt_controller_running_for(const ODTBTController *, const void *io_context);

/* Only management data-in plans are accepted (INQUIRY and the two recovered
 * MODE SENSE pages). Both leases must be valid, prepared, single-segment,
 * nonoverlapping, bound to the same DMA backend/provider, and exclusively transferred to this controller until it
 * reports completion. The write_request backend updates the actual prepared
 * DMA mapping; a CPU virtual address is never treated as its storage. Storage I/O and inline
 * geometry are deliberately unsupported by this initial controller engine. */
ODTBTControllerStatus od_tbt_controller_submit(ODTBTController *, const ODTBTTaskInput *,
                                             ODDmaLease *request, ODDmaLease *payload,
                                             uint64_t now_ms, uint64_t deadline_ms,
                                             uint32_t *transaction);

/* Explicit ticks use a monotonic millisecond clock; no sleeping or busy loop.
 * Interrupt callers use poll. Output storage needs room for all 32 entries.
 * Each accepted completion is reported at most once. No data is interpreted as
 * physical/protected capacity by this layer. */
ODTBTControllerStatus od_tbt_controller_poll(ODTBTController *, uint64_t now_ms,
                                           ODTBTControllerCompletion *, size_t capacity,
                                           size_t *count);
ODTBTControllerStatus od_tbt_controller_tick(ODTBTController *, uint64_t now_ms);

/* Begin blocks new work before masking device interrupts and quarantining
 * mappings. Removal skips all MMIO. The SDK source must then be disabled and
 * cancelled asynchronously. A timeout, disconnect, source cancellation or PCI
 * Close is NOT proof that DMA drained. finish_stop retains all live DMA until
 * BOTH interrupt drain and independent bus-master quiescence are attested.
 * The current SDK session cannot establish the latter; no automatic cleanup
 * path invents it. A failed CompleteDMA retains resources for a later retry. */
ODTBTControllerStatus od_tbt_controller_begin_stop(ODTBTController *, int removed);
ODTBTControllerStatus od_tbt_controller_finish_stop(ODTBTController *, int interrupts_drained,
                                                  int bus_master_quiescent,
                                                  ODTBTControllerCompletion *, size_t capacity,
                                                  size_t *count);
#ifdef __cplusplus
}
#endif
#endif
