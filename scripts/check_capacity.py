#!/usr/bin/env python3
"""Check the isolated capacity decoder and FSKit statistics prototype offline."""
from pathlib import Path
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[1]
BUILD = ROOT / "artifacts/checks"
REPORTS = ROOT / "artifacts/reports"


def main():
    BUILD.mkdir(parents=True, exist_ok=True)
    REPORTS.mkdir(parents=True, exist_ok=True)
    sources = [ROOT / "experiments/capacity" / name for name in
               ("CapacityRecord.swift", "FinderCapacityPreview.swift", "main.swift")]
    executable = BUILD / "capacity-checks"
    subprocess.run(["xcrun", "swiftc", "-target", f"{platform.machine()}-apple-macos26.0",
                    "-o", str(executable), *map(str, sources)], check=True)
    report = REPORTS / "capacity-checks.log"
    with report.open("w") as output:
        subprocess.run([str(executable)], stdout=output, check=True)
    print(f"PASS: isolated capacity checks ({report})")


if __name__ == "__main__":
    main()
