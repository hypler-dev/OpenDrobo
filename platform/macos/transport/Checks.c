#include "CapacityWire.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

static size_t frame(uint8_t *out, uint8_t subpage, size_t body, size_t descriptors) {
    size_t total = 8 + descriptors + 4 + body;
    memset(out, 0, 512);
    out[0] = (uint8_t)((total - 2) >> 8); out[1] = (uint8_t)(total - 2);
    out[6] = (uint8_t)(descriptors >> 8); out[7] = (uint8_t)descriptors;
    size_t start = 8 + descriptors;
    out[start] = 0x7a; out[start + 1] = subpage;
    out[start + 2] = (uint8_t)(body >> 8); out[start + 3] = (uint8_t)body;
    if (subpage == 4) out[start + 4] = 1;
    return total;
}

typedef struct { int step; int capacity_reads; int failed_selector; int multi_lun; int short_reply; int close_failure; } Mock;
static int32_t call(void *context, uint32_t selector, const void *input, size_t input_size, void *output, size_t *output_size) {
    Mock *mock = context;
    ++mock->step;
    if ((int)selector == mock->failed_selector) return -777;
    if (selector == 0 || selector == 1) {
        assert(!input && !input_size && !output && !output_size);
        return selector == 1 && mock->close_failure ? -778 : 0;
    }
    assert(selector == 7 && input_size == 32 && *output_size == 4);
    const ODCapacityRequest *request = input;
    assert(request->cdb_length == 10 && request->length == 512);
    assert(request->cdb[0] == 0x5a && request->cdb[1] == 8 && request->cdb[2] == 0x3a);
    assert(request->cdb[7] == 2 && request->cdb[8] == 0 && request->cdb[9] == 0);
    assert(request->page == 0x3a && request->subpage == request->cdb[3]);
    assert(request->subpage == 4 || request->subpage == 2);
    uint8_t *response = (uint8_t *)(uintptr_t)request->address;
    size_t bytes = frame(response, request->subpage, request->subpage == 4 ? 1 : 56, 0);
    if (request->subpage == 4) response[12] = mock->multi_lun ? 2 : 1;
    else ++mock->capacity_reads;
    *(uint32_t *)output = (uint32_t)bytes;
    if (mock->short_reply) *output_size = 0;
    return 0;
}

int main(void) {
    uint8_t response[512], page[60]; size_t length = 0;
    for (size_t descriptors = 0; descriptors <= 16; descriptors += 8) {
        for (size_t body = 32; body <= 56; body += 24) {
            size_t bytes = frame(response, 2, body, descriptors);
            assert(od_extract_capacity_page(response, bytes, page, sizeof(page), &length) == 0);
            assert(length == body + 4 && page[0] == 0x7a && page[1] == 2);
            for (size_t cut = 0; cut < bytes; ++cut) {
                length = 999;
                assert(od_extract_capacity_page(response, cut, page, sizeof(page), &length) != 0);
                assert(length == 0);
            }
        }
    }
    size_t bytes = frame(response, 2, 32, 0);
    response[9] = 3;
    assert(od_extract_capacity_page(response, bytes, page, 60, &length) != 0);
    frame(response, 2, 32, 0); response[7] = 255;
    assert(od_extract_capacity_page(response, bytes, page, 60, &length) != 0);
    frame(response, 2, 33, 0);
    assert(od_extract_capacity_page(response, bytes + 1, page, 60, &length) != 0);
    Mock mock = {.failed_selector = -1};
    assert(od_run_capacity_query(call, &mock, page, 60, &length) == 0);
    assert(mock.step == 4 && mock.capacity_reads == 1 && length == 60);
    mock = (Mock){.failed_selector = -1, .multi_lun = 1};
    assert(od_run_capacity_query(call, &mock, page, 60, &length) == -10007);
    assert(mock.step == 3 && mock.capacity_reads == 0 && length == 0);
    mock = (Mock){.failed_selector = 7};
    assert(od_run_capacity_query(call, &mock, page, 60, &length) == -777);
    assert(mock.step == 3 && length == 0);
    mock = (Mock){.failed_selector = 0};
    assert(od_run_capacity_query(call, &mock, page, 60, &length) == -777 && mock.step == 1);
    mock = (Mock){.failed_selector = -1, .short_reply = 1};
    assert(od_run_capacity_query(call, &mock, page, 60, &length) != 0 && mock.step == 3);
    mock = (Mock){.failed_selector = -1, .close_failure = 1};
    assert(od_run_capacity_query(call, &mock, page, 60, &length) == -778 && length == 0);
    mock = (Mock){.failed_selector = -1};
    assert(od_run_capacity_query(call, &mock, page, 59, &length) != 0 && mock.step == 0);
    puts("PASS: bounded MODE SENSE framing, LUN scope, read-only command allowlist, and connection cleanup");
}
