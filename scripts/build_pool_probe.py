#!/usr/bin/env python3
"""Build a fixed read-only macOS pool probe; checks never query hardware."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import platform
import subprocess

from desktop_tools import ROOT, environment


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=["aarch64-apple-darwin", "x86_64-apple-darwin"])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--check", action="store_true", help="check argument refusal only; never read a device")
    args = parser.parse_args()
    if platform.system() != "Darwin":
        parser.error("this probe requires macOS")
    native = {"arm64": "aarch64-apple-darwin", "x86_64": "x86_64-apple-darwin"}.get(platform.machine())
    target = args.target or native
    if not target:
        parser.error("unsupported macOS architecture")
    if args.check and target != native:
        parser.error("--check requires the native target")
    architecture = "arm64" if target == "aarch64-apple-darwin" else "x86_64"
    directory = (args.output or ROOT / "artifacts/pool-probe" / architecture).resolve()
    directory.mkdir(parents=True, exist_ok=True)
    executable = directory / "opendrobo-pool-probe"
    sources = [ROOT / "platform/macos/transport" / name for name in
               ["PoolDiagnosticProbe.c", "OpenDroboTransport.c", "CapacityWire.c"]]
    env = environment()
    command = ["xcrun", "clang", "-target", f"{architecture}-apple-macos14.0", "-std=c11",
               "-Wall", "-Wextra", "-Werror", *(str(p) for p in sources),
               "-framework", "IOKit", "-framework", "DiskArbitration", "-framework", "CoreFoundation",
               "-o", str(executable)]
    subprocess.run(command, env=env, check=True)
    subprocess.run(["codesign", "--force", "--sign", "-", str(executable)], env=env, check=True)
    subprocess.run(["codesign", "--verify", "--strict", str(executable)], env=env, check=True)
    checks = []
    if args.check:
        for arguments, expected in [(["--help"], 0), ([], 2), (["--capture-pool-json"], 2),
                                    (["--capture-pool-json", "0"], 2), (["--capture-pool-json", "-1"], 2),
                                    (["--capture-pool-json", "18446744073709551616"], 2),
                                    (["--capture-pool-json", "13x"], 2), (["--write", "1"], 2)]:
            result = subprocess.run([str(executable), *arguments], env=env, capture_output=True, text=True)
            if result.returncode != expected:
                raise RuntimeError(f"argument check failed: {arguments}: {result.returncode}")
            checks.append({"arguments": arguments, "exit_code": result.returncode})
    inputs = sources + [ROOT / "platform/macos/transport" / name for name in
                        ["PoolDiagnostic.h", "CapacityWire.h"]]
    inputs += [ROOT / "platform/macos/include/OpenDroboTransport.h", Path(__file__).resolve()]
    receipt = {"target": target, "minimum_macos": "14.0", "command": command,
               "source_sha256": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs},
               "binary": str(executable), "sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
               "argument_checks": checks, "hardware_queried_by_build": False}
    (directory / "build.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"PASS: {architecture} signed pool probe; {len(checks)} argument checks; no hardware query")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
