#!/usr/bin/env python3
"""Build a local native preview without driver installation or provisioning."""
from pathlib import Path
import json
import plistlib
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "dist"
BUILD = OUTPUT / "build"
APP = OUTPUT / "OpenDroboSwiftReference.app"


def run(*arguments):
    subprocess.run([str(argument) for argument in arguments], cwd=ROOT, check=True)


def main():
    BUILD.mkdir(parents=True, exist_ok=True)
    sources = sorted((ROOT / "macos").glob("*.swift"))
    for arch in ("arm64", "x86_64"):
        run("xcrun", "swiftc", "-target", f"{arch}-apple-macos26.0", "-O",
            "-o", BUILD / f"OpenDrobo-{arch}", *sources)

    # Replace only this build's generated bundle; the application is never installed.
    if APP.exists():
        shutil.rmtree(APP)
    executable_dir = APP / "Contents/MacOS"
    resources = APP / "Contents/Resources"
    executable_dir.mkdir(parents=True)
    resources.mkdir(parents=True)
    run("lipo", "-create", BUILD / "OpenDrobo-arm64", BUILD / "OpenDrobo-x86_64",
        "-output", executable_dir / "OpenDrobo")
    info = {
        "CFBundleExecutable": "OpenDrobo",
        "CFBundleIdentifier": "dev.hypler.OpenDrobo.swift-reference",
        "CFBundleName": "OpenDrobo",
        "CFBundleDisplayName": "OpenDrobo",
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": "0.1.0",
        "CFBundleVersion": "1",
        "LSMinimumSystemVersion": "26.0",
        "NSHighResolutionCapable": True,
        "NSHumanReadableCopyright": "Copyright 2026 OpenDrobo contributors. Not affiliated with Drobo or its successors.",
    }
    if (ROOT / "artifacts/artwork/DroboOriginalApp.icns").is_file():
        info["CFBundleIconFile"] = "DroboOriginalApp.icns"
    (APP / "Contents/Info.plist").write_bytes(plistlib.dumps(info))
    # Vendor artwork is optional local input and is excluded from source control.
    # Public source builds retain the interface with generated/SF Symbol fallbacks.
    artwork = ROOT / "artifacts/artwork"
    if artwork.exists():
        for asset in artwork.iterdir():
            if asset.is_file() and asset.suffix.lower() in (".icns", ".png"):
                shutil.copyfile(asset, resources / asset.name)
    (resources / "BUILD_INFO.json").write_text(json.dumps({
        "mode": "local native preview; registry discovery and release update policy only; no transport, hardware management, or BeyondRAID support",
        "repository": "https://github.com/hypler-dev/OpenDrobo",
        "architectures": ["arm64", "x86_64"],
        "minimum_macos": "26.0",
    }, indent=2) + "\n")
    # Ad-hoc signing permits local launch. It does not confer distribution trust.
    run("codesign", "--force", "--sign", "-", APP)
    run("codesign", "--verify", "--strict", APP)
    run("ditto", "-c", "-k", "--keepParent", APP, OUTPUT / "OpenDrobo-SwiftReference-0.1.0-macos-universal.zip")
    print(APP)


if __name__ == "__main__":
    main()
