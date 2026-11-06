#!/usr/bin/env python3
"""Generate offline, non-executing spare-unit lab manifests and test plans."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import stat
import sys
import time
from typing import Any

MAX_INPUT_BYTES = 1_048_576
SCHEMA_VERSION = 1
PLAN_FILE = "spare-unit-lab-plan.json"
MANIFEST_FILE = "spare-unit-lab-manifest.json"

TEMPLATE: dict[str, Any] = {
    "schemaVersion": SCHEMA_VERSION,
    "target": {
        "model": "pending",
        "firmware": "pending",
        "operatingSystem": "pending",
        "transport": "pending",
    },
    "unit": {"opaqueFingerprint": "pending"},
    "disposableMemberDisks": ["pending"],
    "exclusions": {
        "protectedUnitFingerprints": [],
        "homeUnitFingerprints": [],
        "protectedDiskFingerprints": [],
    },
    "unknown": {"driveOrder": "pending", "onDiskFormatRevision": "pending"},
}


def fail(message: str) -> ValueError:
    return ValueError(message)


def stable_fingerprint(value: Any, name: str) -> str:
    if not isinstance(value, str) or re.fullmatch(r"sha256:[0-9a-f]{64}", value) is None:
        raise fail(f"{name} must be a sha256: fingerprint with 64 lowercase hexadecimal digits")
    return value


def string_list(value: Any, name: str) -> list[str]:
    if not isinstance(value, list):
        raise fail(f"{name} must be a list")
    result = [stable_fingerprint(item, name) for item in value]
    if len(set(result)) != len(result):
        raise fail(f"{name} contains duplicate fingerprints")
    return sorted(result)


def validate_manifest(manifest: Any) -> dict[str, Any]:
    if (
        not isinstance(manifest, dict)
        or not isinstance(manifest.get("schemaVersion"), int)
        or isinstance(manifest.get("schemaVersion"), bool)
        or manifest.get("schemaVersion") != SCHEMA_VERSION
    ):
        raise fail("unsupported or missing manifest schemaVersion")
    if any(key in manifest for key in ("registryId", "registry_id", "deviceId")):
        raise fail("manifest must not use transient registry IDs as hardware identity")
    target = manifest.get("target")
    if not isinstance(target, dict):
        raise fail("target must be an object")
    for key in ("model", "firmware", "operatingSystem", "transport"):
        value = target.get(key)
        if not isinstance(value, str) or not value or value != value.strip() or value.lower() == "pending":
            raise fail(f"target.{key} must be a confirmed non-whitespace value")
    unit = manifest.get("unit")
    if not isinstance(unit, dict):
        raise fail("unit must be an object")
    unit_fingerprint = stable_fingerprint(unit.get("opaqueFingerprint"), "unit.opaqueFingerprint")
    disks = string_list(manifest.get("disposableMemberDisks"), "disposableMemberDisks")
    if not disks:
        raise fail("disposableMemberDisks must identify at least one disposable disk")
    exclusions = manifest.get("exclusions")
    if not isinstance(exclusions, dict):
        raise fail("exclusions must be an object")
    protected = string_list(exclusions.get("protectedUnitFingerprints"), "protectedUnitFingerprints")
    home = string_list(exclusions.get("homeUnitFingerprints"), "homeUnitFingerprints")
    protected_disks = string_list(exclusions.get("protectedDiskFingerprints"), "protectedDiskFingerprints")
    if unit_fingerprint in set(protected) | set(home):
        raise fail("target unit is protected or home-unit excluded")
    if set(disks) & set(protected_disks):
        raise fail("disposableMemberDisks includes a protected disk")
    return {
        "schemaVersion": SCHEMA_VERSION,
        "target": {key: target[key] for key in ("model", "firmware", "operatingSystem", "transport")},
        "unitFingerprint": unit_fingerprint,
        "disposableMemberDisks": disks,
        "exclusions": {
            "protectedUnitFingerprints": protected,
            "homeUnitFingerprints": home,
            "protectedDiskFingerprints": protected_disks,
        },
        "unknown": manifest.get("unknown", {}),
        "destructiveAuthorizationClaimPresent": "destructiveAuthorized" in manifest,
        "destructiveAuthorizationClaimTrue": manifest.get("destructiveAuthorized") is True,
        "manifestAuthorizationIgnored": "destructiveAuthorized" in manifest,
    }


def read_json(path_text: str) -> tuple[dict[str, Any], dict[str, Any]]:
    path = Path(path_text)
    try:
        initial = os.lstat(path)
    except OSError as error:
        raise fail(f"cannot read evidence file: {error}") from error
    if stat.S_ISLNK(initial.st_mode) or not stat.S_ISREG(initial.st_mode):
        raise fail("evidence file must be a regular non-symlink file")
    try:
        descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    except OSError as error:
        raise fail(f"cannot read evidence file: {error}") from error
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode):
            raise fail("evidence file must be a regular non-symlink file")
        if not 0 < info.st_size <= MAX_INPUT_BYTES:
            raise fail("evidence file exceeds the 1 MiB input bound or is empty")
        data = os.read(descriptor, MAX_INPUT_BYTES + 1)
        if len(data) != info.st_size:
            raise fail("evidence file changed while being read")
        value = json.loads(data)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise fail(f"invalid JSON evidence file: {error}") from error
    finally:
        os.close(descriptor)
    if not isinstance(value, dict):
        raise fail("evidence JSON root must be an object")
    return value, {"envelopeSha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)}


def evidence_envelope(value: dict[str, Any], now_ms: int) -> tuple[str, list[str]]:
    required = {"capturedAtMs", "unitFingerprint", "memberDiskFingerprints", "target", "payload"}
    if not required.issubset(value):
        raise fail("evidence envelope is missing required identity, target, timestamp, or payload")
    captured = value["capturedAtMs"]
    if isinstance(captured, bool) or not isinstance(captured, int):
        raise fail("evidence capturedAtMs must be an integer, not boolean")
    if captured > now_ms + 300_000:
        raise fail("evidence timestamp is more than five minutes in the future")
    if captured < now_ms - 86_400_000:
        raise fail("evidence timestamp is older than 24 hours")
    target = value["target"]
    if not isinstance(target, dict):
        raise fail("evidence target must be an object")
    for key in ("model", "firmware", "operatingSystem", "transport"):
        item = target.get(key)
        if not isinstance(item, str) or not item or item != item.strip() or item.lower() == "pending":
            raise fail(f"evidence target.{key} is unbound")
    if not isinstance(value["payload"], dict) or not value["payload"]:
        raise fail("evidence payload must contain nonempty copied raw CLI JSON")
    return (
        stable_fingerprint(value["unitFingerprint"], "evidence.unitFingerprint"),
        string_list(value["memberDiskFingerprints"], "evidence.memberDiskFingerprints"),
    )


def baseline_state(
    manifest: dict[str, Any], evidence: list[tuple[str, dict[str, Any], dict[str, Any]]], now_ms: int | None = None
) -> dict[str, Any]:
    now_ms = int(time.time() * 1000) if now_ms is None else now_ms
    receipts = []
    mismatches = []
    valid_envelopes = 0
    for name, value, receipt in evidence:
        receipts.append({"kind": name, "identityTrust": "operator-asserted-not-attested", **receipt})
        try:
            unit, disks = evidence_envelope(value, now_ms)
        except ValueError as error:
            mismatches.append(f"{name}: {error}")
            continue
        valid_envelopes += 1
        if unit != manifest["unitFingerprint"]:
            mismatches.append(f"{name}: unit fingerprint changed or wrong unit")
        if disks != manifest["disposableMemberDisks"]:
            mismatches.append(f"{name}: disposable member inventory changed")
        if value["target"] != manifest["target"]:
            mismatches.append(f"{name}: target metadata changed or wrong enclosure")
    return {
        "state": "BLOCKED" if mismatches else "PENDING_HARDWARE",
        "ready": False,
        "receipts": receipts,
        "identityEvidencePresent": valid_envelopes > 0,
        "mismatches": mismatches,
    }


def plan(manifest: dict[str, Any], baseline: dict[str, Any]) -> dict[str, Any]:
    guard = [
        "Match the stable unit fingerprint and current disposable member-disk inventory.",
        "Confirm isolation from protected and home units, with a current backup receipt.",
        "Obtain separate explicit destructive authorization at execution time.",
    ]
    return {
        "schemaVersion": SCHEMA_VERSION,
        "mode": "offline-plan-only",
        "execution": "BLOCKED: this tool never formats, resets, mounts, opens a driver, or dispatches hardware commands.",
        "target": manifest["target"],
        "unitFingerprint": manifest["unitFingerprint"],
        "baseline": baseline,
        "manifestDestructiveAuthorization": {
            "present": manifest["destructiveAuthorizationClaimPresent"],
            "trueValue": manifest["destructiveAuthorizationClaimTrue"],
            "ignored": manifest["manifestAuthorizationIgnored"],
        },
        "manifestAuthorizationIgnored": manifest["manifestAuthorizationIgnored"],
        "steps": [
            {"name": "initial-inventory", "kind": "manual-read-only", "guards": guard[:2]},
            {"name": "non-destructive-baseline", "kind": "manual-read-only", "requires": ["copied pool/storage/scan JSON", "bit-for-bit member-disk image copies", "hashes", "read-only BeyondRAID parser output"]},
            {"name": "controlled-format-reset", "kind": "future-manual-destructive", "guards": guard},
            {"name": "capacity-profile", "kind": "future-manual", "guards": guard[:2]},
            {"name": "drive-migration", "kind": "future-manual", "guards": guard},
            {"name": "fault-replacement-recovery", "kind": "future-manual", "guards": guard + ["Reconstruct only from image copies and hash-verified parser evidence."]},
        ],
    }


def template_plan() -> dict[str, Any]:
    return plan(
        {
            "target": {
                "model": "pending",
                "firmware": "pending",
                "operatingSystem": "pending",
                "transport": "pending",
            },
            "unitFingerprint": None,
            "disposableMemberDisks": [],
            "exclusions": {},
            "unknown": {"hardware": "pending"},
            "destructiveAuthorizationClaimPresent": False,
            "destructiveAuthorizationClaimTrue": False,
            "manifestAuthorizationIgnored": False,
        },
        {
            "state": "PENDING_HARDWARE",
            "ready": False,
            "receipts": [],
            "identityEvidencePresent": False,
            "mismatches": [],
        },
    )


def write_new(out_dir: str, name: str, content: dict[str, Any]) -> Path:
    directory = Path(out_dir)
    directory.mkdir(parents=True, exist_ok=True)
    destination = directory / name
    try:
        with destination.open("x", encoding="utf-8") as handle:
            json.dump(content, handle, indent=2, sort_keys=True)
            handle.write("\n")
    except FileExistsError as error:
        raise fail(f"refusing to overwrite stale receipt: {destination}") from error
    return destination


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest="command", required=True)
    init = commands.add_parser("init", help="write a manifest template")
    init.add_argument("--out-dir", required=True)
    validate = commands.add_parser("validate", help="validate a supplied manifest")
    validate.add_argument("--manifest", required=True)
    build = commands.add_parser("plan", help="write a deterministic, non-executing lab plan")
    source = build.add_mutually_exclusive_group(required=True)
    source.add_argument("--manifest")
    source.add_argument("--template", action="store_true")
    build.add_argument("--out-dir", required=True)
    for option in ("pool", "storage", "scan"):
        build.add_argument(f"--{option}-json")
    return result


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        if args.command == "init":
            print(write_new(args.out_dir, MANIFEST_FILE, TEMPLATE))
            return 0
        if args.command == "validate":
            manifest_value, _ = read_json(args.manifest)
            checked = validate_manifest(manifest_value)
            print(json.dumps(checked, sort_keys=True))
            return 0
        if args.template:
            if any(getattr(args, f"{name}_json") for name in ("pool", "storage", "scan")):
                raise fail("template plans refuse evidence; use a validated real manifest")
            print(write_new(args.out_dir, PLAN_FILE, template_plan()))
            return 0
        manifest_value, _ = read_json(args.manifest)
        checked = validate_manifest(manifest_value)
        evidence = []
        for name in ("pool", "storage", "scan"):
            location = getattr(args, f"{name}_json")
            if location:
                value, receipt = read_json(location)
                evidence.append((name, value, receipt))
        print(write_new(args.out_dir, PLAN_FILE, plan(checked, baseline_state(checked, evidence))))
        return 0
    except ValueError as error:
        print(f"spare-unit-lab: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
