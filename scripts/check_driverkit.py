#!/usr/bin/env python3
"""Check non-loading DriverKit components with SDK objects and mock backends."""
from pathlib import Path
import subprocess
import tempfile

from desktop_tools import ROOT, environment


def main() -> int:
    env = environment()
    sdk = subprocess.run(["xcrun", "--sdk", "driverkit", "--show-sdk-path"],
                         text=True, capture_output=True, check=True, env=env).stdout.strip()
    sdk_root = Path(sdk) / "System/DriverKit"
    source = ROOT / "platform/macos/driverkit"
    with tempfile.TemporaryDirectory(prefix="opendrobo-driverkit-", dir="/private/tmp") as temporary:
        common = ["-target", "arm64-apple-driverkit25.0", "-isysroot", sdk,
                  "-isystem", str(sdk_root / "usr/include"),
                  "-iframework", str(sdk_root / "System/Library/Frameworks"),
                  "-Wall", "-Wextra", "-Werror", "-c"]
        for name in ("ReadOnlyTaskAdapter", "DriverKitDma", "DriverKitSession"):
            subprocess.run(["xcrun", "clang++", *common, "-std=c++20", "-fblocks",
                            str(source / f"{name}.cpp"), "-o", f"{temporary}/{name}.o"],
                           check=True, env=env)
        for name in ("ReadOnlyTaskPolicy", "DmaOwnership", "InterruptProgram", "ControllerTransport"):
            subprocess.run(["xcrun", "clang", *common, "-std=c11",
                            str(source / f"{name}.c"), "-o", f"{temporary}/{name}.o"],
                           check=True, env=env)
        checks = {
            "policy": ["ReadOnlyTaskPolicy.c", "Checks.c"],
            "dma": ["DmaOwnership.c", "DmaChecks.c"],
            "interrupt": ["InterruptProgram.c", "InterruptChecks.c"],
            "controller": ["ReadOnlyTaskPolicy.c", "DmaOwnership.c", "InterruptProgram.c",
                           "ControllerTransport.c", "ControllerChecks.c"],
        }
        for name, files in checks.items():
            executable = f"{temporary}/{name}-checks"
            subprocess.run(["xcrun", "clang", "-std=c11", "-Wall", "-Wextra", "-Werror",
                            "-fsanitize=address,undefined", "-fno-omit-frame-pointer",
                            *(str(source / file) for file in files), "-o", executable],
                           check=True, env=env)
            subprocess.run([executable], check=True, env=env)
    print("PASS: seven ARM64 DriverKit objects and four sanitizer suites; no driver bundle, activation, or hardware access")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
