#!/usr/bin/env python3
"""Read a macOS app bundle and emit a bounded, read-only release-gate receipt."""
from __future__ import annotations

import argparse
import json
import plistlib
import re
import struct
import subprocess
import sys
from pathlib import Path

from desktop_tools import ROOT
from release_notices import inspect_packaged_notices
from release_metadata import MetadataError, desktop_metadata

CPU_TYPES = {0x01000007: "x86_64", 0x0100000C: "arm64"}
TARGET_ARCHITECTURES = {"aarch64-apple-darwin": "arm64", "x86_64-apple-darwin": "x86_64"}
LC_BUILD_VERSION = 0x32
LC_VERSION_MIN_MACOSX = 0x24


def _version(value: int) -> str:
    return f"{value >> 16}.{(value >> 8) & 0xff}.{value & 0xff}"


def _thin_macho(data: bytes) -> dict[str, str]:
    if len(data) < 28:
        raise ValueError("Mach-O header is truncated")
    magic_le = struct.unpack_from("<I", data)[0]
    if magic_le == 0xfeedfacf:
        endian, header_size = "<", 32
    elif magic_le == 0xfeedface:
        endian, header_size = "<", 28
    elif magic_le == 0xcffaedfe:
        endian, header_size = ">", 32
    elif magic_le == 0xcefaedfe:
        endian, header_size = ">", 28
    else:
        raise ValueError("Not a Mach-O executable")
    if len(data) < header_size:
        raise ValueError("Mach-O header is truncated")
    cpu, _, _, ncmds, sizeofcmds = struct.unpack_from(f"{endian}iiIII", data, 4)
    end = header_size + sizeofcmds
    if end > len(data):
        raise ValueError("Mach-O load commands are truncated")
    minimum = None
    offset = header_size
    for _ in range(ncmds):
        if offset + 8 > end:
            raise ValueError("Mach-O load command is truncated")
        command, size = struct.unpack_from(f"{endian}II", data, offset)
        if size < 8 or offset + size > end:
            raise ValueError("Mach-O load command size is invalid")
        if command in (LC_BUILD_VERSION, LC_VERSION_MIN_MACOSX) and size >= 16:
            minimum = _version(struct.unpack_from(f"{endian}I", data, offset + 12)[0])
        offset += size
    architecture = CPU_TYPES.get(cpu)
    if architecture is None:
        raise ValueError(f"Unsupported Mach-O CPU type {cpu:#x}")
    if minimum is None:
        raise ValueError("Mach-O has no macOS minimum version")
    return {"architecture": architecture, "minimum_macos": minimum}


def macho_metadata(path: Path) -> list[dict[str, str]]:
    """Read thin or FAT Mach-O arch/minimum versions without executing it."""
    data = path.read_bytes()
    magic = struct.unpack_from(">I", data + b"\0\0\0\0")[0]
    if magic not in (0xcafebabe, 0xcafebabf):
        return [_thin_macho(data)]
    is_64 = magic == 0xcafebabf
    if len(data) < 8:
        raise ValueError("FAT Mach-O header is truncated")
    count = struct.unpack_from(">I", data, 4)[0]
    entry_size = 32 if is_64 else 20
    if len(data) < 8 + count * entry_size:
        raise ValueError("FAT Mach-O architecture table is truncated")
    records = []
    for index in range(count):
        offset = 8 + index * entry_size
        cpu = struct.unpack_from(">i", data, offset)[0]
        slice_offset = struct.unpack_from(">Q" if is_64 else ">I", data, offset + (8 if is_64 else 8))[0]
        slice_size = struct.unpack_from(">Q" if is_64 else ">I", data, offset + (16 if is_64 else 12))[0]
        if slice_offset + slice_size > len(data):
            raise ValueError("FAT Mach-O slice is truncated")
        record = _thin_macho(data[slice_offset:slice_offset + slice_size])
        expected = CPU_TYPES.get(cpu)
        if expected != record["architecture"]:
            raise ValueError("FAT Mach-O CPU type disagrees with slice")
        records.append(record)
    return records


def codesign_status(app: Path) -> dict[str, object]:
    """Verify signature and return only public signing gate fields."""
    verified = subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)],
                              text=True, capture_output=True)
    detail = subprocess.run(["codesign", "-dvv", str(app)], text=True, capture_output=True)
    text = detail.stdout + detail.stderr
    team = re.search(r"^TeamIdentifier=(.+)$", text, re.MULTILINE)
    authority = re.search(r"^Authority=(.+)$", text, re.MULTILINE)
    return {"verified": verified.returncode == 0, "team_id": team.group(1) if team else None,
            "authority": authority.group(1) if authority else None}


def _gate(name: str, passed: bool, detail: str) -> dict[str, object]:
    return {"gate": name, "passed": passed, "detail": detail}


def _same_version(value: str, expected: str) -> bool:
    try:
        normalized = tuple(int(part) for part in value.split("."))
        wanted = tuple(int(part) for part in expected.split("."))
    except ValueError:
        return False
    if len(normalized) not in (2, 3) or len(wanted) not in (2, 3):
        return False
    return normalized == wanted + (0,) * (len(normalized) - len(wanted))


def preflight(app: Path, *, root: Path = ROOT, local_structural: bool = False,
              check_stapler: bool = False, check_spctl: bool = False,
              inspect_macho=macho_metadata, inspect_codesign=codesign_status,
              command=subprocess.run) -> dict[str, object]:
    """Check a bundle. Local structural mode never reports distribution ready."""
    app = app.resolve()
    gates: list[dict[str, object]] = []
    try:
        expected = desktop_metadata(root)
        gates.append(_gate("version-parity", True, str(expected["version"])))
    except MetadataError as error:
        expected = None
        gates.append(_gate("version-parity", False, str(error)))
    info_path = app / "Contents/Info.plist"
    build_path = app / "Contents/Resources/BUILD_INFO.json"
    try:
        info = plistlib.loads(info_path.read_bytes())
        build = json.loads(build_path.read_text())
        short_version = info.get("CFBundleShortVersionString")
        runtime_version = info.get("CFBundleVersion")
        gates.append(_gate("bundle-version", expected is not None and short_version == expected["version"],
                           str(short_version)))
        gates.append(_gate("runtime-version", expected is not None and runtime_version == expected["version"],
                           str(runtime_version)))
        bundle_minimum = info.get("LSMinimumSystemVersion")
        gates.append(_gate("bundle-minimum-macos", expected is not None
                           and str(expected["minimum_macos"]) == "14.0"
                           and _same_version(str(bundle_minimum), str(expected["minimum_macos"])),
                           str(bundle_minimum)))
    except (OSError, ValueError, plistlib.InvalidFileException, json.JSONDecodeError) as error:
        info, build = {}, {}
        gates.append(_gate("bundle-metadata", False, str(error)))
        gates.append(_gate("runtime-version", False, "bundle metadata unavailable"))
        gates.append(_gate("bundle-minimum-macos", False, "bundle metadata unavailable"))
    profile = build.get("build_profile")
    gates.append(_gate("release-profile", profile == "release", str(profile)))
    extension_root = app / "Contents/Extensions"
    contains_extension = extension_root.is_dir() and any(extension_root.iterdir())
    research_free = not bool(build.get("fskit_research")) and not contains_extension
    gates.append(_gate("no-research-module", research_free,
                       f"metadata={build.get('fskit_research')} extension={contains_extension}"))
    gates.append(_gate("no-local-vendor-artwork", build.get("artwork_source") in ("generated-fallback", "opendrobo-rendered-v2"),
                       str(build.get("artwork_source"))))
    gates.append(_gate("no-ad-hoc-signature", build.get("signing_identity") not in (None, "ad-hoc"),
                       str(build.get("signing_identity"))))
    declared_target = build.get("target")
    declared_architecture = build.get("architecture")
    target_architecture = TARGET_ARCHITECTURES.get(declared_target)
    declared_minimum = build.get("minimum_macos")
    gates.append(_gate("build-target", target_architecture is not None
                       and declared_architecture == target_architecture
                       and expected is not None
                       and _same_version(str(declared_minimum), str(expected["minimum_macos"])),
                       f"target={declared_target} architecture={declared_architecture} min={declared_minimum}"))
    executable = info.get("CFBundleExecutable")
    try:
        records = inspect_macho(app / "Contents/MacOS" / executable)
        target = expected["minimum_macos"] if expected else None
        arches = sorted({record["architecture"] for record in records})
        minimums = sorted({record["minimum_macos"] for record in records})
        minimum_ok = target is not None and all(_same_version(value, str(target)) for value in minimums)
        architecture_ok = (target_architecture is not None
                           and declared_architecture == target_architecture
                           and set(arches) == {declared_architecture})
        gates.append(_gate("macho", bool(arches) and minimum_ok and architecture_ok,
                           f"arches={','.join(arches)} min={','.join(minimums)} "
                           f"expected_arch={target_architecture} expected_min={target}"))
    except (OSError, TypeError, ValueError) as error:
        gates.append(_gate("macho", False, str(error)))
    try:
        signing = inspect_codesign(app)
        gates.append(_gate("codesign-verify", bool(signing.get("verified")), "verified"))
        team = signing.get("team_id")
        authority = signing.get("authority") or ""
        gates.append(_gate("developer-id-team", bool(team) and authority.startswith("Developer ID Application:"),
                           str(team)))
    except OSError as error:
        gates.append(_gate("codesign-verify", False, str(error)))
        gates.append(_gate("developer-id-team", False, "codesign unavailable"))
    for name, arguments in (("stapler", ["xcrun", "stapler", "validate", str(app)]),
                            ("spctl", ["spctl", "--assess", "--type", "execute", str(app)])):
        requested = check_stapler if name == "stapler" else check_spctl
        if requested:
            try:
                completed = command(arguments, text=True, capture_output=True)
                gates.append(_gate(name, completed.returncode == 0, "checked"))
            except OSError as error:
                gates.append(_gate(name, False, str(error)))
        else:
            # Public readiness must include explicit notarization and Gatekeeper proof.
            gates.append(_gate(name, False, "not requested"))
    notices_ok, notices_detail = inspect_packaged_notices(app / "Contents/Resources", build, root=root)
    gates.append(_gate("third-party-notices", notices_ok, notices_detail))
    ready = not local_structural and all(gate["passed"] for gate in gates)
    return {"schema": "opendrobo-release-preflight-v1", "app": str(app),
            "localStructural": local_structural, "distributionReady": ready, "gates": gates}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path)
    parser.add_argument("--local-structural", action="store_true")
    parser.add_argument("--check-stapler", action="store_true")
    parser.add_argument("--check-spctl", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    receipt = preflight(args.app, local_structural=args.local_structural,
                         check_stapler=args.check_stapler, check_spctl=args.check_spctl)
    encoded = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(encoded)
    else:
        print(encoded, end="")
    return 0 if receipt["distributionReady"] or args.local_structural else 1


if __name__ == "__main__":
    raise SystemExit(main())
