#!/usr/bin/env python3
"""Build, typecheck, or directly check the local read-only FSKit extension."""
from __future__ import annotations

import argparse
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
FSKIT = ROOT / "platform/macos/fskit"
SOURCES = [FSKIT / "ReadOnlyBackend.swift", FSKIT / "OpenDroboFS.swift"]
TRANSPORT = [ROOT / "platform/macos/transport/OpenDroboTransport.c", ROOT / "platform/macos/transport/CapacityWire.c"]
CORE = ROOT / "artifacts/cargo-target/debug/libopendrobo_core.a"

def run(command: list[str], env: dict[str, str]) -> int:
    return subprocess.run(command, cwd=ROOT, env=env).returncode

def sdk_path(env: dict[str, str]) -> str | None:
    result = subprocess.run(["xcrun", "--sdk", "macosx", "--show-sdk-path"], text=True, capture_output=True, env=env)
    if result.returncode or not result.stdout.strip():
        print("FAIL: xcrun could not resolve macOS SDK", file=sys.stderr)
        return None
    return result.stdout.strip()

def swift_command(sdk: str, sources: list[pathlib.Path], objects: list[pathlib.Path], output: pathlib.Path | None, test: bool) -> list[str]:
    command = ["swiftc", "-sdk", sdk, "-target", "arm64-apple-macos26.0", "-F", f"{sdk}/System/Library/Frameworks", "-framework", "FSKit", "-framework", "Foundation", "-framework", "OSLog", "-framework", "IOKit", "-framework", "DiskArbitration", "-framework", "CoreFoundation", "-import-objc-header", str(FSKIT / "OpenDroboFS-Bridging-Header.h")]
    if test: command += ["-D", "OD_FSKIT_TEST", "-parse-as-library"]
    command += [str(source) for source in sources] + [str(obj) for obj in objects]
    if output: command += ["-o", str(output)]
    else: command += ["-typecheck"]
    return command

def compile_transport(sdk: str, temp: pathlib.Path, env: dict[str, str]) -> list[pathlib.Path] | None:
    objects: list[pathlib.Path] = []
    for source in TRANSPORT:
        if not source.is_file(): print(f"FAIL: missing {source.relative_to(ROOT)}", file=sys.stderr); return None
        output = temp / f"{source.stem}.o"
        command = ["xcrun", "--sdk", "macosx", "clang", "-target", "arm64-apple-macos26.0", "-isysroot", sdk, "-c", str(source), "-o", str(output)]
        if run(command, env): return None
        objects.append(output)
    return objects

def main() -> int:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--typecheck", action="store_true")
    group.add_argument("--check", action="store_true")
    args = parser.parse_args()
    env = {**os.environ, "TMPDIR": "/private/tmp"}
    sdk = sdk_path(env)
    if not sdk: return 2
    if args.typecheck:
        return run(swift_command(sdk, SOURCES, [], None, False), env)
    if not CORE.is_file(): print("FAIL: missing Rust static library", file=sys.stderr); return 2
    with tempfile.TemporaryDirectory(dir="/private/tmp", prefix="opendrobo-fskit-") as temporary:
        objects = compile_transport(sdk, pathlib.Path(temporary), env)
        if objects is None: return 2
        if args.check:
            executable = pathlib.Path(temporary) / "checks"
            if run(swift_command(sdk, SOURCES + [FSKIT / "Checks/main.swift"], objects + [CORE], executable, True), env): return 1
            return run([str(executable)], env)
        bundle = ROOT / "dist/OpenDroboFS.appex/Contents"
        (bundle / "MacOS").mkdir(parents=True, exist_ok=True)
        shutil.copy2(FSKIT / "Info.plist", bundle / "Info.plist")
        stale_entitlements = bundle / "OpenDroboFS.entitlements"
        if stale_entitlements.exists(): stale_entitlements.unlink()
        return run(swift_command(sdk, SOURCES, objects + [CORE], bundle / "MacOS/OpenDroboFS", False), env)

if __name__ == "__main__": raise SystemExit(main())
