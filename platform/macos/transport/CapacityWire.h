#ifndef OD_CAPACITY_WIRE_H
#define OD_CAPACITY_WIRE_H
#include <stdint.h>
#include <stddef.h>

typedef struct {
    uint8_t page;
    uint8_t subpage;
    uint8_t cdb[16];
    uint8_t cdb_length;
    uint8_t reserved;
    uint32_t length;
    uint64_t address;
} ODCapacityRequest;

typedef int32_t (*ODReadCall)(void *context, uint32_t selector,
                             const void *input, size_t input_size,
                             void *output, size_t *output_size);

/* Diagnostic-only pool record. LUN/capacity data are copied only after both
 * fixed data-in reads and connection close succeed. Values are raw protocol
 * bytes: no byte scale, volume association, or Finder statistic is implied. */
typedef struct {
    uint32_t reported_lun_count;
    uint32_t lun_page_length;
    uint32_t capacity_page_length;
    uint8_t lun_page[512];
    uint8_t capacity_page[60];
} ODPoolDiagnosticWire;

int32_t od_extract_capacity_page(const uint8_t *response, size_t realized,
                                uint8_t *page, size_t limit, size_t *length);
int32_t od_run_capacity_query(ODReadCall call, void *context,
                             uint8_t *page, size_t limit, size_t *length);
int32_t od_run_pool_diagnostic(ODReadCall call, void *context,
                               ODPoolDiagnosticWire *diagnostic);
#endif
