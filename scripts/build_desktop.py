#!/usr/bin/env python3
"""Package a local macOS desktop preview with an optional inactive FSKit extension."""
from __future__ import annotations

import argparse
import hashlib
import json
import platform
import plistlib
import shutil
import sys
from pathlib import Path

from desktop_tools import ROOT, environment, run


SUPPORTED_TARGETS = ("aarch64-apple-darwin", "x86_64-apple-darwin")


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
        if png.is_file() or icns.is_file():
            destination.mkdir(parents=True, exist_ok=True)
            output = destination / png.name
            if png.is_file():
                shutil.copy2(png, output)
            else:
                run(["sips", "-s", "format", "png", icns, "--out", output], env=env)
    for source in (ROOT / "artifacts/artwork").glob("DroboNav-*.png"):
        destination.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination / source.name)


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
    if original_icon.is_file():
        shutil.copy2(original_icon, resources / original_icon.name)
        info_path = app / "Contents/Info.plist"
        info = plistlib.loads(info_path.read_bytes())
        info["CFBundleIconFile"] = original_icon.name
        info_path.write_bytes(plistlib.dumps(info))
    metadata = {
        "version": "0.2.0",
        "mode": "read-only development preview; no driver replacement or mount activation",
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
