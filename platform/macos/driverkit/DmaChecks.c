#include "DmaOwnership.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

typedef struct {
    int retain_calls;
    int release_memory_calls;
    int create_calls;
    int release_command_calls;
    int prepare_calls;
    int complete_calls;
    int retain_error;
    int create_error;
    int prepare_error;
    int complete_error_count;
    uint32_t map_count;
    uint32_t returned_count;
    uint32_t device_access;
    ODDmaSegment map[OD_DMA_MAX_SEGMENTS];
} FakeDma;

static int fake_memory;
static int fake_command;

static int retain_memory(void *context, void *memory) {
    FakeDma *fake = context;
    assert(memory == &fake_memory);
    ++fake->retain_calls;
    return fake->retain_error;
}

static void release_memory(void *context, void *memory) {
    FakeDma *fake = context;
    assert(memory == &fake_memory);
    ++fake->release_memory_calls;
}

static int create_command(void *context, uint32_t bits, void **command) {
    FakeDma *fake = context;
    assert(bits == 32);
    ++fake->create_calls;
    if (fake->create_error) return fake->create_error;
    *command = &fake_command;
    return 0;
}

static void release_command(void *context, void *command) {
    FakeDma *fake = context;
    assert(command == &fake_command);
    ++fake->release_command_calls;
}

static int prepare(void *context, void *command, void *memory, uint64_t offset,
                   uint64_t length, ODDmaSegment *segments, uint32_t *count,
                   uint32_t *device_access) {
    FakeDma *fake = context;
    assert(command == &fake_command && memory == &fake_memory);
    assert(offset == 64 && length == 1024 && *count == OD_DMA_MAX_SEGMENTS);
    ++fake->prepare_calls;
    if (fake->prepare_error) return fake->prepare_error;
    assert(fake->map_count <= OD_DMA_MAX_SEGMENTS);
    memcpy(segments, fake->map, fake->map_count * sizeof(*segments));
    *count = fake->returned_count;
    *device_access = fake->device_access;
    return 0;
}

static int complete(void *context, void *command) {
    FakeDma *fake = context;
    assert(command == &fake_command);
    ++fake->complete_calls;
    if (fake->complete_error_count) {
        --fake->complete_error_count;
        return 1;
    }
    return 0;
}

static ODDmaBackend backend(FakeDma *fake) {
    ODDmaBackend result = {0};
    result.context = fake;
    result.retain_memory = retain_memory;
    result.release_memory = release_memory;
    result.create_command = create_command;
    result.release_command = release_command;
    result.prepare = prepare;
    result.complete = complete;
    return result;
}

static FakeDma valid_fake(void) {
    FakeDma fake = {0};
    fake.map_count = 2;
    fake.returned_count = 2;
    fake.device_access = OD_DMA_DEVICE_READ | OD_DMA_DEVICE_WRITE;
    fake.map[0] = (ODDmaSegment){0x1000, 512};
    fake.map[1] = (ODDmaSegment){0x3000, 512};
    return fake;
}

static ODDmaLease prepared(FakeDma *fake) {
    ODDmaLease lease = OD_DMA_LEASE_INITIALIZER;
    assert(od_dma_lease_init(&lease) == OD_DMA_STATUS_OK);
    const ODDmaBackend ops = backend(fake);
    assert(od_dma_lease_prepare(&lease, &ops, &fake_memory, 64, 1024) == OD_DMA_STATUS_OK);
    assert(lease.state == OD_DMA_LEASE_PREPARED && lease.segment_count == 2);
    assert(lease.device_access == (OD_DMA_DEVICE_READ | OD_DMA_DEVICE_WRITE));
    return lease;
}

static void test_prepare_failure_releases_retained_memory(void) {
    FakeDma fake = valid_fake();
    fake.prepare_error = -1;
    ODDmaLease lease = OD_DMA_LEASE_INITIALIZER;
    const ODDmaBackend ops = backend(&fake);
    assert(od_dma_lease_prepare(&lease, &ops, &fake_memory, 64, 1024) == OD_DMA_STATUS_PREPARE_FAILED);
    assert(lease.state == OD_DMA_LEASE_EMPTY);
    assert(fake.retain_calls == 1 && fake.create_calls == 1 && fake.prepare_calls == 1);
    assert(fake.release_command_calls == 1 && fake.release_memory_calls == 1);
}

static void test_invalid_mappings_stay_unpublished_until_abort(void) {
    ODDmaSegment segment = {0x1000, 1024};
    assert(od_dma_validate_mapping(&segment, 1, 1024, OD_DMA_DEVICE_READ) == OD_DMA_STATUS_OK);
    segment.length = 0;
    assert(od_dma_validate_mapping(&segment, 1, 1024, OD_DMA_DEVICE_READ) == OD_DMA_STATUS_RANGE);
    segment = (ODDmaSegment){UINT32_MAX - 128, 256};
    assert(od_dma_validate_mapping(&segment, 1, 256, OD_DMA_DEVICE_READ) == OD_DMA_STATUS_RANGE);
    segment = (ODDmaSegment){0x1000, OD_DMA_MAX_SEGMENT_BYTES + 1U};
    assert(od_dma_validate_mapping(&segment, 1, segment.length, OD_DMA_DEVICE_READ) == OD_DMA_STATUS_RANGE);
    ODDmaSegment too_many[OD_DMA_MAX_SEGMENTS + 1] = {{0}};
    assert(od_dma_validate_mapping(too_many, OD_DMA_MAX_SEGMENTS + 1, 1024, OD_DMA_DEVICE_READ) == OD_DMA_STATUS_RANGE);
    segment = (ODDmaSegment){0x1000, 1024};
    assert(od_dma_validate_mapping(&segment, 1, 1024, 0) == OD_DMA_STATUS_RANGE);
    assert(od_dma_validate_mapping(&segment, 1, 1024, 4) == OD_DMA_STATUS_RANGE);

    FakeDma fake = valid_fake();
    fake.map_count = 1;
    fake.returned_count = 1;
    fake.map[0] = (ODDmaSegment){0x1000, 512};
    ODDmaLease lease = OD_DMA_LEASE_INITIALIZER;
    const ODDmaBackend ops = backend(&fake);
    assert(od_dma_lease_prepare(&lease, &ops, &fake_memory, 64, 1024) == OD_DMA_STATUS_MAPPING_INVALID);
    assert(lease.state == OD_DMA_LEASE_PREPARED && !lease.mapping_valid);
    assert(od_dma_lease_publish(&lease) == OD_DMA_STATUS_BAD_STATE);
    assert(od_dma_lease_abort_unpublished(&lease) == OD_DMA_STATUS_OK);
    assert(fake.complete_calls == 1 && fake.release_command_calls == 1 && fake.release_memory_calls == 1);
}

static void test_uncertain_prepare_failure_and_returned_count_keep_ownership(void) {
    FakeDma uncertain = valid_fake();
    uncertain.prepare_error = 1;
    ODDmaLease lease = OD_DMA_LEASE_INITIALIZER;
    const ODDmaBackend uncertain_ops = backend(&uncertain);
    assert(od_dma_lease_prepare(&lease, &uncertain_ops, &fake_memory, 64, 1024) == OD_DMA_STATUS_PREPARE_FAILED);
    assert(lease.state == OD_DMA_LEASE_PREPARED && !lease.mapping_valid);
    assert(uncertain.release_command_calls == 0 && uncertain.release_memory_calls == 0);
    uncertain.complete_error_count = 1;
    assert(od_dma_lease_abort_unpublished(&lease) == OD_DMA_STATUS_COMPLETE_FAILED);
    assert(lease.state == OD_DMA_LEASE_PREPARED);
    assert(od_dma_lease_abort_unpublished(&lease) == OD_DMA_STATUS_OK);
    assert(uncertain.complete_calls == 2 && uncertain.release_command_calls == 1 &&
           uncertain.release_memory_calls == 1);

    FakeDma oversized = valid_fake();
    oversized.map_count = 0;
    oversized.returned_count = OD_DMA_MAX_SEGMENTS + 1;
    lease = (ODDmaLease)OD_DMA_LEASE_INITIALIZER;
    const ODDmaBackend oversized_ops = backend(&oversized);
    assert(od_dma_lease_prepare(&lease, &oversized_ops, &fake_memory, 64, 1024) == OD_DMA_STATUS_MAPPING_INVALID);
    assert(lease.state == OD_DMA_LEASE_PREPARED && lease.segment_count == OD_DMA_MAX_SEGMENTS + 1 &&
           !lease.mapping_valid);
    assert(od_dma_lease_abort_unpublished(&lease) == OD_DMA_STATUS_OK);
}

static void test_complete_failure_retries_without_releasing_live_command(void) {
    FakeDma fake = valid_fake();
    ODDmaLease lease = prepared(&fake);
    assert(od_dma_lease_publish(&lease) == OD_DMA_STATUS_OK);
    fake.complete_error_count = 1;
    assert(od_dma_lease_complete(&lease) == OD_DMA_STATUS_COMPLETE_FAILED);
    assert(lease.state == OD_DMA_LEASE_PUBLISHED);
    assert(fake.release_command_calls == 0 && fake.release_memory_calls == 0);
    assert(od_dma_lease_complete(&lease) == OD_DMA_STATUS_OK);
    assert(lease.state == OD_DMA_LEASE_COMPLETED && fake.complete_calls == 2);
    assert(fake.release_command_calls == 1 && fake.release_memory_calls == 1);
}

static void test_double_completion_and_release_are_refused(void) {
    FakeDma fake = valid_fake();
    ODDmaLease lease = prepared(&fake);
    assert(od_dma_lease_complete(&lease) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_complete(&lease) == OD_DMA_STATUS_BAD_STATE);
    assert(od_dma_lease_release(&lease) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_release(&lease) == OD_DMA_STATUS_BAD_STATE);
    assert(fake.release_command_calls == 1 && fake.release_memory_calls == 1);
}

static void test_published_timeout_needs_explicit_quiescence(void) {
    FakeDma fake = valid_fake();
    ODDmaLease lease = prepared(&fake);
    assert(od_dma_lease_publish(&lease) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_quarantine_published(&lease) == OD_DMA_STATUS_OK);
    assert(lease.state == OD_DMA_LEASE_QUARANTINED);
    assert(od_dma_lease_release(&lease) == OD_DMA_STATUS_BAD_STATE);
    assert(fake.complete_calls == 0 && fake.release_command_calls == 0 && fake.release_memory_calls == 0);
    assert(od_dma_lease_release_after_bus_master_quiescence(&lease) == OD_DMA_STATUS_OK);
    assert(lease.state == OD_DMA_LEASE_COMPLETED && fake.complete_calls == 1);
    assert(fake.release_command_calls == 1 && fake.release_memory_calls == 1);
}

int main(void) {
    test_prepare_failure_releases_retained_memory();
    test_invalid_mappings_stay_unpublished_until_abort();
    test_uncertain_prepare_failure_and_returned_count_keep_ownership();
    test_complete_failure_retries_without_releasing_live_command();
    test_double_completion_and_release_are_refused();
    test_published_timeout_needs_explicit_quiescence();
    puts("PASS: DMA lease ownership, mapping limits, retry, and quarantine");
}
