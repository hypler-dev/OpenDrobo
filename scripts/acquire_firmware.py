#!/usr/bin/env python3
"""Download archived firmware for static research; never execute or install it."""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path, PurePosixPath
import hashlib
import json
import stat
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SOURCES = {
    "5D": "https://github.com/dhomas1/DroboApps/releases/download/5DFirmware/Drobo-5D-firmware-4.2.3.zip",
    "5D3": "https://github.com/dhomas1/DroboApps/releases/download/5D3Firmware/Drobo-5D3-firmware-4.2.3.zip",
}


def extract_regular_files(archive_path, destination):
    destination = destination.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive_path) as archive:
        entries = []
        seen = set()
        total = 0
        for entry in archive.infolist():
            parts = PurePosixPath(entry.filename)
            mode = entry.external_attr >> 16
            if (parts.is_absolute() or ".." in parts.parts or "\\" in entry.filename
                    or not parts.parts or ":" in parts.parts[0]
                    or (stat.S_IFMT(mode) not in (0, stat.S_IFREG, stat.S_IFDIR))):
                raise ValueError(f"Unsafe archive entry: {entry.filename!r}")
            target = destination.joinpath(*parts.parts)
            if not target.resolve().is_relative_to(destination) or target in seen:
                raise ValueError(f"Archive path escapes or repeats: {entry.filename!r}")
            if target.exists() or target.is_symlink():
                raise FileExistsError(f"Refusing to overwrite: {target}")
            total += entry.file_size
            if entry.file_size > 1_073_741_824 or total > 2_147_483_648:
                raise ValueError("Firmware archive exceeds extraction bounds")
            seen.add(target)
            entries.append((entry, target))
        for entry, target in entries:
            if entry.is_dir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            with archive.open(entry) as source, target.open("xb") as output:
                while block := source.read(1_048_576):
                    output.write(block)
    return [entry.filename for entry, _ in entries]


def acquire(item):
    model, url = item
    archive = ROOT / "artifacts/vendor" / url.rsplit("/", 1)[1]
    archive.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["curl", "-fsSL", "--max-time", "90", "-o", str(archive), url], check=True)
    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    destination = ROOT / "artifacts/vendor/firmware" / model
    files = extract_regular_files(archive, destination)
    return {"model": model, "url": url, "sha256": digest,
            "archive": str(archive.relative_to(ROOT)), "files": files,
            "authenticity": "Community archive; no vendor-published digest verified"}


if __name__ == "__main__":
    with ThreadPoolExecutor(max_workers=2) as pool:
        records = list(pool.map(acquire, SOURCES.items()))
    report = ROOT / "artifacts/reports/firmware-acquired.json"
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(records, indent=2) + "\n")
    print(json.dumps(records, indent=2))
