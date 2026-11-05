#ifndef OPENDROBO_INTERRUPT_PROGRAM_H
#define OPENDROBO_INTERRUPT_PROGRAM_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum { OD_TBT_INTERRUPT_PROGRAM_BYTES = 84 };

typedef enum {
    OD_TBT_INTERRUPT_OK = 0,
    OD_TBT_INTERRUPT_INVALID_ARGUMENT = 1,
    OD_TBT_INTERRUPT_INVALID_FORMAT = 2,
    OD_TBT_INTERRUPT_RANGE = 3,
    OD_TBT_INTERRUPT_BACKEND = 4,
    OD_TBT_INTERRUPT_REMOVED = 5
} ODTBTInterruptStatus;

typedef enum {
    OD_TBT_INTERRUPT_AND = 0,
    OD_TBT_INTERRUPT_OR = 1,
    OD_TBT_INTERRUPT_XOR = 2,
    OD_TBT_INTERRUPT_ASSIGN = 3,
    OD_TBT_INTERRUPT_READ = 4
} ODTBTInterruptOperator;

typedef enum {
    OD_TBT_INTERRUPT_TRIGGER = 0,
    OD_TBT_INTERRUPT_ENABLE = 1,
    OD_TBT_INTERRUPT_DISABLE = 2,
    OD_TBT_INTERRUPT_CLEAR = 3
} ODTBTInterruptAction;

typedef struct {
    uint32_t address;
    /* Preserved from the definition. Hardware access remains exactly u32. */
    uint32_t declared_size;
    uint32_t mask;
    ODTBTInterruptOperator operation;
} ODTBTInterruptRegister;

typedef struct {
    uint32_t unknown_word0;
    uint32_t bar0_base;
    uint32_t bar0_length;
    ODTBTInterruptRegister trigger;
    ODTBTInterruptRegister state;
    ODTBTInterruptRegister enable;
    ODTBTInterruptRegister disable;
    ODTBTInterruptRegister clear;
    uint32_t format_tag;
} ODTBTInterruptProgram;

/* All callbacks return zero on success and nonzero on a backend failure. */
typedef int (*ODTBTInterruptRead32)(void *context, uint32_t address, uint32_t *value);
typedef int (*ODTBTInterruptWrite32)(void *context, uint32_t address, uint32_t value);
typedef int (*ODTBTInterruptOrder)(void *context);

typedef struct {
    void *context;
    ODTBTInterruptRead32 read32;
    ODTBTInterruptWrite32 write32;
    ODTBTInterruptOrder order;
} ODTBTInterruptMMIO;

/* Decodes exactly the observed 84-byte little-endian definition. No pointer
 * into bytes is retained. All five addresses must name aligned u32 accesses
 * contained in [bar0_base, bar0_base + bar0_length). The output is untouched
 * on error, so input/output overlap is safe. */
ODTBTInterruptStatus od_tbt_interrupt_parse(const void *bytes, size_t byte_count,
                                             uint32_t bar0_base, uint32_t bar0_length,
                                             ODTBTInterruptProgram *output);

/* Reads state once and computes a local result. No write or order callback.
 * A zero result is ready; nonzero is pending. */
ODTBTInterruptStatus od_tbt_interrupt_state(const ODTBTInterruptProgram *program,
                                             const ODTBTInterruptMMIO *mmio,
                                             uint32_t *state);

/* Applies one selected action. For non-READ: read32, write32, order, read32.
 * UINT32_MAX from either read reports removal. Failure stops the action; the
 * caller owns cross-action serialization and must not continue after failure. */
ODTBTInterruptStatus od_tbt_interrupt_apply(const ODTBTInterruptProgram *program,
                                             ODTBTInterruptAction action,
                                             const ODTBTInterruptMMIO *mmio);

#ifdef __cplusplus
}
#endif

#endif
