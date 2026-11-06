#!/usr/bin/env python3
"""Package a local macOS desktop preview with an optional inactive FSKit extension."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import plistlib
import shutil
import stat
import struct
import sys
import tempfile
import zlib
from pathlib import Path

from desktop_tools import ROOT, environment, run


SUPPORTED_TARGETS = ("aarch64-apple-darwin", "x86_64-apple-darwin")
MAX_ARTWORK_BYTES = 4 * 1024 * 1024


def artwork_bytes(path: Path, kind: str) -> bytes | None:
    """Read optional artwork through pinned no-follow descriptors, never a link."""
    path = path.absolute()
    if ".." in path.parts:
        raise ValueError("Artwork path must be canonical")
    directory = os.open(path.anchor, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for part in path.parts[1:-1]:
            next_directory = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                                     dir_fd=directory)
            os.close(directory)
            directory = next_directory
        try:
            descriptor = os.open(path.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK,
                                 dir_fd=directory)
        except FileNotFoundError:
            return None
        with os.fdopen(descriptor, "rb") as source:
            metadata = os.fstat(source.fileno())
            if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
                raise ValueError("Artwork must be a single-link regular input file")
            if metadata.st_size > MAX_ARTWORK_BYTES:
                raise ValueError("Artwork exceeds the size limit")
            data = source.read(MAX_ARTWORK_BYTES + 1)
    except FileNotFoundError:
        return None
    finally:
        os.close(directory)
    if len(data) > MAX_ARTWORK_BYTES:
        raise ValueError("Artwork exceeds the size limit")
    if kind == "png":
        if len(data) < 45 or data[:8] != b"\x89PNG\r\n\x1a\n":
            raise ValueError("Artwork is not a PNG image")
        position = 8
        first = True
        has_data = False
        while position < len(data):
            if len(data) - position < 12:
                raise ValueError("Artwork PNG is truncated")
            length = int.from_bytes(data[position:position + 4], "big")
            end = position + 12 + length
            if end > len(data):
                raise ValueError("Artwork PNG is truncated")
            chunk = data[position + 4:position + 8]
            payload = data[position + 8:end - 4]
            if zlib.crc32(chunk + payload) != int.from_bytes(data[end - 4:end], "big"):
                raise ValueError("Artwork PNG checksum is invalid")
            if first:
                if chunk != b"IHDR" or length != 13:
                    raise ValueError("Artwork PNG header is invalid")
                width, height = struct.unpack(">II", payload[:8])
                if not 0 < width <= 4096 or not 0 < height <= 4096:
                    raise ValueError("Artwork PNG dimensions exceed the limit")
                depths = {0: (1, 2, 4, 8, 16), 2: (8, 16), 3: (1, 2, 4, 8),
                          4: (8, 16), 6: (8, 16)}
                if (payload[8] not in depths.get(payload[9], ())
                        or payload[10:12] != b"\0\0" or payload[12] not in (0, 1)):
                    raise ValueError("Artwork PNG encoding is invalid")
                first = False
            elif chunk == b"IHDR":
                raise ValueError("Artwork PNG repeats its header")
            if chunk == b"IDAT" and length:
                has_data = True
            if chunk == b"IEND":
                if not has_data or length != 0 or end != len(data):
                    raise ValueError("Artwork PNG has no image data or contains trailing data")
                return data
            position = end
        raise ValueError("Artwork PNG is incomplete")
    if kind == "icns":
        if len(data) < 8 or data[:4] != b"icns" or int.from_bytes(data[4:8], "big") != len(data):
            raise ValueError("Artwork is not an ICNS file")
        return data
    raise ValueError("Unsupported artwork format")


def native_target() -> str | None:
    machine = platform.machine().lower()
    return {"arm64": "aarch64-apple-darwin", "aarch64": "aarch64-apple-darwin",
            "x86_64": "x86_64-apple-darwin"}.get(machine)


def target_architecture(target: str) -> str:
    return "arm64" if target == "aarch64-apple-darwin" else "x86_64"


def prepare_artwork(env: dict[str, str]) -> None:
    destination = ROOT / "desktop/public/local-artwork"
    # This directory contains only optional, generated, ignored vendor artwork.
    if destination.exists():
        shutil.rmtree(destination)
    for name in ("DroboMiniArtwork", "Drobo5DArtwork", "DroboProArtwork", "DroboB800Artwork"):
        png = ROOT / "artifacts/artwork" / f"{name}.png"
        icns = png.with_suffix(".icns")
        data = artwork_bytes(png, "png")
        icon = artwork_bytes(icns, "icns") if data is None else None
        if data is not None or icon is not None:
            destination.mkdir(parents=True, exist_ok=True)
            output = destination / png.name
            if data is not None:
                output.write_bytes(data)
            else:
                with tempfile.TemporaryDirectory(prefix="opendrobo-art-") as staging:
                    staging_path = Path(staging).resolve(strict=True)
                    source = staging_path / "source.icns"
                    assert icon is not None
                    source.write_bytes(icon)
                    converted = staging_path / "converted.png"
                    run(["sips", "-s", "format", "png", source, "--out", converted], env=env)
                    converted_data = artwork_bytes(converted, "png")
                    if converted_data is None:
                        raise ValueError("Artwork conversion produced no PNG image")
                    output.write_bytes(converted_data)
    for source in (ROOT / "artifacts/artwork").glob("DroboNav-*.png"):
        data = artwork_bytes(source, "png")
        if data is not None:
            destination.mkdir(parents=True, exist_ok=True)
            (destination / source.name).write_bytes(data)
    # An optional, locally extracted neutral front view supports drive selection.
    # Vendor artwork remains ignored and is never required by a public checkout.
    front = ROOT / "artifacts/artwork/Drobo5DFront.png"
    data = artwork_bytes(front, "png")
    if data is not None:
        destination.mkdir(parents=True, exist_ok=True)
        (destination / front.name).write_bytes(data)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=SUPPORTED_TARGETS,
                        help="macOS Rust target; defaults to this Mac's native target")
    parser.add_argument("--without-fskit", action="store_true",
                        help="omit the ARM64 macOS 26 FSKit research extension")
    parser.add_argument("--output", type=Path, default=ROOT / "dist",
                        help="directory receiving OpenDrobo.app and its local archive")
    args = parser.parse_args()
    if platform.system() != "Darwin":
        print("This packaging helper builds the macOS/FSKit preview. Use the Tauri CLI for other hosts.", file=sys.stderr)
        return 2

    native = native_target()
    if native is None:
        print(f"Unsupported native macOS architecture: {platform.machine()}", file=sys.stderr)
        return 2
    target = args.target or native
    include_fskit = not args.without_fskit
    if include_fskit and (target != native or target != "aarch64-apple-darwin"):
        print("FSKit preview requires the native aarch64-apple-darwin target. "
              "Use --without-fskit for an Intel or cross-target shared-UI preview.", file=sys.stderr)
        return 2

    env = environment()
    env["MACOSX_DEPLOYMENT_TARGET"] = "14.0"
    output = args.output.resolve()
    if include_fskit:
        run(["cargo", "build", "--locked", "-p", "opendrobo-core"], env=environment("cargo-target"))
        run([sys.executable, "scripts/build_fskit.py"], env=env)
    prepare_artwork(env)
    tauri_command = ["npm", "run", "tauri", "--", "build", "--debug", "--bundles", "app"]
    if args.target:
        tauri_command += ["--target", target]
    run(tauri_command, env=env, cwd=ROOT / "desktop")
    target_path = Path(target) / "debug" if args.target else Path("debug")
    source = ROOT / "artifacts/desktop-target" / target_path / "bundle/macos/OpenDrobo.app"
    app = output / "OpenDrobo.app"
    if app.exists():
        shutil.rmtree(app)
    output.mkdir(parents=True, exist_ok=True)
    shutil.copytree(source, app, symlinks=True)
    extension = app / "Contents/Extensions/OpenDroboFS.appex"
    if include_fskit:
        extension.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(ROOT / "dist/OpenDroboFS.appex", extension, symlinks=True)
        run(["codesign", "--force", "--sign", "-", "--entitlements",
             ROOT / "platform/macos/fskit/OpenDroboFS.entitlements", extension], env=env)
    elif extension.exists():
        shutil.rmtree(extension)
    resources = app / "Contents/Resources"
    resources.mkdir(exist_ok=True)
    original_icon = ROOT / "artifacts/artwork/DroboOriginalApp.icns"
    icon = artwork_bytes(original_icon, "icns")
    if icon is not None:
        (resources / original_icon.name).write_bytes(icon)
        info_path = app / "Contents/Info.plist"
        info = plistlib.loads(info_path.read_bytes())
        info["CFBundleIconFile"] = original_icon.name
        info_path.write_bytes(plistlib.dumps(info))
    metadata = {
        "version": "0.2.0",
        "mode": "development preview; read-only monitoring and explicit GUI controls; no driver replacement or mount activation",
        "architecture": target_architecture(target),
        "target": target,
        "minimum_macos": "14.0",
        "filesystem_extension": "bundled, not installed or activated" if include_fskit else "not bundled",
        "repository": "https://github.com/hypler-dev/OpenDrobo",
    }
    (resources / "BUILD_INFO.json").write_text(json.dumps(metadata, indent=2) + "\n")
    run(["codesign", "--force", "--sign", "-", app], env=env)
    run(["codesign", "--verify", "--deep", "--strict", app], env=env)
    archive = output / f"OpenDrobo-0.2.0-preview-macos-{target_architecture(target)}.zip"
    if archive.exists():
        archive.unlink()
    run(["ditto", "-c", "-k", "--keepParent", app, archive], env=env)
    with archive.open("rb") as source_file:
        digest = hashlib.file_digest(source_file, "sha256").hexdigest()
    receipt = {**metadata, "app": str(app), "archive": str(archive), "sha256": digest}
    report = ROOT / "artifacts/reports/desktop-build.json"
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"PASS: {app}\nSHA-256: {digest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
