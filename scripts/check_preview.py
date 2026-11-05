#!/usr/bin/env python3
"""Run local discovery and application-policy checks without hardware or network access."""
from pathlib import Path
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[1]
BUILD = ROOT / "artifacts/checks"
REPORTS = ROOT / "artifacts/reports"


def main():
    BUILD.mkdir(parents=True, exist_ok=True)
    REPORTS.mkdir(parents=True, exist_ok=True)
    sources = [
        ROOT / "macos/LocalDiscovery.swift",
        ROOT / "macos/ReleaseUpdater.swift",
        ROOT / "macos/Checks/main.swift",
    ]
    executable = BUILD / "preview-checks"
    subprocess.run(["xcrun", "swiftc", "-target", f"{platform.machine()}-apple-macos26.0",
                    "-o", str(executable), *map(str, sources)], check=True)
    report = REPORTS / "preview-checks.log"
    with report.open("w") as output:
        subprocess.run([str(executable)], stdout=output, check=True)
    print(f"PASS: preview checks ({report})")


if __name__ == "__main__":
    main()
