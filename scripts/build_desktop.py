#!/usr/bin/env python3
"""Package a local macOS desktop preview with the inactive FSKit extension."""
from __future__ import annotations

import hashlib
import json
import platform
import plistlib
import shutil
import sys

from desktop_tools import ROOT, environment, run


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
    if platform.system() != "Darwin":
        print("This packaging helper builds the macOS/FSKit preview. Use the Tauri CLI for other hosts.", file=sys.stderr)
        return 2
    env = environment()
    run(["cargo", "build", "--locked", "-p", "opendrobo-core"], env=environment("cargo-target"))
    run([sys.executable, "scripts/build_fskit.py"], env=env)
    prepare_artwork(env)
    run(["npm", "run", "tauri", "--", "build", "--debug", "--bundles", "app"], env=env, cwd=ROOT / "desktop")
    source = ROOT / "artifacts/desktop-target/debug/bundle/macos/OpenDrobo.app"
    app = ROOT / "dist/OpenDrobo.app"
    if app.exists():
        shutil.rmtree(app)
    shutil.copytree(source, app, symlinks=True)
    extension = app / "Contents/Extensions/OpenDroboFS.appex"
    extension.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(ROOT / "dist/OpenDroboFS.appex", extension, symlinks=True)
    run(["codesign", "--force", "--sign", "-", "--entitlements",
         ROOT / "platform/macos/fskit/OpenDroboFS.entitlements", extension], env=env)
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
        "architecture": platform.machine(),
        "minimum_macos": "26.0",
        "filesystem_extension": "bundled, not installed or activated",
        "repository": "https://github.com/hypler-dev/OpenDrobo",
    }
    (resources / "BUILD_INFO.json").write_text(json.dumps(metadata, indent=2) + "\n")
    run(["codesign", "--force", "--sign", "-", app], env=env)
    run(["codesign", "--verify", "--deep", "--strict", app], env=env)
    archive = ROOT / f"dist/OpenDrobo-0.2.0-preview-macos-{platform.machine()}.zip"
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
