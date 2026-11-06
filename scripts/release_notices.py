"""Bind a local notice bundle to current inputs; never infer legal approval."""
from __future__ import annotations

import hashlib
import json
import os
import re
from pathlib import Path
import stat

from desktop_tools import ROOT

MAX_NOTICE_BYTES = 16 * 1024 * 1024
NOTICE_FILES = ("third-party-manifest.json", "THIRD_PARTY_NOTICES.md")
NOTICE_INPUTS = ("desktop/src-tauri/Cargo.lock", "desktop/package-lock.json", "desktop/package.json")


def bounded_file(path: Path, limit: int = MAX_NOTICE_BYTES) -> bytes:
    """Pin every path component and refuse links, special files and oversized data."""
    path = path.absolute()
    if ".." in path.parts:
        raise ValueError("Notice path must be canonical")
    directory = os.open(path.anchor, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for component in path.parts[1:-1]:
            next_directory = os.open(component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                                     dir_fd=directory)
            os.close(directory)
            directory = next_directory
        descriptor = os.open(path.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=directory)
        with os.fdopen(descriptor, "rb") as source:
            info = os.fstat(source.fileno())
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or not 0 < info.st_size <= limit:
                raise ValueError("Notice input must be a bounded single-link regular file")
            data = source.read(limit + 1)
            if len(data) != info.st_size or len(data) > limit:
                raise ValueError("Notice input changed during read")
            return data
    finally:
        os.close(directory)


def read_notice_bundle(directory: Path | None, *, root: Path = ROOT) -> tuple[dict, dict[str, bytes]]:
    if directory is None:
        return {"bundled": False, "complete": False, "legalReviewRequired": True}, {}
    files = {name: bounded_file(directory / name) for name in NOTICE_FILES}
    try:
        manifest = json.loads(files[NOTICE_FILES[0]].decode("utf-8"))
        text = files[NOTICE_FILES[1]].decode("utf-8")
    except (ValueError, UnicodeDecodeError) as error:
        raise ValueError("Notice bundle is not valid UTF-8 JSON/text") from error
    if not isinstance(manifest, dict) or type(manifest.get("schemaVersion")) is not int or manifest["schemaVersion"] != 1:
        raise ValueError("Notice manifest schema is unsupported")
    components = manifest.get("components")
    missing = manifest.get("missingLicenseText")
    if (not isinstance(components, list) or not 0 < len(components) <= 2048
            or not isinstance(missing, list) or type(manifest.get("noticeReady")) is not bool
            or manifest.get("legalReviewRequired") is not True
            or manifest.get("defaultGraphVerified") is not True
            or manifest["noticeReady"] and missing or not text.strip() or "\0" in text):
        raise ValueError("Notice manifest coverage or graph verification is invalid")
    verification = manifest.get("graphVerification")
    graph_hashes = manifest.get("metadataSha256")
    supported = {"aarch64-apple-darwin", "x86_64-apple-darwin"}
    if (not isinstance(verification, dict) or not verification or not set(verification) <= supported
            or not isinstance(graph_hashes, dict) or set(graph_hashes) != set(verification)):
        raise ValueError("Notice target graph receipts are missing")
    for target, proof in verification.items():
        expected_command = f"cargo metadata --offline --locked --format-version 1 --filter-platform {target} --manifest-path desktop/src-tauri/Cargo.toml"
        if (not isinstance(proof, dict) or proof.get("command") != expected_command
                or not isinstance(graph_hashes[target], str)
                or not re.fullmatch(r"[0-9a-f]{64}", graph_hashes[target])):
            raise ValueError("Notice target graph receipt is invalid")
    inputs = manifest.get("inputSha256")
    if not isinstance(inputs, dict):
        raise ValueError("Notice input bindings are missing")
    for name in NOTICE_INPUTS:
        digest = hashlib.sha256(bounded_file(root / name, 2 * 1024 * 1024)).hexdigest()
        if inputs.get(name) != digest:
            raise ValueError(f"Notice bundle does not match current {name}")
    return {"bundled": True, "complete": manifest["noticeReady"],
            "legalReviewRequired": True, "components": len(components),
            "missingLicenseText": len(missing), "targets": sorted(verification),
            "sha256": {name: hashlib.sha256(data).hexdigest() for name, data in files.items()}}, files


def inspect_packaged_notices(resources: Path, build: dict, *, root: Path = ROOT) -> tuple[bool, str]:
    """Inspect signed resource bytes and current lock bindings, not legal suitability."""
    try:
        metadata, _ = read_notice_bundle(resources / "licenses", root=root)
        if build.get("third_party_notices") != metadata:
            return False, "notice resources disagree with build receipt"
        if build.get("target") not in metadata["targets"]:
            return False, "notice graph does not cover the artifact target"
        if not metadata["complete"]:
            return False, f"{metadata['missingLicenseText']} components lack complete notice coverage"
        own_license = bounded_file(resources / "LICENSE.txt")
        if own_license != bounded_file(root / "LICENSE"):
            return False, "application license does not match source"
        return True, f"{metadata['components']} components; documentary coverage only, legal review separate"
    except (OSError, ValueError) as error:
        return False, str(error)
