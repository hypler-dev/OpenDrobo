#include "CapacityWire.h"
#include <string.h>
#include <stddef.h>

_Static_assert(sizeof(ODCapacityRequest) == 32, "Original driver request ABI is 32 bytes");
_Static_assert(offsetof(ODCapacityRequest, length) == 20, "Transfer length ABI");
_Static_assert(offsetof(ODCapacityRequest, address) == 24, "User buffer address ABI");

int32_t od_extract_capacity_page(const uint8_t *response, size_t realized,
                                uint8_t *page, size_t limit, size_t *length) {
    if (length) *length = 0;
    if (!response || !page || !length || limit < 60) return -10001;
    memset(page, 0, 60);
    if (realized < 8) return -10004;
    size_t declared = ((size_t)response[0] << 8 | response[1]) + 2;
    size_t descriptors = (size_t)response[6] << 8 | response[7];
    if (declared > realized || declared < 12 || descriptors > declared - 8) return -10004;
    size_t start = 8 + descriptors;
    if (start > declared - 4) return -10004;
    if (response[start] != 0x7a || response[start + 1] != 2) return -10005;
    size_t body = (size_t)response[start + 2] << 8 | response[start + 3];
    if (body != 32 && body != 56) return -10005;
    size_t bytes = body + 4;
    if (bytes > declared - start || bytes > limit) return -10004;
    memcpy(page, response + start, bytes);
    *length = bytes;
    return 0;
}

static int32_t extract_page(const uint8_t *response, size_t realized, uint8_t subpage,
                            const uint8_t **page, size_t *length) {
    if (realized < 8) return -10004;
    size_t declared = ((size_t)response[0] << 8 | response[1]) + 2;
    size_t descriptors = (size_t)response[6] << 8 | response[7];
    if (declared > realized || declared < 12 || descriptors > declared - 8) return -10004;
    size_t start = 8 + descriptors;
    if (start > declared - 4) return -10004;
    if (response[start] != 0x7a || response[start + 1] != subpage) return -10005;
    size_t bytes = 4 + ((size_t)response[start + 2] << 8 | response[start + 3]);
    if (bytes > declared - start) return -10004;
    *page = response + start;
    *length = bytes;
    return 0;
}

static int32_t read_page(ODReadCall call, void *context, uint8_t subpage,
                         uint8_t response[512], uint32_t *realized) {
    ODCapacityRequest request = {0};
    request.page = 0x3a;
    request.subpage = subpage;
    request.cdb[0] = 0x5a;
    request.cdb[1] = 8;
    request.cdb[2] = 0x3a;
    request.cdb[3] = subpage;
    request.cdb[7] = 2;
    request.cdb_length = 10;
    request.length = 512;
    request.address = (uint64_t)(uintptr_t)response;
    size_t output_size = sizeof(*realized);
    int32_t status = call(context, 7, &request, sizeof(request), realized, &output_size);
    if (status) return status;
    return output_size == sizeof(*realized) && *realized <= 512 ? 0 : -10004;
}

/* Only MODE SENSE(10) page 0x3a is submitted: LUN info (4), then capacity (2).
 * The LUN count must be one; one mounted volume alone cannot prove this scope.
 * Selectors 0/1 manage the connection; selector 7 is the original data-in ABI.
 * No caller-supplied command, write selector, or block-size override is accepted. */
int32_t od_run_capacity_query(ODReadCall call, void *context,
                             uint8_t *page, size_t limit, size_t *length) {
    if (length) *length = 0;
    if (!call || !page || !length || limit < 60) return -10001;
    memset(page, 0, 60);
    int32_t result = call(context, 0, NULL, 0, NULL, NULL);
    if (result) return result;
    uint8_t response[512] = {0};
    uint32_t realized = 0;
    result = read_page(call, context, 4, response, &realized);
    if (result == 0) {
        const uint8_t *lun_page = NULL;
        size_t lun_length = 0;
        result = extract_page(response, realized, 4, &lun_page, &lun_length);
        if (result == 0 && (lun_length < 5 || lun_page[4] != 1)) result = -10007;
    }
    if (result == 0) {
        memset(response, 0, sizeof(response));
        realized = 0;
        result = read_page(call, context, 2, response, &realized);
    }
    int32_t close_result = call(context, 1, NULL, 0, NULL, NULL);
    if (result) return result;
    if (close_result) return close_result;
    return od_extract_capacity_page(response, realized, page, limit, length);
}

/* Pool diagnostic deliberately differs from od_run_capacity_query: it records
 * a bounded raw LUN page for a pack that reports one through 32 LUNs. It does
 * not claim that any LUN belongs to one mounted volume or that capacity fields
 * have byte units. The established per-volume query retains its one-LUN gate. */
int32_t od_run_pool_diagnostic(ODReadCall call, void *context,
                               ODPoolDiagnosticWire *diagnostic) {
    if (!diagnostic) return -10001;
    memset(diagnostic, 0, sizeof(*diagnostic));
    if (!call) return -10001;
    int32_t result = call(context, 0, NULL, 0, NULL, NULL);
    if (result) return result;

    uint8_t response[512] = {0};
    uint8_t lun[512] = {0};
    uint8_t capacity[60] = {0};
    const uint8_t *lun_page = NULL;
    size_t lun_length = 0, capacity_length = 0;
    uint32_t realized = 0, lun_count = 0;

    result = read_page(call, context, 4, response, &realized);
    if (result == 0) result = extract_page(response, realized, 4, &lun_page, &lun_length);
    if (result == 0 && lun_length >= 5) lun_count = lun_page[4];
    if (result == 0 && (lun_length < 5 || lun_count == 0 || lun_count > 32)) result = -10007;
    if (result == 0) {
        memcpy(lun, lun_page, lun_length);
        memset(response, 0, sizeof(response));
        realized = 0;
        result = read_page(call, context, 2, response, &realized);
    }
    if (result == 0)
        result = od_extract_capacity_page(response, realized, capacity, sizeof(capacity), &capacity_length);

    int32_t close_result = call(context, 1, NULL, 0, NULL, NULL);
    if (result || close_result) {
        memset(diagnostic, 0, sizeof(*diagnostic));
        return result ? result : close_result;
    }
    memcpy(diagnostic->lun_page, lun, lun_length);
    memcpy(diagnostic->capacity_page, capacity, capacity_length);
    diagnostic->reported_lun_count = lun_count;
    diagnostic->lun_page_length = (uint32_t)lun_length;
    diagnostic->capacity_page_length = (uint32_t)capacity_length;
    return 0;
}
