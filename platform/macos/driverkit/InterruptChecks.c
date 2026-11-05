#include "InterruptProgram.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

typedef struct {
    uint32_t base, registers[32];
    unsigned reads, writes, orders, events[24], event_count;
    unsigned fail_read, fail_write, fail_order, remove_read;
} FakeMMIO;

static void put32(uint8_t *bytes, unsigned offset, uint32_t value) {
    bytes[offset] = (uint8_t)value; bytes[offset + 1] = (uint8_t)(value >> 8);
    bytes[offset + 2] = (uint8_t)(value >> 16); bytes[offset + 3] = (uint8_t)(value >> 24);
}

static void definition(uint8_t bytes[OD_TBT_INTERRUPT_PROGRAM_BYTES]) {
    const uint32_t address[] = {0x1000, 0x1004, 0x1008, 0x100c, 0x1010};
    const uint32_t mask[] = {0x0f, 0xf0, 0xff, 0x55, 0xaa};
    const uint32_t operation[] = {0, 4, 1, 2, 3};
    const unsigned offset[] = {4, 20, 36, 52, 68};
    memset(bytes, 0, OD_TBT_INTERRUPT_PROGRAM_BYTES); put32(bytes, 0, 0x12345678);
    for (unsigned i = 0; i < 5; ++i) {
        put32(bytes, offset[i], address[i]); put32(bytes, offset[i] + 4, i + 1);
        put32(bytes, offset[i] + 8, mask[i]); put32(bytes, offset[i] + 12, operation[i]);
    }
}

static int fake_read(void *context, uint32_t address, uint32_t *value) {
    FakeMMIO *fake = context;
    ++fake->reads; fake->events[fake->event_count++] = 1;
    if (fake->fail_read == fake->reads) return 1;
    if (fake->remove_read == fake->reads) { *value = UINT32_MAX; return 0; }
    *value = fake->registers[(address - fake->base) / 4]; return 0;
}

static int fake_write(void *context, uint32_t address, uint32_t value) {
    FakeMMIO *fake = context;
    ++fake->writes; fake->events[fake->event_count++] = 2;
    if (fake->fail_write == fake->writes) return 1;
    fake->registers[(address - fake->base) / 4] = value; return 0;
}

static int fake_order(void *context) {
    FakeMMIO *fake = context;
    ++fake->orders; fake->events[fake->event_count++] = 3;
    return fake->fail_order == fake->orders;
}

static ODTBTInterruptMMIO backend(FakeMMIO *fake) {
    ODTBTInterruptMMIO mmio = {fake, fake_read, fake_write, fake_order};
    return mmio;
}

static ODTBTInterruptProgram parsed(void) {
    uint8_t bytes[OD_TBT_INTERRUPT_PROGRAM_BYTES]; ODTBTInterruptProgram program;
    definition(bytes);
    assert(od_tbt_interrupt_parse(bytes, sizeof(bytes), 0x1000, 0x80, &program) == OD_TBT_INTERRUPT_OK);
    assert(program.unknown_word0 == 0x12345678 && program.clear.declared_size == 5);
    return program;
}

int main(void) {
    union {
        uint8_t bytes[OD_TBT_INTERRUPT_PROGRAM_BYTES];
        ODTBTInterruptProgram program;
    } aliased;
    definition(aliased.bytes);
    assert(od_tbt_interrupt_parse(aliased.bytes, sizeof(aliased.bytes), 0x1000, 0x80,
                                  &aliased.program) == OD_TBT_INTERRUPT_OK);
    assert(aliased.program.trigger.address == 0x1000);

    ODTBTInterruptProgram program = parsed();
    FakeMMIO fake = {.base = 0x1000}; ODTBTInterruptMMIO mmio = backend(&fake);
    fake.registers[0] = 0xf3;
    assert(od_tbt_interrupt_apply(&program, OD_TBT_INTERRUPT_TRIGGER, &mmio) == OD_TBT_INTERRUPT_OK);
    assert(fake.registers[0] == 3 && fake.reads == 2 && fake.writes == 1 && fake.orders == 1);
    const unsigned sequence[] = {1, 2, 3, 1};
    assert(!memcmp(fake.events, sequence, sizeof(sequence)));
    fake.registers[2] = 0x0f;
    assert(od_tbt_interrupt_apply(&program, OD_TBT_INTERRUPT_ENABLE, &mmio) == OD_TBT_INTERRUPT_OK);
    assert(fake.registers[2] == 0xff);
    fake.registers[3] = 0x0f;
    assert(od_tbt_interrupt_apply(&program, OD_TBT_INTERRUPT_DISABLE, &mmio) == OD_TBT_INTERRUPT_OK);
    assert(fake.registers[3] == 0x5a);
    assert(od_tbt_interrupt_apply(&program, OD_TBT_INTERRUPT_CLEAR, &mmio) == OD_TBT_INTERRUPT_OK);
    assert(fake.registers[4] == 0xaa);

    fake = (FakeMMIO){.base = 0x1000}; fake.registers[1] = 0xf0; mmio = backend(&fake);
    uint32_t state = UINT32_MAX;
    assert(od_tbt_interrupt_state(&program, &mmio, &state) == OD_TBT_INTERRUPT_OK && state == 0xf0);
    assert(fake.reads == 1 && !fake.writes && !fake.orders);
    program.state.operation = OD_TBT_INTERRUPT_AND; program.state.mask = 0x0f;
    assert(od_tbt_interrupt_state(&program, &mmio, &state) == OD_TBT_INTERRUPT_OK && state == 0);
    program.state.operation = OD_TBT_INTERRUPT_OR; program.state.mask = 1;
    assert(od_tbt_interrupt_state(&program, &mmio, &state) == OD_TBT_INTERRUPT_OK && state == 0xf1);
    program.state.operation = OD_TBT_INTERRUPT_XOR; program.state.mask = 0xff;
    assert(od_tbt_interrupt_state(&program, &mmio, &state) == OD_TBT_INTERRUPT_OK && state == 0x0f);
    program.state.operation = OD_TBT_INTERRUPT_ASSIGN; program.state.mask = 7;
    assert(od_tbt_interrupt_state(&program, &mmio, &state) == OD_TBT_INTERRUPT_OK && state == 7);

    program = parsed(); fake = (FakeMMIO){.base = 0x1000, .remove_read = 1}; mmio = backend(&fake);
    assert(od_tbt_interrupt_apply(&program, OD_TBT_INTERRUPT_TRIGGER, &mmio) == OD_TBT_INTERRUPT_REMOVED && !fake.writes);
    fake = (FakeMMIO){.base = 0x1000, .remove_read = 2}; mmio = backend(&fake);
    assert(od_tbt_interrupt_apply(&program, OD_TBT_INTERRUPT_TRIGGER, &mmio) == OD_TBT_INTERRUPT_REMOVED && fake.writes == 1);
    fake = (FakeMMIO){.base = 0x1000, .fail_order = 1}; mmio = backend(&fake);
    assert(od_tbt_interrupt_apply(&program, OD_TBT_INTERRUPT_TRIGGER, &mmio) == OD_TBT_INTERRUPT_BACKEND && fake.writes == 1);
    fake = (FakeMMIO){.base = 0x1000, .fail_read = 1}; mmio = backend(&fake);
    assert(od_tbt_interrupt_apply(&program, OD_TBT_INTERRUPT_TRIGGER, &mmio) == OD_TBT_INTERRUPT_BACKEND && !fake.writes);
    fake = (FakeMMIO){.base = 0x1000, .fail_write = 1}; mmio = backend(&fake);
    assert(od_tbt_interrupt_apply(&program, OD_TBT_INTERRUPT_TRIGGER, &mmio) == OD_TBT_INTERRUPT_BACKEND &&
           fake.reads == 1 && fake.writes == 1 && !fake.orders);

    uint8_t bytes[OD_TBT_INTERRUPT_PROGRAM_BYTES]; definition(bytes); put32(bytes, 4, 0x1002);
    assert(od_tbt_interrupt_parse(bytes, sizeof(bytes), 0x1000, 0x80, &program) != OD_TBT_INTERRUPT_OK);
    definition(bytes); put32(bytes, 36, 0x1080);
    assert(od_tbt_interrupt_parse(bytes, sizeof(bytes), 0x1000, 0x80, &program) != OD_TBT_INTERRUPT_OK);
    definition(bytes); put32(bytes, 80, 5);
    assert(od_tbt_interrupt_parse(bytes, sizeof(bytes), 0x1000, 0x80, &program) != OD_TBT_INTERRUPT_OK);
    definition(bytes);
    assert(od_tbt_interrupt_parse(bytes, sizeof(bytes) - 1, 0x1000, 0x80, &program) != OD_TBT_INTERRUPT_OK);
    assert(od_tbt_interrupt_parse(bytes, sizeof(bytes), 0x1000, 0, &program) == OD_TBT_INTERRUPT_RANGE);
    program = parsed();
    assert(od_tbt_interrupt_state(&program, NULL, &state) == OD_TBT_INTERRUPT_INVALID_ARGUMENT);
    puts("PASS: bounded interrupt definitions and injected MMIO execution");
}
