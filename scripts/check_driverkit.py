#!/usr/bin/env python3
"""Check non-loading DriverKit components with SDK objects and mock backends."""
from pathlib import Path
import subprocess
import tempfile

from desktop_tools import ROOT, environment
from build_driverkit import build


def main() -> int:
    env = environment()
    source = ROOT / "platform/macos/driverkit"
    with tempfile.TemporaryDirectory(prefix="opendrobo-driverkit-", dir="/private/tmp") as temporary:
        build(Path(temporary) / "sdk")
        checks = {
            "policy": ["ReadOnlyTaskPolicy.c", "Checks.c"],
            "dma": ["DmaOwnership.c", "DmaChecks.c"],
            "interrupt": ["InterruptProgram.c", "InterruptChecks.c"],
            "controller": ["ReadOnlyTaskPolicy.c", "DmaOwnership.c", "InterruptProgram.c",
                           "ControllerTransport.c", "ScsiDrain.c", "ScsiTaskTable.c", "ControllerChecks.c"],
            "scsi-responses": ["ScsiResponses.c", "ScsiResponseChecks.c"],
            "scsi-tasks": ["ScsiTaskTable.c", "ScsiTaskChecks.c"],
            "scsi-drain": ["ScsiDrain.c", "ScsiDrainChecks.c"],
        }
        for name, files in checks.items():
            executable = f"{temporary}/{name}-checks"
            subprocess.run(["xcrun", "clang", "-std=c11", "-Wall", "-Wextra", "-Werror",
                            "-fsanitize=address,undefined", "-fno-omit-frame-pointer",
                            *(str(source / file) for file in files), "-o", executable],
                           check=True, env=env)
            subprocess.run([executable], check=True, env=env)
    print("PASS: twelve ARM64 objects, IIG/link, seven sanitizer suites; inactive bundle, hardware gate closed, no activation")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
