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

typedef struct {
    uint64_t registry_id;
    /* Opaque physical-enclosure registry identity; zero when ancestry is unknown. */
    uint64_t physical_registry_group;
    uint64_t total_bytes;
    uint64_t used_bytes;
    uint64_t available_bytes;
    uint32_t read_only;
    uint32_t statistics_available;
    char id[64];
    char name[256];
    char device_name[256];
    char filesystem[32];
    char mount_path[4096];
} ODNativeVolume;

/* Mounted root and /Volumes entries only; cached OS logical statistics.
 * No directory traversal, mount, device open, or protected-pool claim. */
int32_t od_list_volumes(ODNativeVolume *volumes, size_t limit, size_t *count);

/* Returns a 48px AppKit PNG icon for a volume verified by od_list_volumes.
 * The result is base64 without line breaks, bounded to 12 KiB PNG / 16 KiB base64. */
#define OD_VOLUME_ICON_PNG_BASE64_LIMIT 16385
int32_t od_volume_icon_png_base64(uint64_t registry_id, const char *mount_path,
                                  char *output, size_t limit, size_t *length);
/* Count every mounted Drobo filesystem, including custom mount locations. */
int32_t od_mounted_drobo_count(uint32_t *count);

/* Only the original driver's read-only capacity selector is exposed. */
int32_t od_capture_capacity(uint64_t registry_id, uint8_t *page, size_t limit, size_t *length);

/* Resolves the actual mounted volume through Disk Arbitration and I/O Registry.
 * A nonzero result means no unique supported binding was established. */
int32_t od_bind_volume(const char *mount_path, uint64_t *registry_id, uint32_t *volume_count);
int32_t od_device_scope(uint64_t registry_id, char *mount_path, size_t path_limit, uint32_t *volume_count);
#endif
