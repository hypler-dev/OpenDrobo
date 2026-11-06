#import <AppKit/AppKit.h>
#include "../include/OpenDroboTransport.h"
#include <stdlib.h>
#include <limits.h>
#include <stdint.h>
#include <string.h>

enum {
    ODVolumeIconPixels = 48,
    ODVolumeIconMaxPngBytes = 12 * 1024,
    ODVolumeIconMaxBase64Bytes = 16 * 1024,
};

static int32_t od_macos_volume_icon_png_base64(const char *mount_path, char *output,
                                        size_t limit, size_t *length) {
    if (!mount_path || !output || !length || limit < 2) return -10001;
    output[0] = 0;
    *length = 0;
    if (strnlen(mount_path, PATH_MAX) == PATH_MAX || mount_path[0] != '/') return -10001;

    @autoreleasepool {
        @try {
            NSString *path = [[NSString alloc] initWithUTF8String:mount_path];
            if (!path) return -10003;
            NSImage *icon = [[NSWorkspace sharedWorkspace] iconForFile:path];
            if (!icon) return -10003;
            NSBitmapImageRep *bitmap = [[NSBitmapImageRep alloc]
                initWithBitmapDataPlanes:NULL
                pixelsWide:ODVolumeIconPixels
                pixelsHigh:ODVolumeIconPixels
                bitsPerSample:8
                samplesPerPixel:4
                hasAlpha:YES
                isPlanar:NO
                colorSpaceName:NSDeviceRGBColorSpace
                bitmapFormat:0
                bytesPerRow:0
                bitsPerPixel:0];
            if (!bitmap) return -10003;
            NSGraphicsContext *context = [NSGraphicsContext graphicsContextWithBitmapImageRep:bitmap];
            if (!context) return -10003;
            [NSGraphicsContext saveGraphicsState];
            @try {
                [NSGraphicsContext setCurrentContext:context];
                [icon drawInRect:NSMakeRect(0, 0, ODVolumeIconPixels, ODVolumeIconPixels)
                         fromRect:NSZeroRect
                        operation:NSCompositingOperationCopy
                         fraction:1.0
                   respectFlipped:NO
                            hints:nil];
            } @finally {
                [NSGraphicsContext restoreGraphicsState];
            }
            NSData *png = [bitmap representationUsingType:NSBitmapImageFileTypePNG properties:@{}];
            if (!png || png.length == 0 || png.length > ODVolumeIconMaxPngBytes) return -10006;
            NSString *base64 = [png base64EncodedStringWithOptions:0];
            NSData *encoded = [base64 dataUsingEncoding:NSASCIIStringEncoding];
            if (!encoded || encoded.length == 0 || encoded.length > ODVolumeIconMaxBase64Bytes ||
                encoded.length + 1 > limit) return -10006;
            memcpy(output, encoded.bytes, encoded.length);
            output[encoded.length] = 0;
            *length = encoded.length;
            return 0;
        } @catch (NSException *exception) {
            (void)exception;
            output[0] = 0;
            *length = 0;
            return -10003;
        }
    }
}

/* Volume paths arrive only from Rust records produced by od_list_volumes. Re-list and
 * exact-match here so this bridge cannot be used to request an arbitrary file icon. */
static int32_t od_macos_volume_icon_png_base64(const char *mount_path, char *output,
                                                size_t limit, size_t *length);

int32_t od_volume_icon_png_base64(uint64_t registry_id, const char *mount_path,
                                  char *output, size_t limit, size_t *length) {
    if (!registry_id || !mount_path || !output || !length || limit < 2) return -10001;
    output[0] = 0;
    *length = 0;
    if (strnlen(mount_path, PATH_MAX) == PATH_MAX) return -10001;
    ODNativeVolume *volumes = calloc(128, sizeof(*volumes));
    if (!volumes) return -10008;
    size_t count = 0;
    int32_t result = od_list_volumes(volumes, 128, &count);
    if (result || count > 128) {
        free(volumes);
        return result ? result : -10006;
    }
    int matched = 0;
    for (size_t i = 0; i < count; ++i) {
        if (volumes[i].registry_id == registry_id && !strcmp(volumes[i].mount_path, mount_path)) {
            matched = 1;
            break;
        }
    }
    free(volumes);
    if (!matched) return -10003;
    return od_macos_volume_icon_png_base64(mount_path, output, limit, length);
}

