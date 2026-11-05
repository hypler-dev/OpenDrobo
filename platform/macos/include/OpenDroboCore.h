#ifndef OPEN_DROBO_CORE_H
#define OPEN_DROBO_CORE_H
#include <stddef.h>
#include <stdint.h>

/* No ABI function retains a caller pointer. */
typedef struct {
    uint64_t protected_free_bytes;
    uint64_t protected_used_bytes;
    uint64_t protected_total_bytes;
    uint64_t unprotected_total_bytes;
    int64_t captured_at_ms;
} ODCapacity;

enum {
    OD_CONFIRM_BYTE_UNITS = 1,
    OD_CONFIRM_SAME_DEVICE = 2,
    OD_CONFIRM_SINGLE_VOLUME = 4
};

/* Zero means success. page must point to min(length, 60) readable bytes.
 * output must be aligned and writable as one ODCapacity for the call.
 * Input and output may overlap: at most 60 input bytes are copied first.
 * Every decode error clears a valid output; null pointers return errors. */
int32_t od_capacity_decode(const uint8_t *page, size_t length, uint32_t confirmations,
                           int64_t captured_at_ms, int64_t now_ms, ODCapacity *output);
/* capacity must be aligned and readable as one ODCapacity for the call.
 * Validates freshness first, then nonzero and consistent numeric counts. */
int32_t od_capacity_validate(const ODCapacity *capacity, int64_t now_ms);
#endif
