#!/usr/bin/env python3
"""Build an inactive, hardware-disabled ARM64 DriverKit research bundle."""
from __future__ import annotations

import argparse
import json
import plistlib
import subprocess
from pathlib import Path

from desktop_tools import ROOT, environment


def build(output: Path) -> Path:
    env = environment()
    sdk = subprocess.check_output(["xcrun", "--sdk", "driverkit", "--show-sdk-path"],
                                  text=True, env=env).strip()
    driver_root = Path(sdk) / "System/DriverKit"
    frameworks = str(driver_root / "System/Library/Frameworks")
    includes = str(driver_root / "usr/include")
    source = ROOT / "platform/macos/driverkit"
    output.mkdir(parents=True, exist_ok=True)
    generated = output / "generated"
    objects = output / "objects"
    generated.mkdir(exist_ok=True)
    objects.mkdir(exist_ok=True)
    commands = []

    def run(command):
        commands.append(command)
        subprocess.run(command, check=True, env=env)

    # Xcode's Iig.xcspec marks IIG architecture-neutral. ARM64 selection belongs
    # to the subsequent C/C++ compilation, not to this interface-generation step.
    run(["xcrun", "iig", "--def", str(source / "OpenDroboSCSIController.iig"),
         "--header", str(generated / "OpenDroboSCSIController.h"),
         "--impl", str(generated / "OpenDroboSCSIController.iig.cpp"),
         "--deployment-target", "25.0", "--", "-x", "c++", "-std=c++20", "-D__IIG=1",
         "-isysroot", sdk, "-isystem", includes, "-iframework", frameworks])
    common = ["-target", "arm64-apple-driverkit25.0", "-isysroot", sdk,
              "-isystem", includes, "-iframework", frameworks, "-I", str(source),
              "-I", str(generated), "-Wall", "-Wextra", "-Werror"]
    c_files = ["ReadOnlyTaskPolicy", "DmaOwnership", "InterruptProgram", "ControllerTransport",
               "ScsiResponses", "ScsiTaskTable"]
    cpp_files = [source / f"{name}.cpp" for name in
                 ("ReadOnlyTaskAdapter", "DriverKitDma", "DriverKitSession", "OpenDroboSCSIController")]
    cpp_files.append(generated / "OpenDroboSCSIController.iig.cpp")
    object_files = []
    for name in c_files:
        obj = objects / f"{name}.o"
        run(["xcrun", "clang", *common, "-std=c11", "-c", str(source / f"{name}.c"), "-o", str(obj)])
        object_files.append(str(obj))
    for path in cpp_files:
        obj = objects / f"{path.stem}.o"
        run(["xcrun", "clang++", "-isystem", str(driver_root / "usr/include/c++/v1"),
             *common, "-std=c++20", "-fblocks", "-fno-exceptions", "-fno-rtti",
             "-c", str(path), "-o", str(obj)])
        object_files.append(str(obj))
    bundle = output / "OpenDroboStorage.dext"
    contents = bundle / "Contents"
    executable = contents / "MacOS/OpenDroboStorage"
    executable.parent.mkdir(parents=True, exist_ok=True)
    run(["xcrun", "clang++", *common, *object_files, "-framework", "DriverKit",
         "-framework", "PCIDriverKit", "-framework", "SCSIControllerDriverKit",
         "-o", str(executable)])
    with (source / "DriverInfo.plist").open("rb") as stream:
        info = plistlib.load(stream)
    (contents / "Info.plist").write_bytes(plistlib.dumps(info, sort_keys=False))
    receipt = {"sdk": sdk, "target": "arm64-apple-driverkit25.0", "commands": commands,
               "bundle": str(bundle), "hardware_enabled": False, "registered": False,
               "activated": False, "installed": False, "signed_for_distribution": False}
    (output / "build.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"PASS: inactive ARM64 DriverKit bundle at {bundle}; hardware gate closed; no installation or activation")
    return bundle


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "artifacts/driverkit")
    args = parser.parse_args()
    try:
        build(args.output.resolve())
    except subprocess.CalledProcessError as error:
        return error.returncode
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
