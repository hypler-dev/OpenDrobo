#ifndef OPENDROBO_DMA_OWNERSHIP_H
#define OPENDROBO_DMA_OWNERSHIP_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
    OD_DMA_MAX_TRANSFER_BYTES = 1024U * 1024U,
    OD_DMA_MAX_SEGMENTS = 32U,
    OD_DMA_MAX_SEGMENT_BYTES = 512U * 1024U,
    OD_DMA_DEVICE_READ = 1U,  /* Device may read host memory. */
    OD_DMA_DEVICE_WRITE = 2U  /* Device may write host memory. */
};

typedef enum {
    OD_DMA_STATUS_OK = 0,
    OD_DMA_STATUS_INVALID_ARGUMENT = 1,
    OD_DMA_STATUS_BAD_STATE = 2,
    OD_DMA_STATUS_RANGE = 3,
    OD_DMA_STATUS_RETAIN_FAILED = 4,
    OD_DMA_STATUS_CREATE_FAILED = 5,
    OD_DMA_STATUS_PREPARE_FAILED = 6,
    OD_DMA_STATUS_MAPPING_INVALID = 7,
    OD_DMA_STATUS_COMPLETE_FAILED = 8
} ODDmaStatus;

typedef enum {
    OD_DMA_LEASE_EMPTY = 0,
    OD_DMA_LEASE_PREPARED = 1,
    OD_DMA_LEASE_PUBLISHED = 2,
    OD_DMA_LEASE_COMPLETED = 3,
    OD_DMA_LEASE_QUARANTINED = 4
} ODDmaLeaseState;

/* This is an IOVA/physical DMA segment, never a CPU pointer. */
typedef struct {
    uint64_t address;
    uint64_t length;
} ODDmaSegment;

/*
 * Backend handles are intentionally opaque. A successful retain_memory call
 * transfers one memory retain to the lease. create_command returns one command
 * retain to the lease. prepare must map exactly offset..offset + length and
 * return at most OD_DMA_MAX_SEGMENTS segments and device access permissions.
 * prepare returns zero for a mapped range, negative when no mapping exists,
 * and positive when mapping ownership is uncertain. A positive result keeps
 * the lease prepared so the caller can CompleteDMA before releasing it.
 * complete is retryable: a failure leaves the command and retained memory
 * owned by the lease.
 */
typedef struct {
    void *context;
    int (*retain_memory)(void *context, void *memory_handle);
    void (*release_memory)(void *context, void *memory_handle);
    int (*create_command)(void *context, uint32_t max_address_bits,
                          void **command_handle);
    void (*release_command)(void *context, void *command_handle);
    int (*prepare)(void *context, void *command_handle, void *memory_handle,
                   uint64_t offset, uint64_t length, ODDmaSegment *segments,
                   uint32_t *segment_count, uint32_t *device_access);
    int (*complete)(void *context, void *command_handle);
} ODDmaBackend;

/*
 * Caller-owned storage. memory_handle and command_handle are opaque ownership
 * tokens only; neither is exposed as a DMA address. Do not reset or overwrite
 * a prepared, published, or quarantined lease. There is deliberately no
 * destructor which releases an in-flight mapping.
 */
typedef struct {
    uint32_t magic;
    ODDmaLeaseState state;
    uint32_t mapping_valid;
    uint32_t device_access;
    uint32_t segment_count;
    uint64_t offset;
    uint64_t requested_length;
    void *memory_handle;
    void *command_handle;
    ODDmaBackend backend;
    ODDmaSegment segments[OD_DMA_MAX_SEGMENTS];
} ODDmaLease;

#define OD_DMA_LEASE_MAGIC UINT32_C(0x4f44444d)
#define OD_DMA_LEASE_INITIALIZER { OD_DMA_LEASE_MAGIC, OD_DMA_LEASE_EMPTY, 0U, 0U, 0U, 0U, 0U, 0, 0, { 0 }, { { 0U, 0U } } }

/* Safe only for new, empty, or completed caller-owned storage. */
ODDmaStatus od_dma_lease_init(ODDmaLease *lease);

/* Pure mapping check used after a backend prepares an exact requested range. */
ODDmaStatus od_dma_validate_mapping(const ODDmaSegment *segments,
                                    uint32_t segment_count,
                                    uint64_t requested_length,
                                    uint32_t device_access);

/*
 * A negative backend prepare failure returns the lease to EMPTY. A positive
 * (uncertain) backend failure, or a failed mapping check after preparation,
 * leaves the lease PREPARED but unpublished; call abort_unpublished to
 * CompleteDMA and release it, retrying that operation if CompleteDMA fails.
 */
ODDmaStatus od_dma_lease_prepare(ODDmaLease *lease, const ODDmaBackend *backend,
                                 void *memory_handle, uint64_t offset,
                                 uint64_t requested_length);
ODDmaStatus od_dma_lease_publish(ODDmaLease *lease);
ODDmaStatus od_dma_lease_complete(ODDmaLease *lease);
ODDmaStatus od_dma_lease_abort_unpublished(ODDmaLease *lease);

/* A published timeout/removal never releases DMA ownership by itself. */
ODDmaStatus od_dma_lease_quarantine_published(ODDmaLease *lease);

/* Caller attests that bus mastering is proven quiescent before this call. */
ODDmaStatus od_dma_lease_release_after_bus_master_quiescence(ODDmaLease *lease);

/* Releases completed bookkeeping only. It refuses all live DMA states. */
ODDmaStatus od_dma_lease_release(ODDmaLease *lease);

#ifdef __cplusplus
}
#endif

#endif
