#include "ControllerTransport.h"
#include "ScsiDrain.h"
#include "ScsiTaskTable.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

/* Explicitly synthetic registers and IOVA mappings. No device is opened. */
enum { BAR0_BYTES = 0x22000, BAR1_BYTES = 0x2000,
       REQUEST = 0xc00, RESPONSE = 0x1000, REQ_IRQ = 0x100,
       RSP_IRQ = 0x200, SUMMARY_IRQ = 0x300,
       STORAGE_REQUEST = 0xe00, STORAGE_RESPONSE = 0x1400,
       STORAGE_REQ_IRQ = 0x400, STORAGE_RSP_IRQ = 0x500,
       TRACE_CAPACITY = 2048 };
typedef struct { uint8_t kind, bar; uint32_t offset, value; } Event;
typedef struct { uint64_t address, bytes; unsigned retains, prepared, access; uint8_t cpu[512]; } Buffer;
typedef struct { Buffer *buffer; unsigned prepared; } Command;
typedef struct {
    uint32_t bar0[BAR0_BYTES / 4], bar1[BAR1_BYTES / 4];
    ODTBTController controller;
    ODDmaLease request, payload;
    Buffer request_buffer, payload_buffer;
    Command commands[64];
    unsigned created, released, completed, failed_complete;
    unsigned operation, fail_operation, removed_operation, trace_count;
    Event trace[TRACE_CAPACITY];
} Fixture;

static int event(Fixture *f, uint8_t kind, uint8_t bar, uint32_t offset, uint32_t value) {
    assert(f->trace_count < TRACE_CAPACITY);
    f->trace[f->trace_count++] = (Event){kind, bar, offset, value};
    return ++f->operation == f->fail_operation;
}
static int read32(void *context, uint8_t bar, uint32_t offset, uint32_t *value) {
    Fixture *f = context;
    assert(bar < 2 && !(offset & 3) && offset < (bar ? BAR1_BYTES : BAR0_BYTES));
    if (event(f, 'r', bar, offset, 0)) return -1;
    *value = f->operation == f->removed_operation ? UINT32_MAX : (bar ? f->bar1 : f->bar0)[offset / 4];
    return 0;
}
static int write32(void *context, uint8_t bar, uint32_t offset, uint32_t value) {
    Fixture *f = context;
    assert(bar < 2 && !(offset & 3) && offset < (bar ? BAR1_BYTES : BAR0_BYTES));
    /* Model a posted write that reached the device despite its reported error. */
    (bar ? f->bar1 : f->bar0)[offset / 4] = value;
    return event(f, 'w', bar, offset, value) ? -1 : 0;
}
static int barrier(void *context) { return event(context, 'b', 0, 0, 0) ? -1 : 0; }
static int retain_memory(void *context, void *memory) {
    (void)context; ++((Buffer *)memory)->retains; return 0;
}
static void release_memory(void *context, void *memory) {
    (void)context; Buffer *b = memory; assert(b->retains && !b->prepared); --b->retains;
}
static int create_command(void *context, uint32_t bits, void **command) {
    Fixture *f = context; assert(bits == 32 && f->created < 64);
    *command = &f->commands[f->created++]; return 0;
}
static void release_command(void *context, void *command) {
    Fixture *f = context; assert(!((Command *)command)->prepared); ++f->released;
}
static int prepare(void *context, void *command, void *memory, uint64_t offset,
                   uint64_t length, ODDmaSegment *segments, uint32_t *count, uint32_t *access) {
    (void)context; Command *c = command; Buffer *b = memory;
    assert(offset == 0 && length == b->bytes && *count == 32 && !b->prepared);
    c->buffer = b; c->prepared = b->prepared = 1;
    segments[0] = (ODDmaSegment){b->address, length}; *count = 1; *access = b->access; return 0;
}
static int complete(void *context, void *command) {
    Fixture *f = context; Command *c = command; assert(c->prepared && c->buffer->prepared);
    if (f->failed_complete) { --f->failed_complete; return -1; }
    ++f->completed; c->prepared = c->buffer->prepared = 0; return 0;
}
static int write_request(void *context, ODDmaLease *lease, const void *bytes, uint32_t length) {
    Fixture *f = context; Command *command = lease->command_handle;
    assert(lease->state == OD_DMA_LEASE_PREPARED && command->prepared &&
           command->buffer == lease->memory_handle && length == command->buffer->bytes);
    memcpy(command->buffer->cpu, bytes, length);
    return event(f, 'm', 0, 0, length) ? -1 : 0;
}
static void put32(uint32_t *bar, uint32_t offset, uint32_t value) { bar[offset / 4] = value; }
static void definition(Fixture *f, uint32_t where, uint32_t address) {
    /* Trigger OR 1; state AND 1; enable OR 1; disable AND ~1; clear AND ~1. */
    const uint32_t words[21] = {
        0, address, 4, 1, 1, address, 4, 1, 0,
        address + 4, 4, 1, 1, address + 4, 4, UINT32_MAX - 1, 0,
        address, 4, UINT32_MAX - 1, 0
    };
    memcpy((uint8_t *)f->bar1 + where, words, sizeof(words));
}
static void fixture_channel(Fixture *f, int start, ODTBTChannel channel, int external_summary) {
    memset(f, 0, sizeof(*f));
    memcpy(f->bar1, "DroboPort\0\xab\xcd", 12);
    put32(f->bar1, 0x18, 0x100); put32(f->bar1, 0x1c, 0x80);
    put32(f->bar1, 0x30, REQUEST); put32(f->bar1, 0x34, RESPONSE);
    put32(f->bar1, 0x20, STORAGE_REQUEST); put32(f->bar1, 0x24, STORAGE_RESPONSE);
    put32(f->bar1, 0x38, 0x400); put32(f->bar1, 0x3c, 0x800);
    put32(f->bar1, 0x80, 4); put32(f->bar1, 0x88, 1024 * 1024); put32(f->bar1, 0x8c, 4);
    put32(f->bar1, 0x90, 32);
    put32(f->bar1, 0x100, 1); put32(f->bar1, 0x104, 512);
    put32(f->bar1, 0x108, 4096); put32(f->bar1, 0x10c, 0);
    put32(f->bar1, REQUEST, 4);
    put32(f->bar1, STORAGE_REQUEST, 4);
    definition(f, 0x400 + 36 + 4 * 84, REQ_IRQ);
    definition(f, 0x800 + 36 + 84, RSP_IRQ);
    definition(f, 0x400 + 36, STORAGE_REQ_IRQ);
    definition(f, 0x800 + 36, STORAGE_RSP_IRQ);
    definition(f, 0x800 + 36 + 5 * 84, SUMMARY_IRQ);
    ODTBTControllerIO io = {f, read32, write32, barrier, write_request};
    assert(od_tbt_controller_init_channel(&f->controller, &io, BAR0_BYTES, BAR1_BYTES,
                                          channel, external_summary) == OD_TBT_CTL_OK);
    assert(od_dma_lease_init(&f->request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_init(&f->payload) == OD_DMA_STATUS_OK);
    f->request_buffer.address = 0x100000; f->request_buffer.bytes = OD_TBT_TM_REQUEST_BYTES;
    f->payload_buffer.address = 0x200000; f->payload_buffer.bytes = 512;
    f->request_buffer.access = f->payload_buffer.access = OD_DMA_DEVICE_READ | OD_DMA_DEVICE_WRITE;
    if (start) assert(od_tbt_controller_start(&f->controller) == OD_TBT_CTL_OK);
}
static void fixture(Fixture *f, int start) {
    fixture_channel(f, start, OD_TBT_CHANNEL_MANAGEMENT, 0);
}
static void map_buffers(Fixture *f) {
    ODDmaBackend backend = {f, retain_memory, release_memory, create_command, release_command,
                            prepare, complete};
    assert(od_dma_lease_prepare(&f->request, &backend, &f->request_buffer, 0,
                               OD_TBT_TM_REQUEST_BYTES) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_prepare(&f->payload, &backend, &f->payload_buffer, 0, 512) == OD_DMA_STATUS_OK);
}
static void map_buffers_exact(Fixture *f, uint32_t request_bytes, uint32_t payload_bytes) {
    ODDmaBackend backend = {f, retain_memory, release_memory, create_command, release_command,
                            prepare, complete};
    f->request_buffer.bytes = request_bytes; f->payload_buffer.bytes = payload_bytes;
    assert(od_dma_lease_prepare(&f->request, &backend, &f->request_buffer, 0,
                               request_bytes) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_prepare(&f->payload, &backend, &f->payload_buffer, 0,
                               payload_bytes) == OD_DMA_STATUS_OK);
}
static ODTBTTaskInput task(void) {
    ODTBTTaskInput t = {0};
    t.version = 1; t.direction = 2; t.requested_bytes = 512;
    t.buffer_iovm_address = UINT64_MAX; // Must be replaced with the actual lease IOVA.
    t.controller_task_id = UINT64_C(0x1122334455667788); t.cdb_length = 10;
    const uint8_t cdb[10] = {0x5a, 8, 0x3a, 2, 0, 0, 0, 2, 0, 0};
    memcpy(t.cdb, cdb, sizeof(cdb)); return t;
}
static ODTBTControllerStatus submit(Fixture *f, uint32_t *tx) {
    ODTBTTaskInput t = task();
    return od_tbt_controller_submit(&f->controller, &t, &f->request, &f->payload, 10, 100, tx);
}
static void response(Fixture *f, uint32_t tx, uint32_t srb, uint32_t scsi) {
    put32(f->bar0, REQ_IRQ, 0); put32(f->bar0, RSP_IRQ, 1); put32(f->bar0, SUMMARY_IRQ, 1);
    put32(f->bar1, RESPONSE + 4, 1);
    put32(f->bar1, RESPONSE + 8, 0); put32(f->bar1, RESPONSE + 12, tx);
    put32(f->bar1, RESPONSE + 16, 512); put32(f->bar1, RESPONSE + 20, 0);
    put32(f->bar1, RESPONSE + 28, srb); put32(f->bar1, RESPONSE + 32, scsi);
}
static void storage_response(Fixture *f, uint32_t tx, uint32_t bytes) {
    put32(f->bar0, STORAGE_REQ_IRQ, 0); put32(f->bar0, STORAGE_RSP_IRQ, 1); put32(f->bar0, SUMMARY_IRQ, 1);
    put32(f->bar1, STORAGE_RESPONSE + 4, 1);
    put32(f->bar1, STORAGE_RESPONSE + 8, 0); put32(f->bar1, STORAGE_RESPONSE + 12, tx);
    put32(f->bar1, STORAGE_RESPONSE + 16, bytes); put32(f->bar1, STORAGE_RESPONSE + 20, 0);
    put32(f->bar1, STORAGE_RESPONSE + 28, 1); put32(f->bar1, STORAGE_RESPONSE + 32, 0);
}
static ODTBTTaskInput read10(uint32_t lba, uint16_t blocks) {
    ODTBTTaskInput t = {0};
    t.version = 1; t.direction = 2; t.requested_bytes = (uint64_t)blocks * 512;
    t.buffer_iovm_address = UINT64_MAX; t.controller_task_id = UINT64_C(0x1122334455667788);
    t.cdb_length = 10; t.cdb[0] = 0x28;
    t.cdb[2] = (uint8_t)(lba >> 24); t.cdb[3] = (uint8_t)(lba >> 16);
    t.cdb[4] = (uint8_t)(lba >> 8); t.cdb[5] = (uint8_t)lba;
    t.cdb[7] = (uint8_t)(blocks >> 8); t.cdb[8] = (uint8_t)blocks;
    return t;
}
static ODTBTTaskInput read16(uint64_t lba, uint32_t blocks) {
    ODTBTTaskInput t = {0};
    t.version = 1; t.direction = 2; t.requested_bytes = (uint64_t)blocks * 512;
    t.buffer_iovm_address = UINT64_MAX; t.controller_task_id = UINT64_C(0x8877665544332211);
    t.cdb_length = 16; t.cdb[0] = 0x88;
    for (unsigned i = 0; i < 8; ++i) t.cdb[2 + i] = (uint8_t)(lba >> ((7 - i) * 8));
    for (unsigned i = 0; i < 4; ++i) t.cdb[10 + i] = (uint8_t)(blocks >> ((3 - i) * 8));
    return t;
}
static unsigned writes_at(const Fixture *f, uint8_t bar, uint32_t offset) {
    unsigned count = 0;
    for (unsigned i = 0; i < f->trace_count; ++i)
        if (f->trace[i].kind == 'w' && f->trace[i].bar == bar && f->trace[i].offset == offset) ++count;
    return count;
}
static size_t finish(Fixture *f, int drained, int quiescent, ODTBTControllerStatus expected) {
    ODTBTControllerCompletion results[OD_TBT_CONTROLLER_SLOTS]; size_t count = 999;
    assert(od_tbt_controller_finish_stop(&f->controller, drained, quiescent, results,
                                        OD_TBT_CONTROLLER_SLOTS, &count) == expected);
    if (expected != OD_TBT_CTL_OK) assert(!count);
    if (count) assert(results[0].cancelled && !results[0].succeeded);
    return count;
}
static void success_and_wire(void) {
    Fixture f; fixture(&f, 1); map_buffers(&f);
    assert(f.bar1[0xa4 / 4] == 1 && f.bar0[(SUMMARY_IRQ + 4) / 4] == 1);
    unsigned begin = f.trace_count; uint32_t tx = 0;
    assert(submit(&f, &tx) == OD_TBT_CTL_OK && tx == 1);
    const uint8_t expected[104] = {
        [2] = 0x10, [12] = 1, [16] = 0x88, [17] = 0x77, [18] = 0x66,
        [19] = 0x55, [20] = 0x44, [21] = 0x33, [22] = 0x22, [23] = 0x11,
        [24] = 0x68, [28] = 3, [32] = 0x28, [34] = 0x41, [37] = 2, [42] = 0x20,
        [48] = 0x5a, [49] = 8, [50] = 0x3a, [51] = 2, [55] = 2, [90] = 0x20
    };
    assert(!memcmp(f.request_buffer.cpu, expected, sizeof(expected)));
    assert(f.request.state == OD_DMA_LEASE_PUBLISHED && f.payload.state == OD_DMA_LEASE_PUBLISHED);
    unsigned count_write = 0, trigger_write = 0;
    for (unsigned i = begin; i < f.trace_count; ++i) {
        Event e = f.trace[i];
        if (e.kind == 'w' && e.bar == 1 && e.offset == REQUEST + 4) count_write = i;
        if (e.kind == 'w' && e.bar == 0 && e.offset == REQ_IRQ) trigger_write = i;
    }
    assert(count_write > begin && f.trace[count_write - 1].kind == 'b');
    assert(trigger_write > count_write + 1 && f.trace[count_write + 1].kind == 'b');
    assert(f.bar1[(REQUEST + 8) / 4] == 0x100000 && f.bar1[(REQUEST + 16) / 4] == 104);
    response(&f, tx, 1, 0);
    ODTBTControllerCompletion results[32]; size_t count;
    assert(od_tbt_controller_poll(&f.controller, 20, results, 32, &count) == OD_TBT_CTL_OK);
    assert(count == 1 && results[0].succeeded && results[0].realized_bytes == 512 &&
           results[0].task_id == task().controller_task_id && f.completed == 2 && f.released == 2);
    assert(!f.request_buffer.retains && !f.payload_buffer.retains);
    assert(od_tbt_controller_poll(&f.controller, 21, results, 32, &count) == OD_TBT_CTL_OK && !count);
    assert(od_tbt_controller_begin_stop(&f.controller, 0) == OD_TBT_CTL_OK);
    assert(finish(&f, 0, 0, OD_TBT_CTL_NOT_READY) == 0);
    assert(finish(&f, 1, 0, OD_TBT_CTL_OK) == 0);
    assert(od_tbt_controller_start(&f.controller) == OD_TBT_CTL_STATE);
}
static void legacy_management_init_stays_owned(void) {
    Fixture f; fixture_channel(&f, 0, OD_TBT_CHANNEL_MANAGEMENT, 0);
    memset(&f.controller, 0, sizeof(f.controller));
    ODTBTControllerIO io = {&f, read32, write32, barrier, write_request};
    assert(od_tbt_controller_init(&f.controller, &io, BAR0_BYTES, BAR1_BYTES) == OD_TBT_CTL_OK);
    assert(od_tbt_controller_start(&f.controller) == OD_TBT_CTL_OK);
    assert(f.controller.channel == OD_TBT_CHANNEL_MANAGEMENT && !f.controller.external_summary &&
           f.bar1[0xa4 / 4] == 1 && f.bar0[(SUMMARY_IRQ + 4) / 4] == 1);
    map_buffers(&f); uint32_t tx;
    assert(submit(&f, &tx) == OD_TBT_CTL_OK && f.bar1[(REQUEST + 16) / 4] == OD_TBT_TM_REQUEST_BYTES &&
           f.bar1[(STORAGE_REQUEST + 16) / 4] == 0 && f.bar0[REQ_IRQ / 4] && !f.bar0[STORAGE_REQ_IRQ / 4]);
}
static void publication_failure_matrix(void) {
    Fixture f; fixture(&f, 1); map_buffers(&f);
    unsigned initial = f.operation; uint32_t tx;
    assert(submit(&f, &tx) == OD_TBT_CTL_OK);
    unsigned operations = f.operation - initial;
    for (unsigned fail = 1; fail <= operations; ++fail) {
        fixture(&f, 1); map_buffers(&f); f.fail_operation = f.operation + fail; tx = 999;
        ODTBTControllerStatus status = submit(&f, &tx);
        assert((status == OD_TBT_CTL_IO || status == OD_TBT_CTL_DMA) && tx == 999);
        assert(!f.completed && !f.released && f.controller.state == OD_TBT_CTL_QUIESCING);
        if (f.request.state == OD_DMA_LEASE_QUARANTINED) {
            assert(f.payload.state == OD_DMA_LEASE_QUARANTINED);
            assert(!finish(&f, 1, 0, OD_TBT_CTL_QUARANTINED));
            assert(finish(&f, 1, 1, OD_TBT_CTL_OK) == 1);
        } else {
            assert(f.request.state == OD_DMA_LEASE_PREPARED);
            assert(!finish(&f, 1, 0, OD_TBT_CTL_OK));
            assert(od_dma_lease_abort_unpublished(&f.request) == OD_DMA_STATUS_OK);
            assert(od_dma_lease_abort_unpublished(&f.payload) == OD_DMA_STATUS_OK);
        }
        assert(f.completed == 2 && f.released == 2);
    }
    printf("PASS: %u submission failure points preserve DMA ownership\n", operations);
}
static void malformed_and_status_completions(void) {
    for (unsigned mode = 0; mode < 7; ++mode) {
        Fixture f; fixture(&f, 1); map_buffers(&f); uint32_t tx;
        assert(submit(&f, &tx) == OD_TBT_CTL_OK); response(&f, tx, 1, 0);
        switch (mode) {
            case 0: put32(f.bar1, RESPONSE + 12, tx + 1); break;
            case 1: put32(f.bar1, RESPONSE + 8, 4); break;
            case 2: put32(f.bar1, RESPONSE + 16, 513); break;
            case 3:
                put32(f.bar1, RESPONSE + 4, 2);
                memcpy((uint8_t *)f.bar1 + RESPONSE + 40, (uint8_t *)f.bar1 + RESPONSE + 8, 32);
                break;
            case 4: put32(f.bar1, RESPONSE + 4, UINT32_MAX); break;
            case 5: put32(f.bar1, RESPONSE + 32, 2); break;
            case 6: put32(f.bar1, RESPONSE + 28, 0x12); break;
        }
        ODTBTControllerCompletion results[32]; size_t count = 1;
        ODTBTControllerStatus status = od_tbt_controller_poll(&f.controller, 20, results, 32, &count);
        if (mode < 5) {
            assert(status == OD_TBT_CTL_FORMAT && count == 0 && !f.completed);
            assert(f.bar0[RSP_IRQ / 4] == 1); // Bad batches are never acknowledged.
            assert(!finish(&f, 1, 0, OD_TBT_CTL_QUARANTINED));
            assert(finish(&f, 1, 1, OD_TBT_CTL_OK) == 1);
        } else {
            assert(status == OD_TBT_CTL_OK && count == 1);
            assert(results[0].succeeded == (mode == 6));
        }
    }
}
static void timeout_removal_and_completion_retry(void) {
    Fixture f; fixture(&f, 1); map_buffers(&f); uint32_t tx;
    assert(submit(&f, &tx) == OD_TBT_CTL_OK);
    assert(od_tbt_controller_tick(&f.controller, 9) == OD_TBT_CTL_ARGUMENT);
    assert(od_tbt_controller_tick(&f.controller, 99) == OD_TBT_CTL_OK);
    unsigned operations = f.operation;
    assert(od_tbt_controller_tick(&f.controller, 100) == OD_TBT_CTL_TIMEOUT);
    assert(f.operation == operations && !f.completed);
    assert(od_tbt_controller_begin_stop(&f.controller, 1) == OD_TBT_CTL_OK && f.operation == operations);
    assert(!finish(&f, 0, 1, OD_TBT_CTL_NOT_READY));
    assert(!finish(&f, 1, 0, OD_TBT_CTL_QUARANTINED));
    f.failed_complete = 1;
    assert(!finish(&f, 1, 1, OD_TBT_CTL_DMA) && !f.released);
    assert(finish(&f, 1, 1, OD_TBT_CTL_OK) == 1 && f.controller.state == OD_TBT_CTL_REMOVED);

    fixture(&f, 1); map_buffers(&f); assert(submit(&f, &tx) == OD_TBT_CTL_OK);
    response(&f, tx, 1, 0); f.failed_complete = 1;
    ODTBTControllerCompletion results[32]; size_t count;
    assert(od_tbt_controller_poll(&f.controller, 20, results, 32, &count) == OD_TBT_CTL_DMA && !count);
    assert(!f.released && f.request.state == OD_DMA_LEASE_QUARANTINED);
    assert(!finish(&f, 1, 0, OD_TBT_CTL_QUARANTINED));
    assert(finish(&f, 1, 1, OD_TBT_CTL_OK) == 1);
}
static void start_and_command_gates(void) {
    Fixture f;
    for (unsigned mode = 0; mode < 8; ++mode) {
        fixture(&f, 0);
        switch (mode) {
            case 0: put32(f.bar1, 0x80, 3); break;
            case 1: put32(f.bar1, REQUEST, UINT32_MAX); break;
            case 2: put32(f.bar1, 0x110, 1); break;
            case 3: put32(f.bar1, 0x34, REQUEST); break;
            case 4: put32(f.bar0, RSP_IRQ, 1); break;
            case 5: put32(f.bar1, 0x3c, BAR1_BYTES - 4); break;
            case 6: put32(f.bar0, 0x21878, UINT32_MAX); break;
            case 7: put32(f.bar1, 0x90, 33); break;
        }
        assert(od_tbt_controller_start(&f.controller) != OD_TBT_CTL_OK);
        for (unsigned i = 0; i < f.trace_count; ++i) assert(f.trace[i].kind != 'w');
        assert(!finish(&f, 1, 0, OD_TBT_CTL_OK)); // Corrupt window never indexes past slots.
    }
    fixture(&f, 0); put32(f.bar0, 0x21878, 0x200000);
    assert(od_tbt_controller_start(&f.controller) == OD_TBT_CTL_NOT_READY);
    put32(f.bar0, 0x21878, 0);
    assert(od_tbt_controller_start(&f.controller) == OD_TBT_CTL_OK);
    map_buffers(&f);
    ODTBTTaskInput t = task(); uint32_t tx = 99;
    t.cdb[0] = 0x2a; // A mapped buffer never grants write authority.
    unsigned operations = f.operation;
    assert(od_tbt_controller_submit(&f.controller, &t, &f.request, &f.payload,
                                    10, 100, &tx) == OD_TBT_CTL_UNSUPPORTED);
    assert(f.operation == operations && tx == 99 && f.request.state == OD_DMA_LEASE_PREPARED);
    f.payload.segments[0].address = f.request.segments[0].address;
    assert(submit(&f, &tx) == OD_TBT_CTL_ARGUMENT && f.operation == operations);
    f.payload.segments[0].address = f.payload_buffer.address;
    f.payload.device_access = OD_DMA_DEVICE_READ;
    assert(submit(&f, &tx) == OD_TBT_CTL_ARGUMENT && f.operation == operations);
    f.payload.device_access = OD_DMA_DEVICE_WRITE;
    f.request.device_access = OD_DMA_DEVICE_WRITE;
    assert(submit(&f, &tx) == OD_TBT_CTL_ARGUMENT && f.operation == operations);
    f.request.device_access = OD_DMA_DEVICE_READ | OD_DMA_DEVICE_WRITE;
    f.controller.next_transaction = UINT32_MAX;
    assert(submit(&f, &tx) == OD_TBT_CTL_EXHAUSTED);
    assert(od_dma_lease_abort_unpublished(&f.request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_abort_unpublished(&f.payload) == OD_DMA_STATUS_OK);
}
static void partial_start_and_stop_retry(void) {
    Fixture f; fixture(&f, 1);
    unsigned enabled_write = 0;
    for (unsigned i = 0; i < f.trace_count; ++i)
        if (f.trace[i].kind == 'w' && f.trace[i].bar == 0 &&
            f.trace[i].offset == SUMMARY_IRQ + 4) enabled_write = i;
    assert(enabled_write);
    for (unsigned after = 2; after <= 3; ++after) {
        fixture(&f, 0); f.fail_operation = enabled_write + after;
        assert(od_tbt_controller_start(&f.controller) == OD_TBT_CTL_IO);
        assert(f.bar0[(SUMMARY_IRQ + 4) / 4] == 1 && f.controller.device_interrupts_touched);
        assert(od_tbt_controller_begin_stop(&f.controller, 0) == OD_TBT_CTL_OK);
        assert(!f.controller.device_interrupts_touched && !f.bar0[(SUMMARY_IRQ + 4) / 4]);
        assert(!finish(&f, 1, 0, OD_TBT_CTL_OK));
    }
    fixture(&f, 0); f.removed_operation = enabled_write + 3;
    assert(od_tbt_controller_start(&f.controller) == OD_TBT_CTL_IO && f.controller.removed);
    unsigned operations = f.operation;
    assert(od_tbt_controller_begin_stop(&f.controller, 0) == OD_TBT_CTL_OK && f.operation == operations);
    assert(!finish(&f, 1, 0, OD_TBT_CTL_OK) && f.controller.state == OD_TBT_CTL_REMOVED);

    fixture(&f, 1); unsigned before = f.operation;
    assert(od_tbt_controller_begin_stop(&f.controller, 0) == OD_TBT_CTL_OK);
    operations = f.operation - before;
    for (unsigned fail = 1; fail <= operations; ++fail) {
        fixture(&f, 1); f.fail_operation = f.operation + fail;
        assert(od_tbt_controller_begin_stop(&f.controller, 0) == OD_TBT_CTL_IO);
        assert(f.controller.device_interrupts_touched);
        assert(od_tbt_controller_begin_stop(&f.controller, 0) == OD_TBT_CTL_OK);
        assert(!f.controller.device_interrupts_touched && !f.bar0[(SUMMARY_IRQ + 4) / 4]);
        assert(!finish(&f, 1, 0, OD_TBT_CTL_OK));
    }
    printf("PASS: partial startup and %u shutdown failure points retry masking\n", operations);
}
static void multiple_slots_and_stale_reuse(void) {
    Fixture f; fixture(&f, 1); map_buffers(&f); uint32_t first, second;
    assert(submit(&f, &first) == OD_TBT_CTL_OK);
    ODDmaLease request = OD_DMA_LEASE_INITIALIZER, payload = OD_DMA_LEASE_INITIALIZER;
    Buffer request_buffer = {.address = 0x300000, .bytes = 104, .access = 3};
    Buffer payload_buffer = {.address = 0x400000, .bytes = 512, .access = 2};
    ODDmaBackend backend = f.request.backend;
    assert(od_dma_lease_prepare(&request, &backend, &request_buffer, 0, 104) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_prepare(&payload, &backend, &payload_buffer, 0, 512) == OD_DMA_STATUS_OK);
    put32(f.bar0, REQ_IRQ, 0);
    ODTBTTaskInput t = task();
    assert(od_tbt_controller_submit(&f.controller, &t, &request, &payload,
                                    11, 100, &second) == OD_TBT_CTL_ARGUMENT); // Duplicate task.
    ++t.controller_task_id;
    assert(od_tbt_controller_submit(&f.controller, &t, &request, &payload,
                                    11, 100, &second) == OD_TBT_CTL_OK && second != first);
    response(&f, first, 1, 0);
    put32(f.bar1, RESPONSE + 4, 2);
    memcpy((uint8_t *)f.bar1 + RESPONSE + 40, (uint8_t *)f.bar1 + RESPONSE + 8, 32);
    put32(f.bar1, RESPONSE + 8, 1); put32(f.bar1, RESPONSE + 12, second);
    ODTBTControllerCompletion results[32]; size_t count;
    assert(od_tbt_controller_poll(&f.controller, 20, results, 32, &count) == OD_TBT_CTL_OK);
    assert(count == 2 && results[0].task_id == t.controller_task_id &&
           results[1].task_id == task().controller_task_id && f.completed == 4);
    assert(od_dma_lease_init(&f.request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_init(&f.payload) == OD_DMA_STATUS_OK);
    map_buffers(&f);
    assert(od_tbt_controller_submit(&f.controller, &(ODTBTTaskInput){
        .version = 1, .direction = 2, .requested_bytes = 512,
        .controller_task_id = 7, .cdb_length = 10,
        .cdb = {0x5a, 8, 0x3a, 2, 0, 0, 0, 2, 0, 0}},
        &f.request, &f.payload, 30, 100, &second) == OD_TBT_CTL_OK && second > first);
    response(&f, first, 1, 0); // An old reply must not complete a reused queue slot.
    assert(od_tbt_controller_poll(&f.controller, 31, results, 32, &count) == OD_TBT_CTL_FORMAT);
    assert(!count && f.completed == 4 && f.request.state == OD_DMA_LEASE_QUARANTINED);
    assert(finish(&f, 1, 1, OD_TBT_CTL_OK) == 1 && f.completed == 6);
}
static void foreign_dma_provider(void) {
    Fixture local, foreign; fixture(&local, 1); fixture(&foreign, 0);
    map_buffers(&local); map_buffers(&foreign);
    /* Keep the foreign mapping physically disjoint to isolate provider binding. */
    foreign.payload.segments[0].address = 0x600000;
    unsigned operations = local.operation; uint32_t tx = 77; ODTBTTaskInput t = task();
    assert(od_tbt_controller_submit(&local.controller, &t, &local.request, &foreign.payload,
                                    10, 100, &tx) == OD_TBT_CTL_ARGUMENT);
    assert(local.operation == operations && tx == 77 && !local.completed && !foreign.completed);
    assert(local.request.state == OD_DMA_LEASE_PREPARED && foreign.payload.state == OD_DMA_LEASE_PREPARED);
    assert(od_dma_lease_abort_unpublished(&local.request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_abort_unpublished(&local.payload) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_abort_unpublished(&foreign.request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_abort_unpublished(&foreign.payload) == OD_DMA_STATUS_OK);
}
static void storage_wire_and_gates(void) {
    Fixture f; fixture_channel(&f, 1, OD_TBT_CHANNEL_STORAGE, 0);
    map_buffers_exact(&f, OD_TBT_STORAGE_REQUEST_BYTES, 512);
    ODTBTTaskInput t = read10(3, 1); uint32_t tx = 0;
    assert(od_tbt_controller_submit(&f.controller, &t, &f.request, &f.payload,
                                    10, 100, &tx) == OD_TBT_CTL_OK && tx == 1);
    const uint8_t expected[OD_TBT_STORAGE_REQUEST_BYTES] = {
        [2] = 0x10, [12] = 1, [16] = 0x88, [17] = 0x77, [18] = 0x66,
        [19] = 0x55, [20] = 0x44, [21] = 0x33, [22] = 0x22, [23] = 0x11,
        [24] = 0x90, [28] = 1, [48] = 0x32, [50] = 0x40, [53] = 2,
        [72] = 1, [80] = 3, [96] = 1, [130] = 0x20, [137] = 2
    };
    assert(!memcmp(f.request_buffer.cpu, expected, sizeof(expected)));
    assert(!f.request_buffer.cpu[56] && !f.request_buffer.cpu[57] &&
           !f.request_buffer.cpu[58] && !f.request_buffer.cpu[59]);
    assert(f.bar1[(STORAGE_REQUEST + 16) / 4] == OD_TBT_STORAGE_REQUEST_BYTES &&
           f.bar1[(REQUEST + 16) / 4] == 0 && f.bar0[STORAGE_REQ_IRQ / 4] &&
           !f.bar0[REQ_IRQ / 4]);
    storage_response(&f, tx, 512);
    ODTBTControllerCompletion results[OD_TBT_CONTROLLER_SLOTS]; size_t count;
    assert(od_tbt_controller_poll(&f.controller, 20, results, OD_TBT_CONTROLLER_SLOTS, &count) == OD_TBT_CTL_OK);
    assert(count == 1 && results[0].succeeded && results[0].task_id == t.controller_task_id);

    assert(od_dma_lease_init(&f.request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_init(&f.payload) == OD_DMA_STATUS_OK);
    map_buffers_exact(&f, OD_TBT_STORAGE_REQUEST_BYTES, 512);
    t = read16(3, 1);
    assert(od_tbt_controller_submit(&f.controller, &t, &f.request, &f.payload,
                                    21, 100, &tx) == OD_TBT_CTL_OK && tx == 2);
    assert(f.request_buffer.cpu[28] == 1 && f.request_buffer.cpu[72] == 1 &&
           f.request_buffer.cpu[80] == 3 && f.request_buffer.cpu[96] == 2);
    storage_response(&f, tx, 512);
    assert(od_tbt_controller_poll(&f.controller, 22, results, OD_TBT_CONTROLLER_SLOTS, &count) == OD_TBT_CTL_OK &&
           count == 1 && results[0].succeeded);

    Fixture zero; fixture_channel(&zero, 0, OD_TBT_CHANNEL_STORAGE, 0);
    put32(zero.bar1, 0x108, 0);
    assert(od_tbt_controller_start(&zero.controller) == OD_TBT_CTL_UNSUPPORTED);
    Fixture overflow; fixture_channel(&overflow, 0, OD_TBT_CHANNEL_STORAGE, 0);
    put32(overflow.bar1, 0x10c, 0x80000000U);
    assert(od_tbt_controller_start(&overflow.controller) == OD_TBT_CTL_UNSUPPORTED);
    Fixture bad_block_size; fixture_channel(&bad_block_size, 0, OD_TBT_CHANNEL_STORAGE, 0);
    put32(bad_block_size.bar1, 0x104, 4096);
    assert(od_tbt_controller_start(&bad_block_size.controller) == OD_TBT_CTL_UNSUPPORTED);

    Fixture bounds; fixture_channel(&bounds, 0, OD_TBT_CHANNEL_STORAGE, 0);
    put32(bounds.bar1, 0x108, 4);
    assert(od_tbt_controller_start(&bounds.controller) == OD_TBT_CTL_OK);
    map_buffers_exact(&bounds, OD_TBT_STORAGE_REQUEST_BYTES, 512);
    t = read10(4, 1); unsigned before = bounds.operation;
    assert(od_tbt_controller_submit(&bounds.controller, &t, &bounds.request, &bounds.payload,
                                    10, 100, &tx) == OD_TBT_CTL_UNSUPPORTED && bounds.operation == before);
    assert(od_dma_lease_abort_unpublished(&bounds.request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_abort_unpublished(&bounds.payload) == OD_DMA_STATUS_OK);

    Fixture overrun; fixture_channel(&overrun, 0, OD_TBT_CHANNEL_STORAGE, 0);
    put32(overrun.bar1, 0x108, 4);
    assert(od_tbt_controller_start(&overrun.controller) == OD_TBT_CTL_OK);
    map_buffers_exact(&overrun, OD_TBT_STORAGE_REQUEST_BYTES, 1024);
    t = read10(3, 2);
    assert(od_tbt_controller_submit(&overrun.controller, &t, &overrun.request, &overrun.payload,
                                    10, 100, &tx) == OD_TBT_CTL_UNSUPPORTED);
    assert(od_dma_lease_abort_unpublished(&overrun.request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_abort_unpublished(&overrun.payload) == OD_DMA_STATUS_OK);

    Fixture readonly; fixture_channel(&readonly, 1, OD_TBT_CHANNEL_STORAGE, 0);
    map_buffers_exact(&readonly, OD_TBT_STORAGE_REQUEST_BYTES, 512);
    t = read10(0, 1); t.direction = 1; t.cdb[0] = 0x2a;
    assert(od_tbt_controller_submit(&readonly.controller, &t, &readonly.request, &readonly.payload,
                                    10, 100, &tx) == OD_TBT_CTL_UNSUPPORTED);
    assert(od_dma_lease_abort_unpublished(&readonly.request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_abort_unpublished(&readonly.payload) == OD_DMA_STATUS_OK);

    Fixture exact_bytes; fixture_channel(&exact_bytes, 1, OD_TBT_CHANNEL_STORAGE, 0);
    map_buffers_exact(&exact_bytes, OD_TBT_STORAGE_REQUEST_BYTES, 1024);
    t = read10(0, 1); t.requested_bytes = 1024; unsigned exact_before = exact_bytes.operation;
    assert(od_tbt_controller_submit(&exact_bytes.controller, &t, &exact_bytes.request, &exact_bytes.payload,
                                    10, 100, &tx) == OD_TBT_CTL_UNSUPPORTED &&
           exact_bytes.operation == exact_before);
    t = read16(0, 1); t.requested_bytes = 1024;
    assert(od_tbt_controller_submit(&exact_bytes.controller, &t, &exact_bytes.request, &exact_bytes.payload,
                                    10, 100, &tx) == OD_TBT_CTL_UNSUPPORTED &&
           exact_bytes.operation == exact_before);
    assert(od_dma_lease_abort_unpublished(&exact_bytes.request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_abort_unpublished(&exact_bytes.payload) == OD_DMA_STATUS_OK);

    Fixture storage_mismatch; fixture_channel(&storage_mismatch, 1, OD_TBT_CHANNEL_STORAGE, 0);
    map_buffers_exact(&storage_mismatch, OD_TBT_STORAGE_REQUEST_BYTES, 512);
    t = task();
    assert(od_tbt_controller_submit(&storage_mismatch.controller, &t,
                                    &storage_mismatch.request, &storage_mismatch.payload,
                                    10, 100, &tx) == OD_TBT_CTL_UNSUPPORTED);
    assert(od_dma_lease_abort_unpublished(&storage_mismatch.request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_abort_unpublished(&storage_mismatch.payload) == OD_DMA_STATUS_OK);

    Fixture management; fixture(&management, 1); map_buffers(&management);
    t = read10(0, 1);
    assert(od_tbt_controller_submit(&management.controller, &t, &management.request, &management.payload,
                                    10, 100, &tx) == OD_TBT_CTL_UNSUPPORTED);
    assert(od_dma_lease_abort_unpublished(&management.request) == OD_DMA_STATUS_OK);
    assert(od_dma_lease_abort_unpublished(&management.payload) == OD_DMA_STATUS_OK);
}
static void external_summary_is_service_owned(void) {
    Fixture management; fixture_channel(&management, 1, OD_TBT_CHANNEL_MANAGEMENT, 1);
    assert(!writes_at(&management, 0, SUMMARY_IRQ) && !writes_at(&management, 0, SUMMARY_IRQ + 4));
    assert(od_tbt_controller_begin_stop(&management.controller, 0) == OD_TBT_CTL_OK);
    assert(!writes_at(&management, 0, SUMMARY_IRQ) && !writes_at(&management, 0, SUMMARY_IRQ + 4));

    Fixture storage; fixture_channel(&storage, 1, OD_TBT_CHANNEL_STORAGE, 1);
    uint32_t summary_state = UINT32_MAX;
    assert(od_tbt_controller_summary_state(&storage.controller, &summary_state) == OD_TBT_CTL_OK &&
           summary_state == 0);
    put32(storage.bar0, SUMMARY_IRQ, 1);
    assert(od_tbt_controller_summary_state(&storage.controller, &summary_state) == OD_TBT_CTL_OK &&
           summary_state == 1);
    assert(!writes_at(&storage, 0, SUMMARY_IRQ) && !writes_at(&storage, 0, SUMMARY_IRQ + 4));
    put32(storage.bar0, SUMMARY_IRQ, 0);
    map_buffers_exact(&storage, OD_TBT_STORAGE_REQUEST_BYTES, 512);
    ODTBTTaskInput t = read10(0, 1); uint32_t tx;
    assert(od_tbt_controller_submit(&storage.controller, &t, &storage.request, &storage.payload,
                                    10, 100, &tx) == OD_TBT_CTL_OK);
    storage_response(&storage, tx, 512);
    ODTBTControllerCompletion results[OD_TBT_CONTROLLER_SLOTS]; size_t count;
    assert(od_tbt_controller_poll(&storage.controller, 20, results, OD_TBT_CONTROLLER_SLOTS, &count) == OD_TBT_CTL_OK &&
           count == 1);
    assert(!writes_at(&storage, 0, SUMMARY_IRQ) && !writes_at(&storage, 0, SUMMARY_IRQ + 4));
    assert(od_tbt_controller_summary_clear(&storage.controller) == OD_TBT_CTL_OK);
    assert(writes_at(&storage, 0, SUMMARY_IRQ));
    assert(od_tbt_controller_summary_enable(&storage.controller) == OD_TBT_CTL_OK);
    assert(writes_at(&storage, 0, SUMMARY_IRQ + 4));
    assert(od_tbt_controller_summary_disable(&storage.controller) == OD_TBT_CTL_OK);
    assert(od_tbt_controller_begin_stop(&storage.controller, 0) == OD_TBT_CTL_OK);

    Fixture removed; fixture_channel(&removed, 1, OD_TBT_CHANNEL_STORAGE, 1);
    put32(removed.bar0, SUMMARY_IRQ, UINT32_MAX);
    assert(od_tbt_controller_summary_state(&removed.controller, &summary_state) == OD_TBT_CTL_IO &&
           removed.controller.removed && removed.controller.state == OD_TBT_CTL_QUIESCING);
    assert(!writes_at(&removed, 0, SUMMARY_IRQ) && !writes_at(&removed, 0, SUMMARY_IRQ + 4));
}
typedef struct { Fixture *fixture; unsigned retained, released, completed; } DrainAction;
static void drain_retain(void *ctx, void *action) { assert(action); ++((DrainAction *)ctx)->retained; }
static void drain_release(void *ctx, void *action) { assert(action); ++((DrainAction *)ctx)->released; }
static void drain_complete(void *ctx, void *action, const ODTBTControllerCompletion *result) {
    DrainAction *a = ctx; assert(action && (result->succeeded || result->cancelled));
    assert(a->fixture->completed == 2 && a->fixture->request.state == OD_DMA_LEASE_COMPLETED &&
           a->fixture->payload.state == OD_DMA_LEASE_COMPLETED);
    ++a->completed;
}
static void graceful_completion_drain(void) {
    Fixture f; fixture_channel(&f, 1, OD_TBT_CHANNEL_STORAGE, 1);
    map_buffers_exact(&f, OD_TBT_STORAGE_REQUEST_BYTES, 512);
    ODScsiDrain drain = OD_SCSI_DRAIN_INITIALIZER;
    ODScsiTaskTable tasks = {0}; DrainAction action = {&f, 0, 0, 0};
    ODScsiTaskCallbacks callbacks = {&action, drain_retain, drain_release, drain_complete};
    assert(od_scsi_drain_init(&drain) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_tasks_init(&tasks, &callbacks) == OD_SCSI_TASK_OK);
    ODTBTTaskInput input = read10(0, 1); uint32_t slot, tx;
    assert(od_scsi_tasks_reserve(&tasks, input.controller_task_id, &action,
                                &f.request, &f.payload, &slot) == OD_SCSI_TASK_OK);
    assert(od_tbt_controller_submit(&f.controller, &input, &f.request, &f.payload, 10, 100, &tx) == OD_TBT_CTL_OK);
    assert(od_scsi_tasks_bind(&tasks, slot, tx) == OD_SCSI_TASK_OK);
    assert(od_scsi_drain_begin(&drain, 11, 90) == OD_SCSI_DRAIN_WAIT);
    assert(!od_scsi_drain_accepts_work(&drain) && od_scsi_drain_polls_completions(&drain));
    assert(od_scsi_drain_observe(&drain, 12, od_scsi_tasks_pending(&tasks), 2) == OD_SCSI_DRAIN_WAIT);
    ODTBTControllerCompletion results[32]; size_t count = 99;
    assert(od_tbt_controller_poll(&f.controller, 12, results, 32, &count) == OD_TBT_CTL_OK && !count);
    assert(!action.completed && !f.completed); // A zero response count is not completion.
    assert(od_scsi_drain_observe(&drain, 13, od_scsi_tasks_pending(&tasks), 2) == OD_SCSI_DRAIN_WAIT);
    storage_response(&f, tx, 512);
    assert(od_tbt_controller_poll(&f.controller, 14, results, 32, &count) == OD_TBT_CTL_OK && count == 1);
    assert(f.completed == 2 && !action.completed);
    assert(od_scsi_drain_observe(&drain, 14, od_scsi_tasks_pending(&tasks), 0) == OD_SCSI_DRAIN_WAIT);
    assert(od_scsi_tasks_finish(&tasks, results) == OD_SCSI_TASK_OK);
    assert(action.completed == 1 && action.retained == action.released);
    assert(od_scsi_drain_observe(&drain, 15, od_scsi_tasks_pending(&tasks), 0) == OD_SCSI_DRAIN_CLOSE);
    assert(!od_scsi_drain_polls_completions(&drain));
    assert(od_tbt_controller_begin_stop(&f.controller, 0) == OD_TBT_CTL_OK);
    assert(!finish(&f, 1, 0, OD_TBT_CTL_OK)); // No global bus-quiescence attestation needed or supplied.
}
static void failed_completion_cannot_drain(void) {
    for (unsigned mode = 0; mode < 4; ++mode) {
        Fixture f; fixture_channel(&f, 1, OD_TBT_CHANNEL_STORAGE, 1);
        map_buffers_exact(&f, OD_TBT_STORAGE_REQUEST_BYTES, 512);
        ODTBTTaskInput input = read10(0, 1); uint32_t tx, slot;
        ODScsiTaskTable tasks = {0}; DrainAction action = {&f, 0, 0, 0};
        ODScsiTaskCallbacks callbacks = {&action, drain_retain, drain_release, drain_complete};
        assert(od_scsi_tasks_init(&tasks, &callbacks) == OD_SCSI_TASK_OK);
        assert(od_scsi_tasks_reserve(&tasks, input.controller_task_id, &action,
                                    &f.request, &f.payload, &slot) == OD_SCSI_TASK_OK);
        assert(od_tbt_controller_submit(&f.controller, &input, &f.request, &f.payload, 10, 100, &tx) == OD_TBT_CTL_OK);
        assert(od_scsi_tasks_bind(&tasks, slot, tx) == OD_SCSI_TASK_OK);
        ODScsiDrain drain = OD_SCSI_DRAIN_INITIALIZER;
        assert(od_scsi_drain_init(&drain) == OD_SCSI_DRAIN_WAIT);
        assert(od_scsi_drain_begin(&drain, 11, 90) == OD_SCSI_DRAIN_WAIT);
        ODTBTControllerCompletion results[32]; size_t count = 99;
        ODTBTControllerStatus status;
        ODScsiDrainFailure expected;
        if (mode == 0) {
            storage_response(&f, tx + 1, 512);
            status = od_tbt_controller_poll(&f.controller, 12, results, 32, &count);
            assert(status == OD_TBT_CTL_FORMAT && !count);
            expected = OD_SCSI_DRAIN_FAILURE_PROTOCOL;
        } else if (mode == 1) {
            status = od_tbt_controller_poll(&f.controller, 100, results, 32, &count);
            assert(status == OD_TBT_CTL_TIMEOUT && !count);
            expected = OD_SCSI_DRAIN_FAILURE_TIMEOUT;
        } else if (mode == 2) {
            put32(f.bar0, STORAGE_RSP_IRQ, UINT32_MAX);
            status = od_tbt_controller_poll(&f.controller, 12, results, 32, &count);
            assert(status == OD_TBT_CTL_IO && f.controller.removed && !count);
            expected = OD_SCSI_DRAIN_FAILURE_REMOVAL;
        } else {
            put32(f.bar0, SUMMARY_IRQ, UINT32_MAX);
            status = od_tbt_controller_summary_clear(&f.controller);
            assert(status == OD_TBT_CTL_IO && f.controller.removed);
            expected = OD_SCSI_DRAIN_FAILURE_REMOVAL;
        }
        // Same status classifier as the SDK service's queue/summary fault path.
        assert(od_scsi_drain_fail_controller(&drain, status, f.controller.removed) == OD_SCSI_DRAIN_ERROR);
        assert(drain.failure == expected);
        assert(f.request.state == OD_DMA_LEASE_QUARANTINED && f.payload.state == OD_DMA_LEASE_QUARANTINED);
        assert(!f.completed && !f.released && !od_scsi_drain_accepts_work(&drain));
        ODTBTControllerCompletion cancellation = {input.controller_task_id, tx, 0, 0, 0, 0, 1};
        assert(od_scsi_tasks_finish(&tasks, &cancellation) == OD_SCSI_TASK_DMA_LIVE);
        assert(action.retained == 1 && !action.released && !action.completed && od_scsi_tasks_pending(&tasks) == 1);
        assert(!finish(&f, 1, 0, OD_TBT_CTL_QUARANTINED)); // Cancellation cannot release uncertain DMA.
        assert(od_scsi_drain_observe(&drain, 91, 0, 0) == OD_SCSI_DRAIN_ERROR);
        assert(finish(&f, 1, 1, OD_TBT_CTL_OK) == 1); // Explicit test-only external quiescence, never service behavior.
        assert(od_scsi_tasks_finish(&tasks, &cancellation) == OD_SCSI_TASK_OK);
        assert(!od_scsi_tasks_pending(&tasks) && action.completed == 1 && action.retained == action.released);
    }
}
int main(void) {
    success_and_wire(); legacy_management_init_stays_owned(); publication_failure_matrix(); malformed_and_status_completions();
    timeout_removal_and_completion_retry(); start_and_command_gates(); partial_start_and_stop_retry();
    multiple_slots_and_stale_reuse();
    foreign_dma_provider();
    storage_wire_and_gates(); external_summary_is_service_owned();
    graceful_completion_drain(); failed_completion_cannot_drain();
    puts("PASS: controller handshake, storage frames, completion association, DMA, summary, timeout, removal, and drain gates");
    return 0;
}
