#!/usr/bin/env python3
"""Check native icon rendering and a synthetic volume-binding gate without Drobo access."""
from __future__ import annotations

import base64
from pathlib import Path
import struct
import subprocess
import tempfile

from desktop_tools import ROOT, environment


def main() -> int:
    implementation = ROOT / "platform/macos/transport/VolumeIcon.m"
    source = r'''
#include "IMPLEMENTATION"
#include <stdio.h>
#include <assert.h>
/* Synthetic ancestry fixture; no device or real volume discovery is performed. */
int32_t od_list_volumes(ODNativeVolume *volumes, size_t limit, size_t *count) {
    if (!limit) return -10001;
    memset(volumes, 0, sizeof(*volumes));
    volumes[0].registry_id = 42;
    strcpy(volumes[0].mount_path, "/");
    *count = 1;
    return 0;
}
int main(void) {
    char output[16385]; size_t length = 99;
    assert(od_volume_icon_png_base64(0, "/", output, sizeof(output), &length) != 0);
    assert(od_volume_icon_png_base64(43, "/", output, sizeof(output), &length) != 0);
    assert(length == 0 && output[0] == 0);
    assert(od_volume_icon_png_base64(42, "/not-the-fixture", output, sizeof(output), &length) != 0);
    assert(length == 0 && output[0] == 0);
    assert(od_volume_icon_png_base64(42, "/", output, 2, &length) != 0);
    assert(length == 0 && output[0] == 0);
    /* Read only the local system icon; no filesystem data or GUI input. */
    assert(od_volume_icon_png_base64(42, "/", output, sizeof(output), &length) == 0);
    assert(length > 0 && length <= 16384 && output[length] == 0);
    puts(output);
    return 0;
}
'''.replace("IMPLEMENTATION", str(implementation))
    with tempfile.TemporaryDirectory(prefix="opendrobo-volume-icons-") as temporary:
        directory = Path(temporary)
        harness = directory / "checks.m"
        binary = directory / "checks"
        harness.write_text(source)
        env = environment()
        subprocess.run(["xcrun", "clang", "-fobjc-arc", "-Wall", "-Wextra", "-Werror", str(harness), "-framework", "AppKit", "-o", str(binary)], env=env, check=True)
        result = subprocess.run([str(binary)], env=env, text=True, capture_output=True, check=True)
        png = base64.b64decode(result.stdout.strip(), validate=True)
        assert len(png) <= 12288 and png.startswith(b"\x89PNG\r\n\x1a\n")
        assert png[12:16] == b"IHDR" and struct.unpack(">II", png[16:24]) == (48, 48)
    print("PASS: 48px native PNG, bounded output and synthetic registry/path rejection; no Drobo access")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
