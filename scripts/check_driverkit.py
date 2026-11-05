#!/usr/bin/env python3
"""Compile the non-loading DriverKit task adapter and check its pure policy."""
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
        subprocess.run(["xcrun", "clang++", "-target", "arm64-apple-driverkit25.0", "-isysroot", sdk,
                        "-isystem", str(sdk_root / "usr/include"),
                        "-iframework", str(sdk_root / "System/Library/Frameworks"),
                        "-std=c++20", "-fblocks", "-Wall", "-Wextra", "-Werror", "-c",
                        str(source / "ReadOnlyTaskAdapter.cpp"), "-o", f"{temporary}/adapter.o"],
                       check=True, env=env)
        subprocess.run(["xcrun", "clang", "-target", "arm64-apple-driverkit25.0", "-isysroot", sdk,
                        "-isystem", str(sdk_root / "usr/include"),
                        "-std=c11", "-Wall", "-Wextra", "-Werror", "-c",
                        str(source / "ReadOnlyTaskPolicy.c"), "-o", f"{temporary}/policy.o"],
                       check=True, env=env)
        executable = f"{temporary}/policy-checks"
        subprocess.run(["xcrun", "clang", "-std=c11", "-Wall", "-Wextra", "-Werror",
                        "-fsanitize=address,undefined", str(source / "ReadOnlyTaskPolicy.c"),
                        str(source / "Checks.c"), "-o", executable], check=True, env=env)
        subprocess.run([executable], check=True, env=env)
    print("PASS: ARM64 DriverKit SDK adapter and policy objects; no driver bundle, registration, activation, or hardware access")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
