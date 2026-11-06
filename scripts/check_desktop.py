#!/usr/bin/env python3
"""Run offline core, desktop, and macOS bridge checks without accessing hardware."""
from __future__ import annotations

import json
import platform
import subprocess
import sys
import tempfile

from desktop_tools import ROOT, environment


def main() -> int:
    report_dir = ROOT / "artifacts/reports/desktop-checks"
    report_dir.mkdir(parents=True, exist_ok=True)
    receipts = []

    def check(name, command, *, target="desktop-target", cwd=ROOT):
        log = report_dir / f"{name}.log"
        with log.open("w") as output:
            result = subprocess.run(command, cwd=cwd, env=environment(target), stdout=output, stderr=subprocess.STDOUT)
        receipts.append({"name": name, "command": command, "cwd": str(cwd), "exit_code": result.returncode, "log": str(log)})
        (report_dir / "results.json").write_text(json.dumps(receipts, indent=2) + "\n")
        print(f"{'PASS' if result.returncode == 0 else 'FAIL'}: {name} ({log})", flush=True)
        if result.returncode:
            print(log.read_text(), file=sys.stderr)
            raise subprocess.CalledProcessError(result.returncode, command)

    try:
        check("python", [sys.executable, "-m", "unittest", "discover", "-s", "tests"])
        check("core-format", ["cargo", "fmt", "--check", "-p", "opendrobo-core"], target="cargo-target")
        check("core-tests", ["cargo", "test", "--locked", "-p", "opendrobo-core"], target="cargo-target")
        check("core-build", ["cargo", "build", "--locked", "-p", "opendrobo-core"], target="cargo-target")
        check("probe-format", ["cargo", "fmt", "--check", "-p", "opendrobo-probe"], target="cargo-target")
        check("probe-tests", ["cargo", "test", "--locked", "-p", "opendrobo-probe"], target="cargo-target")
        check("statistics-tests", ["node", "--experimental-strip-types", "--test", "tests/statistics.test.mjs"], cwd=ROOT / "desktop")
        check("status-selection-tests", ["node", "--experimental-strip-types", "--test", "tests/status_selection.test.mjs"], cwd=ROOT / "desktop")
        check("drive-indicator-tests", ["node", "--experimental-strip-types", "--test", "tests/drive_indicator.test.mjs"], cwd=ROOT / "desktop")
        check("storage-presentation-tests", ["node", "--experimental-strip-types", "--test", "tests/storage_presentation.test.mjs"], cwd=ROOT / "desktop")
        check("frontend-types", ["npm", "run", "check"], cwd=ROOT / "desktop")
        check("frontend-build", ["npm", "run", "build"], cwd=ROOT / "desktop")
        check("desktop-format", ["cargo", "fmt", "--manifest-path", "desktop/src-tauri/Cargo.toml", "--check"])
        check("desktop-tests", ["cargo", "test", "--locked", "--manifest-path", "desktop/src-tauri/Cargo.toml"])
        if platform.system() == "Darwin":
            check("native-volume-icons", [sys.executable, "scripts/check_volume_icons.py"])
            check("driverkit-contract", [sys.executable, "scripts/check_driverkit.py"])
            with tempfile.TemporaryDirectory(prefix="opendrobo-wire-") as temporary:
                executable = f"{temporary}/wire-checks"
                check("wire-build", ["xcrun", "clang", "-std=c11", "-Wall", "-Wextra", "-Werror", "-fsanitize=address,undefined", "platform/macos/transport/CapacityWire.c", "platform/macos/transport/Checks.c", "-o", executable])
                check("wire-tests", [executable])
            check("native-syntax", ["xcrun", "clang", "-std=c11", "-Wall", "-Wextra", "-Werror", "-fsyntax-only", "platform/macos/transport/OpenDroboTransport.c"])
            check("pool-probe", [sys.executable, "scripts/build_pool_probe.py", "--check"])
            check("fskit-typecheck", [sys.executable, "scripts/build_fskit.py", "--typecheck"])
            check("fskit-tests", [sys.executable, "scripts/build_fskit.py", "--check"])
            check("reference-policy", [sys.executable, "scripts/check_preview.py"])
            check("reference-capacity", [sys.executable, "scripts/check_capacity.py"])
        else:
            print("SKIP: macOS native/FSKit/reference checks require macOS.")
    except subprocess.CalledProcessError:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
