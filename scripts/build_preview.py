#!/usr/bin/env python3
"""Build a local native preview without driver installation or provisioning."""
from pathlib import Path
import json
import plistlib
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = ROOT / "third_party/ReDrobo/ReDrobo"
OUTPUT = ROOT / "dist"
BUILD = OUTPUT / "build"
APP = OUTPUT / "OpenDrobo.app"


def run(*arguments):
    subprocess.run([str(argument) for argument in arguments], cwd=ROOT, check=True)


def main():
    BUILD.mkdir(parents=True, exist_ok=True)
    local_sources = sorted((ROOT / "macos").glob("*.swift"))
    local_names = {path.name for path in local_sources}
    dependencies = ["DroboRecords.swift", "DroboStatus.swift", "DroboIOKit.swift",
                    "Redaction.swift", "Enclosure.swift"]
    sources = [UPSTREAM / name for name in dependencies if name not in local_names]
    sources += local_sources
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
        "CFBundleIdentifier": "dev.hypler.OpenDrobo",
        "CFBundleName": "OpenDrobo",
        "CFBundleDisplayName": "OpenDrobo",
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": "0.1.0",
        "CFBundleVersion": "1",
        "LSMinimumSystemVersion": "26.0",
        "NSHighResolutionCapable": True,
        "NSHumanReadableCopyright": "Includes ReDrobo, copyright 2026 fetzu, MIT license. Not affiliated with Drobo.",
    }
    if (ROOT / "artifacts/artwork/DroboOriginalApp.icns").is_file():
        info["CFBundleIconFile"] = "DroboOriginalApp.icns"
    (APP / "Contents/Info.plist").write_bytes(plistlib.dumps(info))
    shutil.copyfile(ROOT / "third_party/ReDrobo/LICENSE", resources / "ReDrobo-LICENSE.txt")
    # Vendor artwork is optional local input and is excluded from source control.
    # Public source builds retain the interface with generated/SF Symbol fallbacks.
    artwork = ROOT / "artifacts/artwork"
    if artwork.exists():
        for asset in artwork.iterdir():
            if asset.is_file() and asset.suffix.lower() in (".icns", ".png"):
                shutil.copyfile(asset, resources / asset.name)
    pins = json.loads((ROOT / "research/upstreams.json").read_text())
    provenance = next(record for record in pins if record["repository"] == "fetzu/ReDrobo")
    (resources / "BUILD_INFO.json").write_text(json.dumps({
        "mode": "native launch preview; no device management or bundled driver",
        "upstream": provenance,
        "architectures": ["arm64", "x86_64"],
        "minimum_macos": "26.0",
    }, indent=2) + "\n")
    # Ad-hoc signing permits local launch. It does not confer distribution trust.
    run("codesign", "--force", "--sign", "-", APP)
    run("codesign", "--verify", "--strict", APP)
    run("ditto", "-c", "-k", "--keepParent", APP, OUTPUT / "OpenDrobo-0.1.0-macos-universal.zip")
    print(APP)


if __name__ == "__main__":
    main()
