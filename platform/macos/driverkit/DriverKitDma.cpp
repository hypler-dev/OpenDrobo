#include "DriverKitDma.h"

#include <DriverKit/IOBufferMemoryDescriptor.h>
#include <DriverKit/IODMACommand.h>
#include <DriverKit/IOMemoryDescriptor.h>
#include <PCIDriverKit/IOPCIDevice.h>
#include <string.h>

static int driverkit_retain_memory(void *, void *memory_handle) {
    static_cast<IOMemoryDescriptor *>(memory_handle)->retain();
    return 0;
}

static void driverkit_release_memory(void *, void *memory_handle) {
    static_cast<IOMemoryDescriptor *>(memory_handle)->release();
}

static int driverkit_create_command(void *context, uint32_t max_address_bits,
                                    void **command_handle) {
    if (!context || !command_handle || max_address_bits != 32U) return -1;
    IOPCIDevice *provider = static_cast<IOPCIDevice *>(context);
    IODMACommandSpecification specification = {};
    specification.maxAddressBits = max_address_bits;
    IODMACommand *command = nullptr;
    provider->retain();
    const kern_return_t result = IODMACommand::Create(
        provider, kIODMACommandCreateNoOptions,
        &specification, &command);
    if (result != kIOReturnSuccess || !command) {
        provider->release();
        return result != kIOReturnSuccess ? (int)result : -1;
    }
    *command_handle = command;
    return 0;
}

static void driverkit_release_command(void *context, void *command_handle) {
    static_cast<IODMACommand *>(command_handle)->release();
    static_cast<IOPCIDevice *>(context)->release();
}

static int driverkit_prepare(void *, void *command_handle, void *memory_handle,
                             uint64_t offset, uint64_t length,
                             ODDmaSegment *segments, uint32_t *segment_count,
                             uint32_t *device_access) {
    if (!command_handle || !memory_handle || !segments || !segment_count ||
        !device_access || !*segment_count || *segment_count > OD_DMA_MAX_SEGMENTS)
        return -1;
    IOAddressSegment sdk_segments[OD_DMA_MAX_SEGMENTS] = {};
    uint64_t flags = 0;
    uint32_t count = *segment_count;
    const kern_return_t result = static_cast<IODMACommand *>(command_handle)->PrepareForDMA(
        kIODMACommandPrepareForDMANoOptions,
        static_cast<IOMemoryDescriptor *>(memory_handle), offset, length,
        &flags, &count, sdk_segments);
    if (result != kIOReturnSuccess) return 1;
    if (count > OD_DMA_MAX_SEGMENTS) {
        *segment_count = count;
        *device_access = 0;
        return 0;
    }
    for (uint32_t i = 0; i < count; ++i) {
        segments[i].address = sdk_segments[i].address;
        segments[i].length = sdk_segments[i].length;
    }
    *segment_count = count;
    *device_access = 0;
    if (flags & kIOMemoryDirectionOut) *device_access |= OD_DMA_DEVICE_READ;
    if (flags & kIOMemoryDirectionIn) *device_access |= OD_DMA_DEVICE_WRITE;
    return 0;
}

static int driverkit_complete(void *, void *command_handle) {
    if (!command_handle) return -1;
    return (int)static_cast<IODMACommand *>(command_handle)->CompleteDMA(
        kIODMACommandCompleteDMANoOptions);
}

int od_driverkit_dma_write_request(ODDmaLease *lease, const void *bytes,
                                   uint32_t length) {
    if (!lease || lease->magic != OD_DMA_LEASE_MAGIC ||
        lease->state != OD_DMA_LEASE_PREPARED || !lease->mapping_valid ||
        !lease->memory_handle || !lease->command_handle || !bytes || !length ||
        length != lease->requested_length ||
        !(lease->device_access & OD_DMA_DEVICE_READ) ||
        lease->backend.prepare != driverkit_prepare ||
        lease->backend.complete != driverkit_complete)
        return -1;

    IOBufferMemoryDescriptor *source = nullptr;
    kern_return_t result = IOBufferMemoryDescriptor::Create(
        kIOMemoryDirectionOut, length, 0, &source);
    if (result != kIOReturnSuccess || !source)
        return result != kIOReturnSuccess ? (int)result : -1;
    result = source->SetLength(length);
    if (result != kIOReturnSuccess) {
        source->release();
        return (int)result;
    }
    IOAddressSegment source_range = {};
    result = source->GetAddressRange(&source_range);
    if (result != kIOReturnSuccess || !source_range.address || source_range.length < length) {
        source->release();
        return result != kIOReturnSuccess ? (int)result : -1;
    }
    memcpy((void *)(uintptr_t)source_range.address, bytes, length);
    result = static_cast<IODMACommand *>(lease->command_handle)->PerformOperation(
        kIODMACommandPerformOperationOptionWrite, 0, length, 0, source);
    source->release();
    return (int)result;
}

ODDmaBackend od_driverkit_dma_backend(IOPCIDevice *provider) {
    ODDmaBackend backend = {};
    backend.context = provider;
    backend.retain_memory = driverkit_retain_memory;
    backend.release_memory = driverkit_release_memory;
    backend.create_command = driverkit_create_command;
    backend.release_command = driverkit_release_command;
    backend.prepare = driverkit_prepare;
    backend.complete = driverkit_complete;
    return backend;
}
