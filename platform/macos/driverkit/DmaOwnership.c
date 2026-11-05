#include "DmaOwnership.h"

#include <limits.h>
#include <string.h>

static int backend_is_complete(const ODDmaBackend *backend) {
    return backend && backend->retain_memory && backend->release_memory &&
           backend->create_command && backend->release_command &&
           backend->prepare && backend->complete;
}

static void clear_live_fields(ODDmaLease *lease) {
    lease->mapping_valid = 0;
    lease->device_access = 0;
    lease->segment_count = 0;
    lease->offset = 0;
    lease->requested_length = 0;
    lease->memory_handle = 0;
    lease->command_handle = 0;
    memset(&lease->backend, 0, sizeof(lease->backend));
    memset(lease->segments, 0, sizeof(lease->segments));
}

static void release_completed_resources(ODDmaLease *lease) {
    lease->backend.release_command(lease->backend.context, lease->command_handle);
    lease->backend.release_memory(lease->backend.context, lease->memory_handle);
    clear_live_fields(lease);
    lease->state = OD_DMA_LEASE_COMPLETED;
}

ODDmaStatus od_dma_lease_init(ODDmaLease *lease) {
    if (!lease) return OD_DMA_STATUS_INVALID_ARGUMENT;
    if (lease->magic == OD_DMA_LEASE_MAGIC &&
        (lease->state == OD_DMA_LEASE_PREPARED ||
         lease->state == OD_DMA_LEASE_PUBLISHED ||
         lease->state == OD_DMA_LEASE_QUARANTINED))
        return OD_DMA_STATUS_BAD_STATE;
    memset(lease, 0, sizeof(*lease));
    lease->magic = OD_DMA_LEASE_MAGIC;
    return OD_DMA_STATUS_OK;
}

ODDmaStatus od_dma_validate_mapping(const ODDmaSegment *segments,
                                    uint32_t segment_count,
                                    uint64_t requested_length,
                                    uint32_t device_access) {
    const uint32_t allowed_access = OD_DMA_DEVICE_READ | OD_DMA_DEVICE_WRITE;
    uint64_t total = 0;
    if (!requested_length || requested_length > OD_DMA_MAX_TRANSFER_BYTES ||
        !segments || !segment_count || segment_count > OD_DMA_MAX_SEGMENTS ||
        !device_access || device_access & ~allowed_access)
        return OD_DMA_STATUS_RANGE;
    for (uint32_t i = 0; i < segment_count; ++i) {
        const uint64_t address = segments[i].address;
        const uint64_t length = segments[i].length;
        if (!length || length > OD_DMA_MAX_SEGMENT_BYTES || address > UINT32_MAX ||
            length - 1U > UINT32_MAX - address || total > UINT64_MAX - length)
            return OD_DMA_STATUS_RANGE;
        total += length;
    }
    return total == requested_length ? OD_DMA_STATUS_OK : OD_DMA_STATUS_RANGE;
}

ODDmaStatus od_dma_lease_prepare(ODDmaLease *lease, const ODDmaBackend *backend,
                                 void *memory_handle, uint64_t offset,
                                 uint64_t requested_length) {
    if (!lease || !backend_is_complete(backend) || !memory_handle || !requested_length ||
        requested_length > OD_DMA_MAX_TRANSFER_BYTES ||
        offset > UINT64_MAX - requested_length)
        return OD_DMA_STATUS_INVALID_ARGUMENT;
    if (lease->magic != OD_DMA_LEASE_MAGIC || lease->state != OD_DMA_LEASE_EMPTY)
        return OD_DMA_STATUS_BAD_STATE;

    if (backend->retain_memory(backend->context, memory_handle))
        return OD_DMA_STATUS_RETAIN_FAILED;
    void *command_handle = 0;
    if (backend->create_command(backend->context, 32U, &command_handle) || !command_handle) {
        backend->release_memory(backend->context, memory_handle);
        return OD_DMA_STATUS_CREATE_FAILED;
    }

    ODDmaLease prepared = OD_DMA_LEASE_INITIALIZER;
    prepared.magic = OD_DMA_LEASE_MAGIC;
    prepared.state = OD_DMA_LEASE_PREPARED;
    prepared.offset = offset;
    prepared.requested_length = requested_length;
    prepared.memory_handle = memory_handle;
    prepared.command_handle = command_handle;
    prepared.backend = *backend;
    prepared.segment_count = OD_DMA_MAX_SEGMENTS;
    const int prepare_result = backend->prepare(backend->context, command_handle,
                                                memory_handle, offset, requested_length,
                                                prepared.segments, &prepared.segment_count,
                                                &prepared.device_access);
    if (prepare_result < 0) {
        backend->release_command(backend->context, command_handle);
        backend->release_memory(backend->context, memory_handle);
        return OD_DMA_STATUS_PREPARE_FAILED;
    }
    if (prepare_result > 0) {
        prepared.mapping_valid = 0;
        *lease = prepared;
        return OD_DMA_STATUS_PREPARE_FAILED;
    }

    const ODDmaStatus mapping = od_dma_validate_mapping(prepared.segments,
                                                         prepared.segment_count,
                                                         requested_length,
                                                         prepared.device_access);
    prepared.mapping_valid = mapping == OD_DMA_STATUS_OK;
    *lease = prepared;
    return mapping == OD_DMA_STATUS_OK ? OD_DMA_STATUS_OK : OD_DMA_STATUS_MAPPING_INVALID;
}

ODDmaStatus od_dma_lease_publish(ODDmaLease *lease) {
    if (!lease || lease->magic != OD_DMA_LEASE_MAGIC ||
        lease->state != OD_DMA_LEASE_PREPARED || !lease->mapping_valid)
        return OD_DMA_STATUS_BAD_STATE;
    lease->state = OD_DMA_LEASE_PUBLISHED;
    return OD_DMA_STATUS_OK;
}

static ODDmaStatus complete_live_lease(ODDmaLease *lease) {
    if (lease->backend.complete(lease->backend.context, lease->command_handle))
        return OD_DMA_STATUS_COMPLETE_FAILED;
    release_completed_resources(lease);
    return OD_DMA_STATUS_OK;
}

ODDmaStatus od_dma_lease_complete(ODDmaLease *lease) {
    if (!lease || lease->magic != OD_DMA_LEASE_MAGIC ||
        (lease->state != OD_DMA_LEASE_PREPARED && lease->state != OD_DMA_LEASE_PUBLISHED))
        return OD_DMA_STATUS_BAD_STATE;
    return complete_live_lease(lease);
}

ODDmaStatus od_dma_lease_abort_unpublished(ODDmaLease *lease) {
    if (!lease || lease->magic != OD_DMA_LEASE_MAGIC || lease->state != OD_DMA_LEASE_PREPARED)
        return OD_DMA_STATUS_BAD_STATE;
    return complete_live_lease(lease);
}

ODDmaStatus od_dma_lease_quarantine_published(ODDmaLease *lease) {
    if (!lease || lease->magic != OD_DMA_LEASE_MAGIC || lease->state != OD_DMA_LEASE_PUBLISHED)
        return OD_DMA_STATUS_BAD_STATE;
    lease->state = OD_DMA_LEASE_QUARANTINED;
    return OD_DMA_STATUS_OK;
}

ODDmaStatus od_dma_lease_release_after_bus_master_quiescence(ODDmaLease *lease) {
    if (!lease || lease->magic != OD_DMA_LEASE_MAGIC || lease->state != OD_DMA_LEASE_QUARANTINED)
        return OD_DMA_STATUS_BAD_STATE;
    return complete_live_lease(lease);
}

ODDmaStatus od_dma_lease_release(ODDmaLease *lease) {
    if (!lease || lease->magic != OD_DMA_LEASE_MAGIC || lease->state != OD_DMA_LEASE_COMPLETED)
        return OD_DMA_STATUS_BAD_STATE;
    lease->state = OD_DMA_LEASE_EMPTY;
    return OD_DMA_STATUS_OK;
}
