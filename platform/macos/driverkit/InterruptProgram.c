#include "InterruptProgram.h"

#define OD_TBT_INTERRUPT_FORMAT_TAG UINT32_C(0x4f445449)

static uint32_t le32(const uint8_t *bytes) {
    return (uint32_t)bytes[0] | ((uint32_t)bytes[1] << 8) |
           ((uint32_t)bytes[2] << 16) | ((uint32_t)bytes[3] << 24);
}

static int operation_valid(ODTBTInterruptOperator operation) {
    return operation >= OD_TBT_INTERRUPT_AND && operation <= OD_TBT_INTERRUPT_READ;
}

static int address_valid(uint32_t address, uint32_t base, uint32_t length) {
    return length >= 4 && address >= base && address - base <= length - 4;
}

static int register_valid(const ODTBTInterruptRegister *reg, uint32_t base, uint32_t length) {
    return operation_valid(reg->operation) && !(reg->address & 3U) &&
           address_valid(reg->address, base, length);
}

static int program_valid(const ODTBTInterruptProgram *program) {
    return program && program->format_tag == OD_TBT_INTERRUPT_FORMAT_TAG &&
           register_valid(&program->trigger, program->bar0_base, program->bar0_length) &&
           register_valid(&program->state, program->bar0_base, program->bar0_length) &&
           register_valid(&program->enable, program->bar0_base, program->bar0_length) &&
           register_valid(&program->disable, program->bar0_base, program->bar0_length) &&
           register_valid(&program->clear, program->bar0_base, program->bar0_length);
}

static ODTBTInterruptStatus mmio_valid(const ODTBTInterruptMMIO *mmio, int write) {
    if (!mmio || !mmio->read32 || (write && (!mmio->write32 || !mmio->order)))
        return OD_TBT_INTERRUPT_INVALID_ARGUMENT;
    return OD_TBT_INTERRUPT_OK;
}

static uint32_t transform(uint32_t value, const ODTBTInterruptRegister *reg) {
    switch (reg->operation) {
        case OD_TBT_INTERRUPT_AND: return value & reg->mask;
        case OD_TBT_INTERRUPT_OR: return value | reg->mask;
        case OD_TBT_INTERRUPT_XOR: return value ^ reg->mask;
        case OD_TBT_INTERRUPT_ASSIGN: return reg->mask;
        case OD_TBT_INTERRUPT_READ: return value;
    }
    return value;
}

static const ODTBTInterruptRegister *action_register(const ODTBTInterruptProgram *program,
                                                       ODTBTInterruptAction action) {
    switch (action) {
        case OD_TBT_INTERRUPT_TRIGGER: return &program->trigger;
        case OD_TBT_INTERRUPT_ENABLE: return &program->enable;
        case OD_TBT_INTERRUPT_DISABLE: return &program->disable;
        case OD_TBT_INTERRUPT_CLEAR: return &program->clear;
    }
    return NULL;
}

ODTBTInterruptStatus od_tbt_interrupt_parse(const void *bytes, size_t byte_count,
                                             uint32_t bar0_base, uint32_t bar0_length,
                                             ODTBTInterruptProgram *output) {
    if (!bytes || !output) return OD_TBT_INTERRUPT_INVALID_ARGUMENT;
    if (byte_count != OD_TBT_INTERRUPT_PROGRAM_BYTES) return OD_TBT_INTERRUPT_INVALID_FORMAT;
    const uint8_t *raw = (const uint8_t *)bytes;
    ODTBTInterruptProgram decoded = {0};
    ODTBTInterruptRegister *registers[] = {
        &decoded.trigger, &decoded.state, &decoded.enable, &decoded.disable, &decoded.clear
    };
    const unsigned offsets[] = {4, 20, 36, 52, 68};
    decoded.unknown_word0 = le32(raw);
    decoded.bar0_base = bar0_base;
    decoded.bar0_length = bar0_length;
    decoded.format_tag = OD_TBT_INTERRUPT_FORMAT_TAG;
    for (unsigned i = 0; i < sizeof(offsets) / sizeof(offsets[0]); ++i) {
        ODTBTInterruptRegister *reg = registers[i];
        unsigned offset = offsets[i];
        reg->address = le32(raw + offset);
        reg->declared_size = le32(raw + offset + 4);
        reg->mask = le32(raw + offset + 8);
        reg->operation = (ODTBTInterruptOperator)le32(raw + offset + 12);
    }
    if (!program_valid(&decoded))
        return !bar0_length ? OD_TBT_INTERRUPT_RANGE : OD_TBT_INTERRUPT_INVALID_FORMAT;
    *output = decoded;
    return OD_TBT_INTERRUPT_OK;
}

ODTBTInterruptStatus od_tbt_interrupt_state(const ODTBTInterruptProgram *program,
                                             const ODTBTInterruptMMIO *mmio,
                                             uint32_t *state) {
    if (!state || !program_valid(program)) return OD_TBT_INTERRUPT_INVALID_ARGUMENT;
    ODTBTInterruptStatus status = mmio_valid(mmio, 0);
    if (status != OD_TBT_INTERRUPT_OK) return status;
    uint32_t value;
    if (mmio->read32(mmio->context, program->state.address, &value))
        return OD_TBT_INTERRUPT_BACKEND;
    if (value == UINT32_MAX) return OD_TBT_INTERRUPT_REMOVED;
    *state = transform(value, &program->state);
    return OD_TBT_INTERRUPT_OK;
}

ODTBTInterruptStatus od_tbt_interrupt_apply(const ODTBTInterruptProgram *program,
                                             ODTBTInterruptAction action,
                                             const ODTBTInterruptMMIO *mmio) {
    if (!program_valid(program)) return OD_TBT_INTERRUPT_INVALID_ARGUMENT;
    const ODTBTInterruptRegister *reg = action_register(program, action);
    if (!reg) return OD_TBT_INTERRUPT_INVALID_ARGUMENT;
    ODTBTInterruptStatus status = mmio_valid(mmio, 1);
    if (status != OD_TBT_INTERRUPT_OK) return status;
    uint32_t value;
    if (mmio->read32(mmio->context, reg->address, &value)) return OD_TBT_INTERRUPT_BACKEND;
    if (value == UINT32_MAX) return OD_TBT_INTERRUPT_REMOVED;
    if (reg->operation == OD_TBT_INTERRUPT_READ) return OD_TBT_INTERRUPT_OK;
    if (mmio->write32(mmio->context, reg->address, transform(value, reg)))
        return OD_TBT_INTERRUPT_BACKEND;
    if (mmio->order(mmio->context)) return OD_TBT_INTERRUPT_BACKEND;
    if (mmio->read32(mmio->context, reg->address, &value)) return OD_TBT_INTERRUPT_BACKEND;
    return value == UINT32_MAX ? OD_TBT_INTERRUPT_REMOVED : OD_TBT_INTERRUPT_OK;
}
