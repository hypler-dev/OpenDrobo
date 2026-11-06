#include "PoolDiagnostic.h"
#include <inttypes.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

static int parse_identifier(const char *text, uint64_t *identifier) {
    uint64_t value = 0;
    if (!text || !*text) return 0;
    for (const char *at = text; *at; ++at) {
        if (*at < '0' || *at > '9') return 0;
        unsigned digit = (unsigned)(*at - '0');
        if (value > (UINT64_MAX - digit) / 10) return 0;
        value = value * 10 + digit;
    }
    *identifier = value;
    return value != 0;
}

static void print_hex(const uint8_t *bytes, uint32_t count) {
    for (uint32_t i = 0; i < count; ++i) printf("%02x", bytes[i]);
}

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "--help")) {
        puts("Usage: opendrobo-pool-probe --capture-pool-json <registry-id>\n"
             "Reads fixed LUN-info/capacity pages only. Raw pool evidence; units unverified.\n"
             "No per-volume accounting, filesystem override, or device writes.");
        return 0;
    }
    uint64_t identifier = 0;
    if (argc != 3 || strcmp(argv[1], "--capture-pool-json") ||
        !parse_identifier(argv[2], &identifier)) {
        fputs("Usage: opendrobo-pool-probe --capture-pool-json <registry-id>\n", stderr);
        return 2;
    }
    struct timespec captured;
    if (clock_gettime(CLOCK_REALTIME, &captured) || captured.tv_sec < 0 ||
        (uint64_t)captured.tv_sec > (UINT64_MAX - 999) / 1000) return 1;
    uint64_t captured_ms = (uint64_t)captured.tv_sec * 1000 + (uint64_t)captured.tv_nsec / 1000000;
    ODNativePoolDiagnostic record = {0};
    int32_t status = od_capture_pool_diagnostic(identifier, &record);
    if (status) {
        fprintf(stderr, "{\"diagnosticOnly\":true,\"status\":%" PRId32
            ",\"statusHex\":\"0x%08" PRIx32 "\",\"message\":\"Read-only pool diagnostic unavailable.\"}\n",
            status, (uint32_t)status);
        return 1;
    }
    printf("{\"diagnosticOnly\":true,\"perVolumeAccountingAvailable\":false,"
           "\"unitValidation\":\"unverified\",\"deviceId\":\"%" PRIu64
           "\",\"physicalRegistryGroup\":\"%" PRIu64 "\",\"mountedVolumeCount\":%" PRIu32
           ",\"reportedLunCount\":%" PRIu32 ",\"capturedAtMs\":%" PRIu64 ",\"lunPageHex\":\"",
           record.registry_id, record.physical_registry_group, record.mounted_volume_count,
           record.wire.reported_lun_count, captured_ms);
    print_hex(record.wire.lun_page, record.wire.lun_page_length);
    printf("\",\"capacityPageHex\":\"");
    print_hex(record.wire.capacity_page, record.wire.capacity_page_length);
    puts("\",\"message\":\"Raw shared-pool record. Not a verified byte count or per-volume capacity.\"}");
    return 0;
}
