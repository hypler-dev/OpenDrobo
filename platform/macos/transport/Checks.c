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

typedef struct {
    int step, capacity_reads, failed_selector, multi_lun, short_reply, close_failure;
    int bad_subpage, bad_lun_body, bad_capacity_body, bad_descriptors;
    int use_lun_count, reported_lun_count, failed_subpage;
} Mock;
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
    if (request->subpage == mock->failed_subpage) return -779;
    uint8_t *response = (uint8_t *)(uintptr_t)request->address;
    size_t body = request->subpage == 4 ? 1 : 56;
    if (request->subpage == 4 && mock->bad_lun_body) body = 0;
    if (request->subpage == 2 && mock->bad_capacity_body) body = 33;
    size_t bytes = frame(response, request->subpage, body, 0);
    if (mock->bad_subpage) response[9] = 9;
    if (mock->bad_descriptors) response[7] = 255;
    if (request->subpage == 4)
        response[12] = mock->use_lun_count ? mock->reported_lun_count : mock->multi_lun ? 2 : 1;
    else ++mock->capacity_reads;
    *(uint32_t *)output = (uint32_t)bytes;
    if (mock->short_reply) *output_size = 0;
    return 0;
}

static void expect_zeroed(const ODPoolDiagnosticWire *diagnostic) {
    ODPoolDiagnosticWire zero = {0};
    assert(!memcmp(diagnostic, &zero, sizeof(zero)));
}

static void pool_diagnostic_checks(void) {
    ODPoolDiagnosticWire diagnostic;
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(NULL, NULL, &diagnostic) == -10001);
    expect_zeroed(&diagnostic);
    Mock mock = {.failed_selector = -1, .multi_lun = 1};
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == 0);
    assert(mock.step == 4 && mock.capacity_reads == 1);
    assert(diagnostic.reported_lun_count == 2 && diagnostic.lun_page_length == 5);
    assert(diagnostic.capacity_page_length == 60 && diagnostic.lun_page[0] == 0x7a);
    assert(diagnostic.lun_page[1] == 4 && diagnostic.capacity_page[0] == 0x7a);
    assert(diagnostic.capacity_page[1] == 2);

    mock = (Mock){.failed_selector = -1, .use_lun_count = 1, .reported_lun_count = 32};
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == 0);
    assert(diagnostic.reported_lun_count == 32 && mock.step == 4 && mock.capacity_reads == 1);

    mock = (Mock){.failed_selector = -1};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == 0);
    assert(diagnostic.reported_lun_count == 1 && mock.step == 4 && mock.capacity_reads == 1);
    mock = (Mock){.failed_selector = -1, .use_lun_count = 1, .reported_lun_count = 0};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == -10007);
    assert(mock.step == 3 && !mock.capacity_reads); expect_zeroed(&diagnostic);
    mock = (Mock){.failed_selector = -1, .bad_descriptors = 1};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == -10004);
    assert(mock.step == 3 && !mock.capacity_reads); expect_zeroed(&diagnostic);
    mock = (Mock){.failed_selector = -1, .use_lun_count = 1, .reported_lun_count = 33};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == -10007);
    assert(mock.step == 3 && !mock.capacity_reads); expect_zeroed(&diagnostic);
    mock = (Mock){.failed_selector = -1, .bad_lun_body = 1};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == -10007);
    assert(mock.step == 3 && !mock.capacity_reads); expect_zeroed(&diagnostic);
    mock = (Mock){.failed_selector = -1, .bad_subpage = 1};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == -10005);
    assert(mock.step == 3 && !mock.capacity_reads); expect_zeroed(&diagnostic);
    mock = (Mock){.failed_selector = -1, .bad_capacity_body = 1};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == -10005);
    assert(mock.step == 4 && mock.capacity_reads == 1); expect_zeroed(&diagnostic);
    mock = (Mock){.failed_selector = 7};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == -777);
    assert(mock.step == 3); expect_zeroed(&diagnostic);
    mock = (Mock){.failed_selector = -1, .failed_subpage = 2};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == -779);
    assert(mock.step == 4); expect_zeroed(&diagnostic);
    mock = (Mock){.failed_selector = -1, .short_reply = 1};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == -10004);
    assert(mock.step == 3 && !mock.capacity_reads); expect_zeroed(&diagnostic);
    mock = (Mock){.failed_selector = 0};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == -777);
    assert(mock.step == 1); expect_zeroed(&diagnostic);
    mock = (Mock){.failed_selector = -1, .close_failure = 1};
    memset(&diagnostic, 0xa5, sizeof(diagnostic));
    assert(od_run_pool_diagnostic(call, &mock, &diagnostic) == -778);
    assert(mock.step == 4 && mock.capacity_reads == 1); expect_zeroed(&diagnostic);
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
    pool_diagnostic_checks();
    puts("PASS: bounded MODE SENSE framing, LUN scope, read-only command allowlist, and connection cleanup");
}
