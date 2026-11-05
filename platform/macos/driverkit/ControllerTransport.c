#include "ControllerTransport.h"
#include <string.h>

#define CONTROLLER_MAGIC UINT32_C(0x4f445443)
typedef struct { uint32_t offset, length; } Region;

static int valid(const ODTBTController *c) { return c && c->magic == CONTROLLER_MAGIC; }
static int region(uint32_t offset, uint32_t length, uint32_t bar_bytes) {
    return !(offset & 3U) && length && offset <= bar_bytes && length <= bar_bytes - offset;
}
static int overlap(Region a, Region b) {
    return (uint64_t)a.offset < (uint64_t)b.offset + b.length &&
           (uint64_t)b.offset < (uint64_t)a.offset + a.length;
}
static int read_bar(ODTBTController *c, uint8_t bar, uint32_t offset, uint32_t *value) {
    uint32_t bytes = bar ? c->bar1_bytes : c->bar0_bytes;
    return !region(offset, 4, bytes) || c->io.read32(c->io.context, bar, offset, value);
}
static int write_bar(ODTBTController *c, uint8_t bar, uint32_t offset, uint32_t value) {
    uint32_t bytes = bar ? c->bar1_bytes : c->bar0_bytes;
    return !region(offset, 4, bytes) || c->io.write32(c->io.context, bar, offset, value);
}
static int irq_read(void *context, uint32_t offset, uint32_t *value) {
    return read_bar(context, 0, offset, value);
}
static int irq_write(void *context, uint32_t offset, uint32_t value) {
    return write_bar(context, 0, offset, value);
}
static int order(void *context) {
    ODTBTController *c = context;
    return c->io.order(c->io.context);
}
static ODTBTInterruptMMIO mmio(ODTBTController *c) {
    ODTBTInterruptMMIO result = { c, irq_read, irq_write, order };
    return result;
}
static int action(ODTBTController *c, const ODTBTInterruptProgram *p,
                  ODTBTInterruptAction operation) {
    ODTBTInterruptMMIO io = mmio(c);
    ODTBTInterruptStatus status = od_tbt_interrupt_apply(p, operation, &io);
    if (status == OD_TBT_INTERRUPT_REMOVED) c->removed = 1;
    return status != OD_TBT_INTERRUPT_OK;
}
static int irq_state(ODTBTController *c, const ODTBTInterruptProgram *p, uint32_t *state) {
    ODTBTInterruptMMIO io = mmio(c);
    ODTBTInterruptStatus status = od_tbt_interrupt_state(p, &io, state);
    if (status == OD_TBT_INTERRUPT_REMOVED) c->removed = 1;
    return status != OD_TBT_INTERRUPT_OK || *state == UINT32_MAX;
}
static void quarantine(ODDmaLease *lease) {
    if (lease && lease->state == OD_DMA_LEASE_PUBLISHED)
        (void)od_dma_lease_quarantine_published(lease);
}
static ODTBTControllerStatus fault(ODTBTController *c, ODTBTControllerStatus status) {
    c->state = OD_TBT_CTL_QUIESCING;
    for (unsigned i = 0; i < OD_TBT_CONTROLLER_SLOTS; ++i) {
        quarantine(c->slots[i].request);
        quarantine(c->slots[i].payload);
    }
    return status;
}
static void le32(uint8_t *p, uint32_t value) {
    for (unsigned i = 0; i < 4; ++i) p[i] = (uint8_t)(value >> (i * 8));
}
static void le64(uint8_t *p, uint64_t value) {
    le32(p, (uint32_t)value); le32(p + 4, (uint32_t)(value >> 32));
}
static int load_program(ODTBTController *c, uint32_t offset, ODTBTInterruptProgram *p) {
    uint8_t bytes[OD_TBT_INTERRUPT_PROGRAM_BYTES];
    if (!region(offset, sizeof(bytes), c->bar1_bytes)) return 1;
    for (unsigned i = 0; i < sizeof(bytes); i += 4) {
        uint32_t value;
        if (read_bar(c, 1, offset + i, &value)) return 1;
        le32(bytes + i, value);
    }
    return od_tbt_interrupt_parse(bytes, sizeof(bytes), 0, c->bar0_bytes, p) != OD_TBT_INTERRUPT_OK;
}

ODTBTControllerStatus od_tbt_controller_init(ODTBTController *c, const ODTBTControllerIO *io,
                                            uint32_t bar0_bytes, uint32_t bar1_bytes) {
    return od_tbt_controller_init_channel(c, io, bar0_bytes, bar1_bytes,
                                          OD_TBT_CHANNEL_MANAGEMENT, 0);
}
ODTBTControllerStatus od_tbt_controller_init_channel(ODTBTController *c,
                                                    const ODTBTControllerIO *io,
                                                    uint32_t bar0_bytes, uint32_t bar1_bytes,
                                                    ODTBTChannel channel, int external_summary) {
    if (!c || !io || !io->read32 || !io->write32 || !io->order || !io->write_request ||
        bar0_bytes < 0x2187c || bar1_bytes < 0x44 ||
        (channel != OD_TBT_CHANNEL_STORAGE && channel != OD_TBT_CHANNEL_MANAGEMENT))
        return OD_TBT_CTL_ARGUMENT;
    if (c->magic == CONTROLLER_MAGIC) return OD_TBT_CTL_STATE;
    ODTBTControllerIO copied = *io;
    memset(c, 0, sizeof(*c));
    c->magic = CONTROLLER_MAGIC;
    c->io = copied; c->bar0_bytes = bar0_bytes; c->bar1_bytes = bar1_bytes;
    c->channel = channel; c->external_summary = external_summary != 0;
    c->next_transaction = 1;
    return OD_TBT_CTL_OK;
}
int od_tbt_controller_running_for(const ODTBTController *c, const void *context) {
    return valid(c) && c->state == OD_TBT_CTL_RUNNING && c->io.context == context;
}

ODTBTControllerStatus od_tbt_controller_start(ODTBTController *c) {
    if (!valid(c) || c->state != OD_TBT_CTL_EMPTY) return OD_TBT_CTL_STATE;
    uint32_t ready, sig[3], caps, luns, pci, endpoint, version, max_transactions, max_luns;
    uint32_t directory = c->channel == OD_TBT_CHANNEL_STORAGE ? 0x20 : 0x30;
    if (read_bar(c, 0, 0x21878, &ready)) return fault(c, OD_TBT_CTL_IO);
    if (ready == UINT32_MAX) { c->removed = 1; return fault(c, OD_TBT_CTL_IO); }
    if (ready & 0x200000U) return OD_TBT_CTL_NOT_READY;
    for (unsigned i = 0; i < 3; ++i)
        if (read_bar(c, 1, i * 4, sig + i)) return fault(c, OD_TBT_CTL_IO);
    if (sig[0] != 0x626f7244 || sig[1] != 0x726f506f || (sig[2] & 0xffffU) != 0x0074)
        return fault(c, OD_TBT_CTL_FORMAT);
    if (read_bar(c, 1, 0x18, &luns) || read_bar(c, 1, 0x1c, &caps) ||
        read_bar(c, 1, directory, &c->request_offset) ||
        read_bar(c, 1, directory + 4, &c->response_offset) ||
        read_bar(c, 1, 0x38, &pci) || read_bar(c, 1, 0x3c, &endpoint))
        return fault(c, OD_TBT_CTL_IO);
    Region regions[] = { {0, 0x44}, {caps, 64}, {luns, 32 * 16},
                         {pci, 36 + 7 * 84}, {endpoint, 36 + 6 * 84},
                         {c->request_offset, 8}, {c->response_offset, 8} };
    for (unsigned i = 0; i < sizeof(regions) / sizeof(*regions); ++i)
        if (!region(regions[i].offset, regions[i].length, c->bar1_bytes))
            return fault(c, OD_TBT_CTL_FORMAT);
    if (read_bar(c, 1, caps, &version) || read_bar(c, 1, caps + 8, &c->max_transfer) ||
        read_bar(c, 1, caps + 12, &max_transactions) ||
        read_bar(c, 1, caps + 16, &max_luns) ||
        read_bar(c, 1, c->request_offset, &c->window)) return fault(c, OD_TBT_CTL_IO);
    if (version != 4 || !c->max_transfer || c->max_transfer > OD_DMA_MAX_TRANSFER_BYTES ||
        !max_transactions || !c->window || c->window > max_transactions ||
        c->window > OD_TBT_CONTROLLER_SLOTS || max_luns > 32)
        return fault(c, OD_TBT_CTL_UNSUPPORTED);
    regions[5].length = 8 + c->window * 16;
    regions[6].length = 8 + c->window * 32;
    for (unsigned i = 0; i < sizeof(regions) / sizeof(*regions); ++i) {
        if (!region(regions[i].offset, regions[i].length, c->bar1_bytes))
            return fault(c, OD_TBT_CTL_FORMAT);
        for (unsigned j = 0; j < i; ++j)
            if (overlap(regions[i], regions[j])) return fault(c, OD_TBT_CTL_FORMAT);
    }
    for (unsigned i = 0; i < 32; ++i) {
        uint32_t enabled;
        if (read_bar(c, 1, luns + i * 16, &enabled)) return fault(c, OD_TBT_CTL_IO);
        if ((i == 0 && !enabled) || (i && enabled)) return fault(c, OD_TBT_CTL_UNSUPPORTED);
    }
    if (c->channel == OD_TBT_CHANNEL_STORAGE) {
        uint32_t block_size, blocks_low, blocks_high;
        if (read_bar(c, 1, luns + 4, &block_size) ||
            read_bar(c, 1, luns + 8, &blocks_low) || read_bar(c, 1, luns + 12, &blocks_high))
            return fault(c, OD_TBT_CTL_IO);
        c->logical_blocks = (uint64_t)blocks_high << 32 | blocks_low;
        if (block_size != 512 || !c->logical_blocks || c->logical_blocks > UINT64_MAX / 512U)
            return fault(c, OD_TBT_CTL_UNSUPPORTED);
    }
    /* Storage uses PCI/endpoint entries 0/0; management uses 4/1. The shared
     * version-4 summary cause remains endpoint entry 5. */
    unsigned request_entry = c->channel == OD_TBT_CHANNEL_STORAGE ? 0 : 4;
    unsigned response_entry = c->channel == OD_TBT_CHANNEL_STORAGE ? 0 : 1;
    if (load_program(c, pci + 36 + request_entry * 84, &c->request_interrupt) ||
        load_program(c, endpoint + 36 + response_entry * 84, &c->response_interrupt) ||
        load_program(c, endpoint + 36 + 5 * 84, &c->summary_interrupt))
        return fault(c, OD_TBT_CTL_FORMAT);
    uint32_t request_state, response_state, summary_state = 0;
    if (irq_state(c, &c->request_interrupt, &request_state) ||
        irq_state(c, &c->response_interrupt, &response_state) ||
        (!c->external_summary && irq_state(c, &c->summary_interrupt, &summary_state)))
        return fault(c, OD_TBT_CTL_IO);
    /* Never discard a stale response whose DMA owner is unknown. */
    if (request_state || response_state || summary_state) return fault(c, OD_TBT_CTL_NOT_READY);
    if (write_bar(c, 1, caps + 0x24, 1) || order(c)) return fault(c, OD_TBT_CTL_IO);
    c->device_interrupts_touched = 1;
    if ((!c->external_summary && action(c, &c->summary_interrupt, OD_TBT_INTERRUPT_CLEAR)) ||
        action(c, &c->response_interrupt, OD_TBT_INTERRUPT_CLEAR) ||
        (!c->external_summary && action(c, &c->summary_interrupt, OD_TBT_INTERRUPT_ENABLE)))
        return fault(c, OD_TBT_CTL_IO);
    c->state = OD_TBT_CTL_RUNNING;
    return OD_TBT_CTL_OK;
}

static ODTBTControllerStatus summary_action(ODTBTController *c, ODTBTInterruptAction operation) {
    if (!valid(c) || c->state != OD_TBT_CTL_RUNNING || c->removed) return OD_TBT_CTL_STATE;
    return action(c, &c->summary_interrupt, operation) ? fault(c, OD_TBT_CTL_IO) : OD_TBT_CTL_OK;
}
ODTBTControllerStatus od_tbt_controller_summary_clear(ODTBTController *c) {
    return summary_action(c, OD_TBT_INTERRUPT_CLEAR);
}
ODTBTControllerStatus od_tbt_controller_summary_enable(ODTBTController *c) {
    return summary_action(c, OD_TBT_INTERRUPT_ENABLE);
}
ODTBTControllerStatus od_tbt_controller_summary_disable(ODTBTController *c) {
    return summary_action(c, OD_TBT_INTERRUPT_DISABLE);
}
ODTBTControllerStatus od_tbt_controller_summary_state(ODTBTController *c, uint32_t *state) {
    if (!state) return OD_TBT_CTL_ARGUMENT;
    if (!valid(c) || c->state != OD_TBT_CTL_RUNNING || c->removed) return OD_TBT_CTL_STATE;
    return irq_state(c, &c->summary_interrupt, state) ? fault(c, OD_TBT_CTL_IO) : OD_TBT_CTL_OK;
}

static int single_prepared(const ODDmaLease *lease, uint64_t bytes) {
    return lease && lease->magic == OD_DMA_LEASE_MAGIC && lease->state == OD_DMA_LEASE_PREPARED &&
           lease->command_handle && lease->memory_handle && lease->mapping_valid &&
           lease->segment_count == 1 && lease->requested_length == bytes &&
           od_dma_validate_mapping(lease->segments, 1, bytes, lease->device_access) == OD_DMA_STATUS_OK;
}
static int dma_overlap(const ODDmaLease *a, const ODDmaLease *b) {
    return a == b || a->command_handle == b->command_handle || a->memory_handle == b->memory_handle ||
           (a->segments[0].address < b->segments[0].address + b->requested_length &&
            b->segments[0].address < a->segments[0].address + a->requested_length);
}
static int same_dma_backend(const ODDmaLease *a, const ODDmaLease *b) {
    const ODDmaBackend *x = &a->backend, *y = &b->backend;
    return x->context == y->context && x->retain_memory == y->retain_memory &&
           x->release_memory == y->release_memory && x->create_command == y->create_command &&
           x->release_command == y->release_command && x->prepare == y->prepare &&
           x->complete == y->complete;
}
static int leased_elsewhere(const ODTBTController *c, const ODDmaLease *a, const ODDmaLease *b) {
    for (unsigned i = 0; i < c->window; ++i) if (c->slots[i].request) {
        const ODDmaLease *r = c->slots[i].request, *p = c->slots[i].payload;
        if (dma_overlap(a, r) || dma_overlap(a, p) || dma_overlap(b, r) || dma_overlap(b, p)) return 1;
    }
    return 0;
}
ODTBTControllerStatus od_tbt_controller_submit(ODTBTController *c, const ODTBTTaskInput *task,
                                             ODDmaLease *request, ODDmaLease *payload,
                                             uint64_t now_ms, uint64_t deadline_ms,
                                             uint32_t *transaction) {
    if (!valid(c) || c->state != OD_TBT_CTL_RUNNING) return OD_TBT_CTL_STATE;
    uint32_t request_bytes = c->channel == OD_TBT_CHANNEL_STORAGE ?
                             OD_TBT_STORAGE_REQUEST_BYTES : OD_TBT_TM_REQUEST_BYTES;
    if (!task || !transaction ||
        !single_prepared(request, request_bytes) ||
        !single_prepared(payload, task->requested_bytes) ||
        request->device_access != (OD_DMA_DEVICE_READ | OD_DMA_DEVICE_WRITE) ||
        !(payload->device_access & OD_DMA_DEVICE_WRITE) || !same_dma_backend(request, payload) ||
        dma_overlap(request, payload) ||
        leased_elsewhere(c, request, payload) || !now_ms || now_ms < c->last_time_ms ||
        deadline_ms <= now_ms) return OD_TBT_CTL_ARGUMENT;
    ODTBTTaskInput input = *task;
    input.buffer_iovm_address = payload->segments[0].address;
    ODTBTReadPlan plan;
    if (od_tbt_plan_read_only(&input, 512, c->max_transfer, &plan) != OD_TBT_PLAN_OK ||
        plan.channel != c->channel || plan.transfer_bytes != input.requested_bytes)
        return OD_TBT_CTL_UNSUPPORTED;
    if (c->channel == OD_TBT_CHANNEL_STORAGE &&
        (plan.logical_block >= c->logical_blocks ||
         plan.block_count > c->logical_blocks - plan.logical_block ||
         plan.transfer_bytes != (uint64_t)plan.block_count * 512U))
        return OD_TBT_CTL_UNSUPPORTED;
    unsigned index;
    for (index = 0; index < c->window && c->slots[index].request; ++index) { }
    if (index == c->window) return OD_TBT_CTL_NOT_READY;
    for (unsigned i = 0; i < c->window; ++i)
        if (c->slots[i].request && c->slots[i].task_id == input.controller_task_id)
            return OD_TBT_CTL_ARGUMENT;
    if (c->next_transaction == UINT32_MAX) return OD_TBT_CTL_EXHAUSTED;
    ODTBTControllerStatus tick = od_tbt_controller_tick(c, now_ms);
    if (tick != OD_TBT_CTL_OK) return tick;
    uint32_t state, window;
    if (irq_state(c, &c->request_interrupt, &state) ||
        read_bar(c, 1, c->request_offset, &window)) return fault(c, OD_TBT_CTL_IO);
    if (window != c->window) return fault(c, OD_TBT_CTL_FORMAT);
    if (state) return OD_TBT_CTL_NOT_READY;
    c->last_time_ms = now_ms;
    const uint32_t tx = c->next_transaction++;
    uint8_t body[OD_TBT_STORAGE_REQUEST_BYTES] = {0};
    le64(body, request->segments[0].address);
    le32(body + 8, index); le32(body + 12, tx);
    le64(body + 16, input.controller_task_id); // Opaque cookie, never a host pointer.
    le32(body + 24, request_bytes);
    if (c->channel == OD_TBT_CHANNEL_STORAGE) {
        le32(body + 28, 1);
        le32(body + 48, 0x400032); // Data-in, one SGL entry.
        le32(body + 52, (uint32_t)plan.transfer_bytes);
        /* +0x38 is a firmware-opaque host SGL cookie. It remains zero: no CPU
         * pointer is emitted. LUN 0 is represented by its zeroed +0x40 word. */
        le32(body + 72, plan.block_count);
        le64(body + 80, plan.logical_block);
        le64(body + 96, tx);
        le64(body + 128, payload->segments[0].address);
        le32(body + 136, (uint32_t)plan.transfer_bytes);
    } else {
        le32(body + 28, 3);
        le32(body + 32, 0x410028); // Data-in, one direct DMA segment.
        le32(body + 36, (uint32_t)plan.transfer_bytes);
        le64(body + 40, payload->segments[0].address);
        memcpy(body + 48, plan.cdb, plan.cdb_length);
        le64(body + 88, payload->segments[0].address);
    }
    if (c->io.write_request(c->io.context, request, body, request_bytes))
        return fault(c, OD_TBT_CTL_DMA);
    ODTBTControllerSlot slot = {request, payload, input.controller_task_id,
                               deadline_ms, plan.transfer_bytes, tx};
    c->slots[index] = slot;
    /* Ownership changes BEFORE the first publication write. Even a failed
     * write may reach hardware, so every subsequent failure quarantines both
     * mappings. No failed doorbell is treated as an unpublished rollback. */
    if (od_dma_lease_publish(request) != OD_DMA_STATUS_OK ||
        od_dma_lease_publish(payload) != OD_DMA_STATUS_OK) return fault(c, OD_TBT_CTL_DMA);
    if (order(c) || write_bar(c, 1, c->request_offset + 8, (uint32_t)request->segments[0].address) ||
        write_bar(c, 1, c->request_offset + 12, 0) ||
        write_bar(c, 1, c->request_offset + 16, request_bytes) ||
        write_bar(c, 1, c->request_offset + 20, tx) || order(c) ||
        write_bar(c, 1, c->request_offset + 4, 1) || order(c) ||
        action(c, &c->request_interrupt, OD_TBT_INTERRUPT_TRIGGER)) return fault(c, OD_TBT_CTL_IO);
    *transaction = tx;
    return OD_TBT_CTL_OK;
}

ODTBTControllerStatus od_tbt_controller_tick(ODTBTController *c, uint64_t now_ms) {
    if (!valid(c) || c->state != OD_TBT_CTL_RUNNING) return OD_TBT_CTL_STATE;
    if (!now_ms || now_ms < c->last_time_ms) return OD_TBT_CTL_ARGUMENT;
    c->last_time_ms = now_ms;
    for (unsigned i = 0; i < c->window; ++i)
        if (c->slots[i].request && now_ms >= c->slots[i].deadline_ms)
            return fault(c, OD_TBT_CTL_TIMEOUT);
    return OD_TBT_CTL_OK;
}

ODTBTControllerStatus od_tbt_controller_poll(ODTBTController *c, uint64_t now_ms,
                                           ODTBTControllerCompletion *results, size_t capacity,
                                           size_t *count) {
    if (count) *count = 0;
    if (!results || !count || capacity < OD_TBT_CONTROLLER_SLOTS) return OD_TBT_CTL_ARGUMENT;
    ODTBTControllerStatus tick = od_tbt_controller_tick(c, now_ms);
    if (tick != OD_TBT_CTL_OK) return tick;
    uint32_t ready;
    if (irq_state(c, &c->response_interrupt, &ready)) return fault(c, OD_TBT_CTL_IO);
    if (!ready) return OD_TBT_CTL_OK;
    uint32_t length;
    if (order(c) || read_bar(c, 1, c->response_offset + 4, &length)) return fault(c, OD_TBT_CTL_IO);
    if (!length || length > c->window) return fault(c, OD_TBT_CTL_FORMAT);
    ODTBTControllerCompletion copied[OD_TBT_CONTROLLER_SLOTS] = {0};
    uint32_t indices[OD_TBT_CONTROLLER_SLOTS] = {0}, seen = 0;
    for (uint32_t i = 0; i < length; ++i) {
        uint32_t data[8];
        for (unsigned j = 0; j < 8; ++j)
            if (read_bar(c, 1, c->response_offset + 8 + i * 32 + j * 4, data + j))
                return fault(c, OD_TBT_CTL_IO);
        uint32_t index = data[0];
        if (index >= c->window || (seen & (1U << index))) return fault(c, OD_TBT_CTL_FORMAT);
        seen |= 1U << index;
        ODTBTControllerSlot *s = c->slots + index;
        uint64_t realized = (uint64_t)data[3] << 32 | data[2];
        if (!s->request || data[1] != s->transaction || realized > s->expected_bytes ||
            data[6] > 0xffU) return fault(c, OD_TBT_CTL_FORMAT);
        indices[i] = index;
        copied[i] = (ODTBTControllerCompletion){s->task_id, s->transaction, realized, data[5], data[6],
                         (data[5] == 1 || data[5] == 0x12) && data[6] == 0, 0};
    }
    /* Validate the entire batch before acknowledging or unmapping anything.
     * Unknown words +0x10/+0x1c are preserved as uninterpreted protocol data. */
    if (order(c) || action(c, &c->response_interrupt, OD_TBT_INTERRUPT_CLEAR) ||
        (!c->external_summary && action(c, &c->summary_interrupt, OD_TBT_INTERRUPT_CLEAR)) || order(c))
        return fault(c, OD_TBT_CTL_IO);
    for (uint32_t i = 0; i < length; ++i) {
        ODTBTControllerSlot *s = c->slots + indices[i];
        if (od_dma_lease_complete(s->request) != OD_DMA_STATUS_OK ||
            od_dma_lease_complete(s->payload) != OD_DMA_STATUS_OK) return fault(c, OD_TBT_CTL_DMA);
    }
    for (uint32_t i = 0; i < length; ++i) memset(c->slots + indices[i], 0, sizeof(*c->slots));
    memcpy(results, copied, length * sizeof(*results));
    *count = length;
    return OD_TBT_CTL_OK;
}

ODTBTControllerStatus od_tbt_controller_begin_stop(ODTBTController *c, int removed) {
    if (!valid(c) || c->state == OD_TBT_CTL_STOPPED || c->state == OD_TBT_CTL_REMOVED)
        return OD_TBT_CTL_STATE;
    if (removed) c->removed = 1;
    (void)fault(c, OD_TBT_CTL_OK);
    if (c->device_interrupts_touched && !c->removed) {
        uint32_t ready;
        if (read_bar(c, 0, 0x21878, &ready)) return OD_TBT_CTL_IO;
        if (ready == UINT32_MAX) { c->removed = 1; return OD_TBT_CTL_IO; }
        /* Startup may fail AFTER the enable write reached hardware. The
         * explicit shutdown event masks that partially enabled device too. */
        if ((!c->external_summary && action(c, &c->summary_interrupt, OD_TBT_INTERRUPT_DISABLE)) ||
            action(c, &c->response_interrupt, OD_TBT_INTERRUPT_DISABLE) ||
            (!c->external_summary && action(c, &c->summary_interrupt, OD_TBT_INTERRUPT_CLEAR)) ||
            action(c, &c->response_interrupt, OD_TBT_INTERRUPT_CLEAR)) return OD_TBT_CTL_IO;
        c->device_interrupts_touched = 0;
    }
    return OD_TBT_CTL_OK;
}

static int reclaim(ODDmaLease *lease) {
    if (!lease || lease->state == OD_DMA_LEASE_COMPLETED) return 0;
    if (lease->state == OD_DMA_LEASE_PREPARED)
        return od_dma_lease_abort_unpublished(lease) != OD_DMA_STATUS_OK;
    return od_dma_lease_release_after_bus_master_quiescence(lease) != OD_DMA_STATUS_OK;
}
ODTBTControllerStatus od_tbt_controller_finish_stop(ODTBTController *c, int drained,
                                                  int quiescent,
                                                  ODTBTControllerCompletion *results,
                                                  size_t capacity, size_t *count) {
    if (count) *count = 0;
    if (!valid(c) || !results || !count || capacity < OD_TBT_CONTROLLER_SLOTS)
        return OD_TBT_CTL_ARGUMENT;
    if (c->state != OD_TBT_CTL_QUIESCING && c->state != OD_TBT_CTL_DMA_QUARANTINE)
        return OD_TBT_CTL_STATE;
    if (!drained) return OD_TBT_CTL_NOT_READY;
    for (unsigned i = 0; i < OD_TBT_CONTROLLER_SLOTS; ++i) if (c->slots[i].request) {
        if (!quiescent) { c->state = OD_TBT_CTL_DMA_QUARANTINE; return OD_TBT_CTL_QUARANTINED; }
        if (reclaim(c->slots[i].request) || reclaim(c->slots[i].payload)) {
            c->state = OD_TBT_CTL_DMA_QUARANTINE;
            return OD_TBT_CTL_DMA;
        }
    }
    for (unsigned i = 0; i < OD_TBT_CONTROLLER_SLOTS; ++i) if (c->slots[i].request) {
        results[*count] = (ODTBTControllerCompletion){c->slots[i].task_id,
                           c->slots[i].transaction, 0, 0, 0, 0, 1};
        ++*count;
        memset(c->slots + i, 0, sizeof(*c->slots));
    }
    c->state = c->removed ? OD_TBT_CTL_REMOVED : OD_TBT_CTL_STOPPED;
    return OD_TBT_CTL_OK;
}
