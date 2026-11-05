#ifndef OPEN_DROBO_TRANSPORT_H
#define OPEN_DROBO_TRANSPORT_H
#include <stddef.h>
#include <stdint.h>

typedef struct {
    uint64_t registry_id;
    uint32_t driver_ready;
    uint32_t thunderbolt;
    char name[256];
} ODNativeDevice;

/* Registry reads only. No driver is installed or opened by enumeration. */
int32_t od_list_devices(ODNativeDevice *devices, size_t limit, size_t *count);

/* Only the original driver's read-only capacity selector is exposed. */
int32_t od_capture_capacity(uint64_t registry_id, uint8_t *page, size_t limit, size_t *length);

/* Resolves the actual mounted volume through Disk Arbitration and I/O Registry.
 * A nonzero result means no unique supported binding was established. */
int32_t od_bind_volume(const char *mount_path, uint64_t *registry_id, uint32_t *volume_count);
int32_t od_device_scope(uint64_t registry_id, char *mount_path, size_t path_limit, uint32_t *volume_count);
#endif
