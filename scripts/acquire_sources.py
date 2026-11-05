#!/usr/bin/env python3
"""Acquire the pinned, MIT-licensed protocol reference needed for local builds."""
from pathlib import Path, PurePosixPath
import hashlib
import json
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    records = json.loads((ROOT / "research/upstreams.json").read_text())
    record = next(item for item in records if item["repository"] == "fetzu/ReDrobo")
    destination = ROOT / record["source_directory"]
    if destination.exists():
        raise SystemExit(f"Source directory already exists; preserve or move it before reacquiring: {destination}")
    archive = ROOT / record["archive"]
    archive.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["curl", "-fsSL", "--max-time", "90", "-o", str(archive),
                    record["archive_url"]], check=True)
    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if digest != record["sha256"]:
        raise SystemExit("Pinned source archive checksum mismatch")
    prefix = "ReDrobo-" + record["commit"]
    destination.mkdir(parents=True)
    with tarfile.open(archive) as bundle:
        for member in bundle:
            parts = PurePosixPath(member.name).parts
            if not parts or parts[0] != prefix:
                raise ValueError("Unexpected source archive prefix")
            if len(parts) == 1:
                continue
            member.name = str(PurePosixPath(*parts[1:]))
            bundle.extract(member, destination, filter="data")
    print(f"Verified and extracted {record['repository']} at {record['commit']}")


if __name__ == "__main__":
    main()
