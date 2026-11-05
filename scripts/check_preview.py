#!/usr/bin/env python3
"""Run protocol and application-policy checks without hardware or network access."""
from pathlib import Path
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = ROOT / "third_party/ReDrobo/ReDrobo"
BUILD = ROOT / "artifacts/checks"
REPORTS = ROOT / "artifacts/reports"


def main():
    BUILD.mkdir(parents=True, exist_ok=True)
    REPORTS.mkdir(parents=True, exist_ok=True)
    sources = [ROOT / "macos/DroboRecords.swift"]
    sources += [UPSTREAM / name for name in
                ["DroboStatus.swift", "DroboIOKit.swift", "Redaction.swift", "Enclosure.swift"]]
    # The upstream "bit 6 clear" assertion used a snapshot with no status record.
    # Supply a complete zero status for that assertion; missing status is checked
    # separately as unknown in our regression cases.
    upstream_text = (UPSTREAM / "Checks/main.swift").read_text()
    before = 'DroboSnapshot.decode(recs2).isProtected'
    after = 'DroboSnapshot.decode(recs2.merging([0x09: page(0x09, [0, 0, 0, 0])]) { _, new in new }).isProtected'
    if upstream_text.count(before) != 1:
        raise SystemExit("Pinned upstream fixture changed; review the status-record adaptation")
    fixture_directory = BUILD / "upstream"
    fixture_directory.mkdir(exist_ok=True)
    upstream_entry = fixture_directory / "main.swift"
    upstream_entry.write_text(upstream_text.replace(before, after))
    for name, extras, entry in [
        ("upstream", [], upstream_entry),
        ("preview", [ROOT / "macos/LocalDiscovery.swift", ROOT / "macos/PreviewFixture.swift",
                     ROOT / "macos/ReleaseUpdater.swift"], ROOT / "macos/Checks/main.swift"),
    ]:
        executable = BUILD / f"{name}-checks"
        subprocess.run(["xcrun", "swiftc", "-target", f"{platform.machine()}-apple-macos26.0",
                        "-o", str(executable), *map(str, sources + extras + [entry])], check=True)
        with (REPORTS / f"{name}-checks.log").open("w") as output:
            subprocess.run([str(executable)], stdout=output, check=True)
        print(f"PASS: {name} checks ({REPORTS / (name + '-checks.log')})")


if __name__ == "__main__":
    main()
