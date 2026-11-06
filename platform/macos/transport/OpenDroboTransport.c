#include "../include/OpenDroboTransport.h"
#include "CapacityWire.h"
#include "PoolDiagnostic.h"
#include <CoreFoundation/CoreFoundation.h>
#include <DiskArbitration/DiskArbitration.h>
#include <IOKit/IOKitLib.h>
#include <mach/mach.h>
#include <sys/mount.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>
#include <limits.h>

static const char *driver_class = "com_TrustedData_driver_VendorSpecificType00";

_Static_assert(sizeof(ODNativeVolume) == 4752, "ODNativeVolume ABI changed");

static uint32_t number_property(io_registry_entry_t entry, const char *key) {
    CFStringRef name = CFStringCreateWithCString(NULL, key, kCFStringEncodingUTF8);
    if (!name) return 0;
    CFTypeRef value = IORegistryEntryCreateCFProperty(entry, name, NULL, 0);
    CFRelease(name);
    uint32_t result = 0;
    if (value && CFGetTypeID(value) == CFDataGetTypeID() && CFDataGetLength(value) >= 4) {
        const UInt8 *bytes = CFDataGetBytePtr(value);
        for (int i = 0; i < 4; ++i) result |= (uint32_t)bytes[i] << (8 * i);
    } else if (value && CFGetTypeID(value) == CFNumberGetTypeID()) {
        CFNumberGetValue(value, kCFNumberSInt32Type, &result);
    }
    if (value) CFRelease(value);
    return result;
}

static void string_property(io_registry_entry_t entry, const char *key, char *output, size_t limit) {
    if (!output || !limit) return;
    output[0] = 0;
    CFStringRef name = CFStringCreateWithCString(NULL, key, kCFStringEncodingUTF8);
    if (!name) return;
    CFTypeRef value = IORegistryEntryCreateCFProperty(entry, name, NULL, 0);
    CFRelease(name);
    if (value && CFGetTypeID(value) == CFStringGetTypeID()) {
        if (!CFStringGetCString(value, output, (CFIndex)limit, kCFStringEncodingUTF8)) output[0] = 0;
    } else if (value && CFGetTypeID(value) == CFDataGetTypeID()) {
        size_t length = (size_t)CFDataGetLength(value);
        if (length >= limit) length = limit - 1;
        memcpy(output, CFDataGetBytePtr(value), length);
        output[length] = 0;
    }
    if (value) CFRelease(value);
    output[limit - 1] = 0;
    size_t end = strlen(output);
    while (end && isspace((unsigned char)output[end - 1])) output[--end] = 0;
}

static int is_pci(io_registry_entry_t entry) {
    uint32_t vendor = number_property(entry, "vendor-id");
    uint32_t device = number_property(entry, "device-id");
    return vendor == 0x4452 && (device == 0x3844 || device == 0x5570);
}

static int own_identity(io_registry_entry_t entry) {
    char vendor[64], product[128];
    string_property(entry, "Vendor Identification", vendor, sizeof(vendor));
    string_property(entry, "Product Identification", product, sizeof(product));
    return !strcasecmp(vendor, "Drobo") ||
        (!strcmp(vendor, "USB 3.0") && !strcmp(product, "Drobo S - Gen 2")) || is_pci(entry);
}

static int known_identity(io_registry_entry_t entry, uint64_t *pci_id) {
    int found = own_identity(entry);
    if (pci_id) *pci_id = 0;
    io_iterator_t parents = 0;
    if (IORegistryEntryCreateIterator(entry, kIOServicePlane,
            kIORegistryIterateParents | kIORegistryIterateRecursively, &parents) != KERN_SUCCESS) return found;
    io_registry_entry_t parent;
    while ((parent = IOIteratorNext(parents))) {
        if (own_identity(parent)) found = 1;
        if (pci_id && is_pci(parent)) IORegistryEntryGetRegistryEntryID(parent, pci_id);
        IOObjectRelease(parent);
    }
    IOObjectRelease(parents);
    return found;
}

static io_service_t driver_for_media(io_service_t media) {
    if (IOObjectConformsTo(media, driver_class) && known_identity(media, NULL)) {
        IOObjectRetain(media);
        return media;
    }
    io_iterator_t parents = 0;
    if (IORegistryEntryCreateIterator(media, kIOServicePlane,
            kIORegistryIterateParents | kIORegistryIterateRecursively, &parents) != KERN_SUCCESS) return 0;
    io_service_t parent, found = 0;
    while ((parent = IOIteratorNext(parents))) {
        if (IOObjectConformsTo(parent, driver_class) && known_identity(parent, NULL)) {
            if (found && found != parent) { IOObjectRelease(found); found = 0; IOObjectRelease(parent); break; }
            if (!found) { IOObjectRetain(parent); found = parent; }
        }
        IOObjectRelease(parent);
    }
    IOObjectRelease(parents);
    return found;
}

static uint64_t physical_key(uint64_t registry_id);

static void driver_name(uint64_t registry_id, char *output, size_t limit) {
    if (!output || !limit) return;
    output[0] = 0;
    if (!registry_id) return;
    io_service_t service = IOServiceGetMatchingService(kIOMainPortDefault,
        IORegistryEntryIDMatching(registry_id));
    if (!service) return;
    if (!IOObjectConformsTo(service, driver_class) || !known_identity(service, NULL)) {
        IOObjectRelease(service);
        return;
    }
    char product[128] = {0};
    string_property(service, "Product Identification", product, sizeof(product));
    IOObjectRelease(service);
    snprintf(output, limit, "Drobo %s", product[0] ? product : "management endpoint");
}

int32_t od_list_devices(ODNativeDevice *devices, size_t limit, size_t *count) {
    if (!count || (!devices && limit)) return -10001;
    *count = 0;
    uint64_t used_pci[128] = {0};
    uint64_t physical_groups[128] = {0};
    size_t used = 0;
    size_t grouped = 0;
    io_iterator_t iterator = 0;
    kern_return_t result = IOServiceGetMatchingServices(kIOMainPortDefault,
        IOServiceMatching(driver_class), &iterator);
    if (result) return result;
    io_service_t entry;
    while ((entry = IOIteratorNext(iterator))) {
        uint64_t pci = 0, identifier = 0;
        if (known_identity(entry, &pci) && IORegistryEntryGetRegistryEntryID(entry, &identifier) == KERN_SUCCESS) {
            uint64_t group = physical_key(identifier);
            int duplicate = 0;
            for (size_t i = 0; group && i < grouped; ++i)
                if (physical_groups[i] == group) duplicate = 1;
            if (duplicate) { IOObjectRelease(entry); continue; }
            if (group && grouped < 128) physical_groups[grouped++] = group;
            if (*count < limit) {
                ODNativeDevice *device = &devices[*count];
                memset(device, 0, sizeof(*device));
                device->registry_id = identifier;
                device->driver_ready = 1;
                device->thunderbolt = pci != 0;
                char product[128] = {0};
                string_property(entry, "Product Identification", product, sizeof(product));
                snprintf(device->name, sizeof(device->name), "Drobo %s", product[0] ? product : "management endpoint");
            }
            ++*count;
            if (pci && used < 128) used_pci[used++] = pci;
        }
        IOObjectRelease(entry);
    }
    IOObjectRelease(iterator);
    result = IOServiceGetMatchingServices(kIOMainPortDefault, IOServiceMatching("IOPCIDevice"), &iterator);
    if (result) return result;
    while ((entry = IOIteratorNext(iterator))) {
        uint64_t identifier = 0;
        if (is_pci(entry) && IORegistryEntryGetRegistryEntryID(entry, &identifier) == KERN_SUCCESS) {
            int duplicate = 0;
            for (size_t i = 0; i < used; ++i) if (used_pci[i] == identifier) duplicate = 1;
            if (!duplicate) {
                if (*count < limit) {
                    ODNativeDevice *device = &devices[*count];
                    memset(device, 0, sizeof(*device));
                    device->registry_id = identifier;
                    device->thunderbolt = 1;
                    snprintf(device->name, sizeof(device->name), "Drobo Thunderbolt PCI endpoint");
                }
                ++*count;
            }
        }
        IOObjectRelease(entry);
    }
    IOObjectRelease(iterator);
    return *count > limit ? -10006 : 0;
}

static int32_t native_call(void *context, uint32_t selector, const void *input,
                           size_t input_size, void *output, size_t *output_size) {
    io_connect_t connection = *(io_connect_t *)context;
    if (selector == 0 || selector == 1)
        return IOConnectCallScalarMethod(connection, selector, NULL, 0, NULL, NULL);
    if (selector != 7) return -10001;
    return IOConnectCallStructMethod(connection, 7, input, input_size, output, output_size);
}

int32_t od_capture_capacity(uint64_t registry_id, uint8_t *page, size_t limit, size_t *length) {
    if (length) *length = 0;
    if (!registry_id || !page || limit < 60 || !length) return -10001;
    memset(page, 0, 60);
    io_service_t service = IOServiceGetMatchingService(kIOMainPortDefault, IORegistryEntryIDMatching(registry_id));
    if (!service) return -10002;
    if (!IOObjectConformsTo(service, driver_class) || !known_identity(service, NULL)) {
        IOObjectRelease(service);
        return -10003;
    }
    io_connect_t connection = 0;
    kern_return_t result = IOServiceOpen(service, mach_task_self(), 0, &connection);
    IOObjectRelease(service);
    if (result) return result;
    int32_t status = od_run_capacity_query(native_call, &connection, page, limit, length);
    IOServiceClose(connection);
    return status;
}

static uint64_t disk_device(DASessionRef session, const char *mount_path) {
    CFURLRef url = CFURLCreateFromFileSystemRepresentation(NULL,
        (const UInt8 *)mount_path, strlen(mount_path), true);
    if (!url) return 0;
    DADiskRef disk = DADiskCreateFromVolumePath(NULL, session, url);
    CFRelease(url);
    if (!disk) return 0;
    io_service_t media = DADiskCopyIOMedia(disk);
    io_service_t driver = media ? driver_for_media(media) : 0;
    if (media) IOObjectRelease(media);
    CFRelease(disk);
    if (!driver) return 0;
    uint64_t identifier = 0;
    IORegistryEntryGetRegistryEntryID(driver, &identifier);
    IOObjectRelease(driver);
    return identifier;
}

static uint64_t physical_key(uint64_t registry_id) {
    io_service_t service = IOServiceGetMatchingService(kIOMainPortDefault, IORegistryEntryIDMatching(registry_id));
    if (!service) return 0;
    uint64_t key = 0;
    if (!IOObjectConformsTo(service, driver_class) || !known_identity(service, &key)) {
        IOObjectRelease(service); return 0;
    }
    if (!key) {
        io_iterator_t parents = 0;
        if (IORegistryEntryCreateIterator(service, kIOServicePlane,
                kIORegistryIterateParents | kIORegistryIterateRecursively, &parents) == KERN_SUCCESS) {
            io_service_t parent;
            while ((parent = IOIteratorNext(parents))) {
                if (!key && IOObjectConformsTo(parent, "IOUSBHostDevice"))
                    IORegistryEntryGetRegistryEntryID(parent, &key);
                IOObjectRelease(parent);
            }
            IOObjectRelease(parents);
        }
    }
    IOObjectRelease(service);
    return key;
}

int32_t od_bind_volume(const char *mount_path, uint64_t *registry_id, uint32_t *volume_count) {
    if (!mount_path || !registry_id || !volume_count) return -10001;
    *registry_id = 0;
    *volume_count = 0;
    struct statfs info;
    char actual[PATH_MAX], mount[PATH_MAX];
    if (!realpath(mount_path, actual) || statfs(actual, &info) ||
        !realpath(info.f_mntonname, mount) || strcmp(actual, mount)) return -10003;
    DASessionRef session = DASessionCreate(NULL);
    if (!session) return -10003;
    uint64_t identifier = disk_device(session, actual);
    if (!identifier) { CFRelease(session); return -10003; }
    CFRelease(session);
    int32_t scope = od_device_scope(identifier, NULL, 0, volume_count);
    if (scope) return scope;
    if (*volume_count != 1) return -10007;
    *registry_id = identifier;
    return 0;
}

int32_t od_device_scope(uint64_t registry_id, char *mount_path, size_t path_limit, uint32_t *volume_count) {
    if (!registry_id || !volume_count || (!mount_path && path_limit)) return -10001;
    *volume_count = 0;
    if (mount_path && path_limit) mount_path[0] = 0;
    uint64_t pack = physical_key(registry_id);
    if (!pack) return -10003;
    int needed = getfsstat(NULL, 0, MNT_NOWAIT);
    if (needed < 0 || needed > 8192) return -10008;
    size_t allocated = (size_t)needed + 16;
    struct statfs *mounts = calloc(allocated, sizeof(*mounts));
    if (!mounts) return -10008;
    int count = getfsstat(mounts, (int)(allocated * sizeof(*mounts)), MNT_NOWAIT);
    DASessionRef session = DASessionCreate(NULL);
    if (count < 0 || (size_t)count >= allocated || !session) {
        if (session) CFRelease(session);
        free(mounts);
        return -10008;
    }
    for (int i = 0; i < count; ++i) {
        uint64_t device = disk_device(session, mounts[i].f_mntonname);
        if (device && physical_key(device) == pack) {
            ++*volume_count;
            if (mount_path && path_limit && *volume_count == 1) {
                if (strlen(mounts[i].f_mntonname) >= path_limit) {
                    CFRelease(session); free(mounts); return -10006;
                }
                memcpy(mount_path, mounts[i].f_mntonname, strlen(mounts[i].f_mntonname) + 1);
            }
        }
    }
    CFRelease(session);
    free(mounts);
    return 0;
}

int32_t od_capture_pool_diagnostic(uint64_t registry_id, ODNativePoolDiagnostic *output) {
    if (!output) return -10001;
    memset(output, 0, sizeof(*output));
    if (!registry_id) return -10001;
    uint64_t group = physical_key(registry_id);
    if (!group) return -10003;
    uint32_t volume_count = 0;
    int32_t result = od_device_scope(registry_id, NULL, 0, &volume_count);
    if (result) return result;

    io_service_t service = IOServiceGetMatchingService(kIOMainPortDefault,
        IORegistryEntryIDMatching(registry_id));
    if (!service) return -10002;
    if (!IOObjectConformsTo(service, driver_class) || !known_identity(service, NULL)) {
        IOObjectRelease(service);
        return -10003;
    }
    io_connect_t connection = 0;
    result = IOServiceOpen(service, mach_task_self(), 0, &connection);
    IOObjectRelease(service);
    if (result) return result;
    ODPoolDiagnosticWire wire = {0};
    result = od_run_pool_diagnostic(native_call, &connection, &wire);
    kern_return_t close_result = IOServiceClose(connection);
    if (result) return result;
    if (close_result) return close_result;

    uint32_t after_count = 0;
    result = od_device_scope(registry_id, NULL, 0, &after_count);
    if (result) return result;
    if (physical_key(registry_id) != group || after_count != volume_count) return -10009;
    output->registry_id = registry_id;
    output->physical_registry_group = group;
    output->mounted_volume_count = volume_count;
    output->wire = wire;
    return 0;
}

int32_t od_list_volumes(ODNativeVolume *volumes, size_t limit, size_t *count) {
    if (!count || (!volumes && limit)) return -10001;
    *count = 0;
    int needed = getfsstat(NULL, 0, MNT_NOWAIT);
    if (needed < 0 || needed > 8192) return -10008;
    size_t allocated = (size_t)needed + 16;
    struct statfs *mounts = calloc(allocated, sizeof(*mounts));
    if (!mounts) return -10008;
    int found = getfsstat(mounts, (int)(allocated * sizeof(*mounts)), MNT_NOWAIT);
    if (found < 0 || (size_t)found >= allocated) { free(mounts); return -10008; }
    DASessionRef session = DASessionCreate(NULL);
    if (!session) { free(mounts); return -10003; }
    for (int i = 0; i < found; ++i) {
        const struct statfs *fs = mounts + i;
        const char *path = fs->f_mntonname;
        if (strcmp(path, "/") && strncmp(path, "/Volumes/", 9)) continue;
        if (*count < limit) {
            ODNativeVolume *volume = volumes + *count;
            memset(volume, 0, sizeof(*volume));
            volume->registry_id = disk_device(session, path);
            volume->physical_registry_group = physical_key(volume->registry_id);
            driver_name(volume->registry_id, volume->device_name, sizeof(volume->device_name));
            volume->read_only = (fs->f_flags & MNT_RDONLY) != 0;
            snprintf(volume->id, sizeof(volume->id), "%08x-%08x",
                (uint32_t)fs->f_fsid.val[0], (uint32_t)fs->f_fsid.val[1]);
            snprintf(volume->name, sizeof(volume->name), "%s",
                !strcmp(path, "/") ? "System volume" : strrchr(path, '/') + 1);
            snprintf(volume->filesystem, sizeof(volume->filesystem), "%s", fs->f_fstypename);
            snprintf(volume->mount_path, sizeof(volume->mount_path), "%s", path);
            uint64_t size = fs->f_bsize;
            if (size && fs->f_bfree <= fs->f_blocks && fs->f_bavail <= fs->f_bfree &&
                fs->f_blocks <= UINT64_MAX / size) {
                volume->statistics_available = 1;
                volume->total_bytes = fs->f_blocks * size;
                volume->used_bytes = (fs->f_blocks - fs->f_bfree) * size;
                volume->available_bytes = fs->f_bavail * size;
            }
        }
        ++*count;
    }
    CFRelease(session);
    free(mounts);
    return *count > limit ? -10006 : 0;
}

int32_t od_mounted_drobo_count(uint32_t *count) {
    if (!count) return -10001;
    *count = 0;
    int needed = getfsstat(NULL, 0, MNT_NOWAIT);
    if (needed < 0 || needed > 8192) return -10008;
    size_t allocated = (size_t)needed + 16;
    struct statfs *mounts = calloc(allocated, sizeof(*mounts));
    if (!mounts) return -10008;
    int found = getfsstat(mounts, (int)(allocated * sizeof(*mounts)), MNT_NOWAIT);
    if (found < 0 || (size_t)found >= allocated) { free(mounts); return -10008; }
    DASessionRef session = DASessionCreate(NULL);
    if (!session) { free(mounts); return -10003; }
    for (int i = 0; i < found; ++i) {
        if (disk_device(session, mounts[i].f_mntonname)) ++*count;
    }
    CFRelease(session);
    free(mounts);
    return 0;
}
