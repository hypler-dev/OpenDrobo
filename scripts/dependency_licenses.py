#!/usr/bin/env python3
"""Create bounded third-party notices from selected locked macOS dependencies.

This is a receipt generator. It does not decide legal compatibility, shipping
contents, trademark rights, or vendor-software rights.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import io
import json
import os
import re
import stat
import subprocess
import tarfile
import tomllib
from collections import defaultdict, deque
from pathlib import Path
from typing import Any, Callable

from desktop_tools import ROOT, environment

MAX_METADATA_BYTES = 32 * 1024 * 1024
MAX_PACKAGE_JSON_BYTES = 256 * 1024
MAX_LICENSE_FILE_BYTES = 256 * 1024
MAX_NOTICE_BYTES = 16 * 1024 * 1024
MAX_SUPPLEMENT_BYTES = 2 * 1024 * 1024
MAX_DIRECTORY_ENTRIES = 256
MAX_ARCHIVE_BYTES = 32 * 1024 * 1024
MAX_ARCHIVE_MEMBERS = 4096
MAX_ARCHIVE_EXPANDED_BYTES = 64 * 1024 * 1024
CARGO_NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_-]{0,127}$")
NPM_NAME = re.compile(r"^(?:[A-Za-z0-9][A-Za-z0-9._-]{0,127}|@[A-Za-z0-9][A-Za-z0-9._-]{0,127}/[A-Za-z0-9][A-Za-z0-9._-]{0,127})$")
SAFE_VERSION = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._+:-]{0,200}$")
LICENSE_FILE = re.compile(r"^(licen[cs]e|copying|notice)(?:[-._].*)?$", re.IGNORECASE)


def _read_regular(path: Path, limit: int) -> bytes:
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_size > limit:
            raise ValueError(f"Input is not a bounded regular file: {path.name}")
        data = os.read(descriptor, limit + 1)
        if len(data) != info.st_size or len(data) > limit:
            raise ValueError(f"Input changed or exceeds byte limit: {path.name}")
        return data
    finally:
        os.close(descriptor)


def _safe_cargo_identity(name: object, version: object) -> tuple[str, str]:
    if not isinstance(name, str) or not isinstance(version, str):
        raise ValueError("Cargo dependency identity is not text")
    if not CARGO_NAME.fullmatch(name) or not SAFE_VERSION.fullmatch(version):
        raise ValueError("Cargo dependency identity is unsafe")
    return name, version


def _safe_npm_identity(name: object, version: object) -> tuple[str, str]:
    if not isinstance(name, str) or not isinstance(version, str):
        raise ValueError("Npm dependency identity is not text")
    if not NPM_NAME.fullmatch(name) or not SAFE_VERSION.fullmatch(version):
        raise ValueError("Npm dependency identity is unsafe")
    return name, version


def _safe_text(value: object, field: str) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str) or not value or len(value) > 1024:
        raise ValueError(f"{field} is invalid")
    if any(ord(character) < 32 and character not in "\n\r\t" for character in value):
        raise ValueError(f"{field} contains control characters")
    return value


def _load_json(path: Path, limit: int) -> tuple[dict[str, Any], bytes]:
    try:
        raw = _read_regular(path, limit)
        value = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"Invalid JSON: {path.name}") from error
    if not isinstance(value, dict):
        raise ValueError(f"JSON root is not an object: {path.name}")
    return value, raw


def _cargo_lock(path: Path) -> tuple[dict[tuple[str, str], dict[str, str | None]], str]:
    try:
        raw = _read_regular(path, MAX_METADATA_BYTES)
        payload = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError, OSError, ValueError) as error:
        raise ValueError("Cargo.lock is unavailable") from error
    packages: dict[tuple[str, str], dict[str, str | None]] = {}
    for item in payload.get("package", []):
        if not isinstance(item, dict):
            raise ValueError("Cargo.lock package is invalid")
        name, version = _safe_cargo_identity(item.get("name"), item.get("version"))
        source, checksum = item.get("source"), item.get("checksum")
        if source == "registry+https://github.com/rust-lang/crates.io-index":
            if not isinstance(checksum, str) or not re.fullmatch(r"[0-9a-f]{64}", checksum):
                raise ValueError("Cargo.lock registry checksum is invalid")
        elif source is not None or checksum is not None:
            raise ValueError("Cargo.lock source/checksum is invalid")
        key = name, version
        if key in packages:
            raise ValueError("Cargo.lock package identity is duplicated")
        packages[key] = {"source": source, "checksum": checksum}
    return packages, hashlib.sha256(raw).hexdigest()


def _selected_cargo(metadata: dict[str, Any], platform: str,
                    locked: dict[tuple[str, str], dict[str, str | None]]) -> list[dict[str, Any]]:
    packages = metadata.get("packages")
    resolve = metadata.get("resolve")
    defaults = metadata.get("workspace_default_members")
    if not isinstance(packages, list) or not isinstance(resolve, dict) or not isinstance(defaults, list) or len(defaults) != 1:
        raise ValueError("Cargo metadata must have exactly one default workspace package")
    package_by_id = {item.get("id"): item for item in packages if isinstance(item, dict) and isinstance(item.get("id"), str)}
    nodes = resolve.get("nodes")
    if not isinstance(nodes, list):
        raise ValueError("Cargo metadata resolve nodes are missing")
    node_by_id = {item.get("id"): item for item in nodes if isinstance(item, dict) and isinstance(item.get("id"), str)}
    root = defaults[0]
    if root not in node_by_id or root not in package_by_id:
        raise ValueError("Default desktop package is not in Cargo resolution")
    root_name, root_version = _safe_cargo_identity(package_by_id[root].get("name"), package_by_id[root].get("version"))
    if (root_name, root_version) not in locked:
        raise ValueError("Desktop package is absent from Cargo.lock")
    usage: dict[str, set[str]] = defaultdict(set)
    queue: deque[tuple[str, str]] = deque([(root, "runtime")])
    while queue:
        package_id, parent_usage = queue.popleft()
        if parent_usage in usage[package_id]:
            continue
        usage[package_id].add(parent_usage)
        node = node_by_id.get(package_id)
        if node is None:
            raise ValueError("Cargo resolution dependency node is missing")
        for dependency in node.get("deps", []):
            if not isinstance(dependency, dict) or not isinstance(dependency.get("pkg"), str):
                raise ValueError("Cargo resolution dependency is invalid")
            kinds = dependency.get("dep_kinds")
            if not isinstance(kinds, list) or not kinds:
                raise ValueError("Cargo dependency kind is missing")
            for kind in kinds:
                if not isinstance(kind, dict) or kind.get("kind") not in (None, "build"):
                    raise ValueError("Unsupported Cargo dependency kind")
                child_usage = "build" if parent_usage == "build" or kind.get("kind") == "build" else "runtime"
                queue.append((dependency["pkg"], child_usage))
    selected = []
    for package_id, kinds in sorted(usage.items()):
        package = package_by_id.get(package_id)
        if package is None:
            raise ValueError("Cargo package is missing")
        source = package.get("source")
        if source is None:
            continue
        if source != "registry+https://github.com/rust-lang/crates.io-index":
            raise ValueError("Unsupported third-party Cargo source")
        name, version = _safe_cargo_identity(package.get("name"), package.get("version"))
        lock_entry = locked.get((name, version))
        if lock_entry is None or lock_entry["source"] != source or not isinstance(lock_entry["checksum"], str):
            raise ValueError("Cargo metadata registry package is not exactly locked")
        selected.append({"ecosystem": "cargo", "name": name, "version": version,
                         "license": _safe_text(package.get("license"), "Cargo metadata license"),
                         "sourceUrl": f"https://crates.io/crates/{name}/{version}",
                         "platforms": [platform], "usage": sorted(kinds), "lockChecksum": lock_entry["checksum"]})
    return selected


def _merge_components(components: list[dict[str, Any]]) -> list[dict[str, Any]]:
    merged: dict[tuple[str, str, str], dict[str, Any]] = {}
    for item in components:
        key = item["ecosystem"], item["name"], item["version"]
        existing = merged.get(key)
        if existing is None:
            merged[key] = {**item, "platforms": set(item["platforms"]), "usage": set(item["usage"])}
            continue
        for field in ("license", "lockChecksum"):
            if existing[field] != item[field]:
                raise ValueError("Dependency metadata disagrees across target graphs")
        existing["platforms"].update(item["platforms"])
        existing["usage"].update(item["usage"])
    return sorted(({**item, "platforms": sorted(item["platforms"]), "usage": sorted(item["usage"])} for item in merged.values()),
                  key=lambda item: (item["ecosystem"], item["name"], item["version"]))


def _tar_members(data: bytes, prefix: str, manifest_name: str) -> tuple[dict[str, bytes], dict[str, Any]]:
    try:
        archive = tarfile.open(fileobj=io.BytesIO(data), mode="r|gz")
    except (tarfile.TarError, OSError) as error:
        raise ValueError("Package archive is invalid") from error
    files: dict[str, bytes] = {}
    total = count = 0
    with archive:
        for member in archive:
            count += 1
            if count > MAX_ARCHIVE_MEMBERS:
                raise ValueError("Package archive has too many members")
            parts = Path(member.name).parts
            if not member.name.startswith(prefix) or member.name.startswith("/") or ".." in parts:
                raise ValueError("Package archive member path is invalid")
            if member.isdir():
                continue
            if not member.isreg() or member.issym() or member.islnk():
                raise ValueError("Package archive has unsupported member type")
            total += member.size
            if total > MAX_ARCHIVE_EXPANDED_BYTES:
                raise ValueError("Package archive expanded size exceeds limit")
            relative = member.name[len(prefix):]
            selected = relative == manifest_name or relative == ".cargo_vcs_info.json" or LICENSE_FILE.fullmatch(relative)
            if "/" in relative or not selected:
                continue
            handle = archive.extractfile(member)
            if handle is None:
                raise ValueError("Package archive member is unreadable")
            limit = MAX_PACKAGE_JSON_BYTES if relative == manifest_name else MAX_LICENSE_FILE_BYTES
            value = handle.read(limit + 1)
            if len(value) > limit or not value or relative in files:
                raise ValueError("Package archive selected member is invalid")
            files[relative] = value
    raw = files.get(manifest_name)
    if raw is None:
        raise ValueError(f"Package archive lacks top-level {manifest_name}")
    try:
        manifest = tomllib.loads(raw.decode("utf-8")) if manifest_name == "Cargo.toml" else json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError, json.JSONDecodeError) as error:
        raise ValueError("Package archive manifest is invalid") from error
    if not isinstance(manifest, dict):
        raise ValueError("Package archive manifest is invalid")
    return files, manifest


def _cache_directories(cache: Path) -> list[Path]:
    try:
        entries = list(cache.iterdir())
    except OSError as error:
        raise ValueError("Package cache is unavailable") from error
    if len(entries) > MAX_DIRECTORY_ENTRIES:
        raise ValueError("Package cache has too many registry entries")
    result = []
    for entry in entries:
        info = entry.lstat()
        if stat.S_ISLNK(info.st_mode):
            raise ValueError("Package cache registry entry is a symlink")
        if stat.S_ISDIR(info.st_mode):
            result.append(entry)
    return sorted(result)


def _cargo_archive(cargo_home: Path, name: str, version: str, checksum: str) -> tuple[dict[str, bytes], dict[str, Any]]:
    _safe_cargo_identity(name, version)
    for registry in _cache_directories(cargo_home / "registry" / "cache"):
        archive = registry / f"{name}-{version}.crate"
        if archive.exists():
            data = _read_regular(archive, MAX_ARCHIVE_BYTES)
            if hashlib.sha256(data).hexdigest() != checksum:
                raise ValueError("Cargo archive checksum does not match Cargo.lock")
            return _tar_members(data, f"{name}-{version}/", "Cargo.toml")
    raise ValueError(f"Locked Cargo archive unavailable for {name}@{version}")


def _documents(files: dict[str, bytes], provenance: str) -> tuple[list[dict[str, str]], bool]:
    documents: list[dict[str, str]] = []
    terms = False
    for file_name, data in sorted(files.items()):
        if file_name in ("Cargo.toml", "package.json", ".cargo_vcs_info.json"):
            continue
        text = data.decode("utf-8")
        kind = "notice" if file_name.lower().startswith("notice") else "license-terms"
        terms = terms or kind == "license-terms"
        documents.append({"file": file_name, "sha256": hashlib.sha256(data).hexdigest(), "text": text,
                          "provenance": provenance, "kind": kind})
    return documents, terms


def _vcs_commit(files: dict[str, bytes]) -> str | None:
    raw = files.get(".cargo_vcs_info.json")
    if raw is None:
        return None
    try:
        value = json.loads(raw.decode("utf-8"))["git"]["sha1"]
    except (UnicodeDecodeError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise ValueError("Cargo archive VCS receipt is invalid") from error
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError("Cargo archive VCS receipt is invalid")
    return value


def _supplements(paths: list[Path]) -> dict[tuple[str, str, str], list[dict[str, str]]]:
    """Load operator-reviewed immutable upstream terms; never fetch URLs."""
    result: dict[tuple[str, str, str], list[dict[str, str]]] = defaultdict(list)
    for path in paths:
        payload, _ = _load_json(path, MAX_SUPPLEMENT_BYTES)
        records = payload.get("supplements")
        if payload.get("schemaVersion") != 1 or not isinstance(records, list) or len(records) > 64:
            raise ValueError("Supplemental license input has invalid schema")
        for record in records:
            if not isinstance(record, dict):
                raise ValueError("Supplemental license record is invalid")
            name, version = _safe_cargo_identity(record.get("name"), record.get("version"))
            commit, source_url, file_name = record.get("originCommit"), record.get("sourceUrl"), record.get("file")
            text, digest = record.get("text"), record.get("sha256")
            if record.get("ecosystem") != "cargo" or not isinstance(commit, str) or not re.fullmatch(r"[0-9a-f]{40}", commit):
                raise ValueError("Supplemental license origin is invalid")
            if not isinstance(source_url, str) or not source_url.startswith("https://raw.githubusercontent.com/") or f"/{commit}/" not in source_url:
                raise ValueError("Supplemental license source must pin its immutable Git commit")
            if not isinstance(file_name, str) or not LICENSE_FILE.fullmatch(file_name):
                raise ValueError("Supplemental license file name is invalid")
            if not isinstance(text, str) or not 0 < len(text.encode("utf-8")) <= MAX_LICENSE_FILE_BYTES:
                raise ValueError("Supplemental license text is invalid")
            if "copyright" not in text.lower() or not isinstance(digest, str) or hashlib.sha256(text.encode("utf-8")).hexdigest() != digest:
                raise ValueError("Supplemental license text is not original copyright-bearing content")
            kind = "notice" if file_name.lower().startswith("notice") else "license-terms"
            value = {"file": file_name, "sha256": digest, "text": text, "provenance": "supplemental-reviewed",
                     "kind": kind, "originCommit": commit, "sourceUrl": source_url}
            key = "cargo", name, version
            if value in result[key]:
                raise ValueError("Supplemental license record is duplicated")
            result[key].append(value)
    return result


def _npm_archive(npm_cache: Path, integrity: str) -> bytes:
    if not isinstance(integrity, str) or not integrity.startswith("sha512-"):
        raise ValueError("Npm lock integrity is invalid")
    try:
        digest = base64.b64decode(integrity[7:], validate=True)
    except Exception as error:
        raise ValueError("Npm lock integrity is invalid") from error
    if len(digest) != 64:
        raise ValueError("Npm lock integrity is invalid")
    encoded = digest.hex()
    archive = npm_cache / "_cacache" / "content-v2" / "sha512" / encoded[:2] / encoded[2:4] / encoded[4:]
    try:
        data = _read_regular(archive, MAX_ARCHIVE_BYTES)
    except OSError as error:
        raise ValueError("Npm archive integrity cache entry is unavailable") from error
    if hashlib.sha512(data).digest() != digest:
        raise ValueError("Npm archive integrity does not match package-lock")
    return data


def _npm_component(lock: dict[str, Any], desktop_package: dict[str, Any], npm_cache: Path, name: str) -> tuple[dict[str, Any], list[dict[str, str]], bool]:
    _safe_npm_identity(name, "1")
    packages = lock.get("packages")
    root = packages.get("") if isinstance(packages, dict) else None
    location = f"node_modules/{name}"
    locked = packages.get(location) if isinstance(packages, dict) else None
    if not isinstance(root, dict) or not isinstance(locked, dict):
        raise ValueError(f"Npm lock entry unavailable for {name}")
    version = locked.get("version")
    _safe_npm_identity(name, version)
    root_dependencies, desktop_dependencies = root.get("dependencies"), desktop_package.get("dependencies")
    if not isinstance(root_dependencies, dict) or root_dependencies.get(name) != version:
        raise ValueError("Npm runtime package is not a package-lock production dependency")
    if not isinstance(desktop_dependencies, dict) or desktop_dependencies.get(name) != version:
        raise ValueError("Npm runtime package is not a package.json production dependency")
    files, package = _tar_members(_npm_archive(npm_cache, locked.get("integrity")), "package/", "package.json")
    if package.get("name") != name or package.get("version") != version:
        raise ValueError("Npm archive identity does not match package-lock")
    archive_license, lock_license = _safe_text(package.get("license"), "Npm archive license"), _safe_text(locked.get("license"), "Npm lock license")
    documents, terms = _documents(files, "verified-npm-archive")
    return ({"ecosystem": "npm", "name": name, "version": version, "license": lock_license, "archiveLicense": archive_license,
             "licenseDeclarationMatch": lock_license == archive_license, "sourceUrl": f"https://www.npmjs.com/package/{name}/v/{version}",
             "platforms": ["macos-ui-bundle"], "usage": ["runtime"], "lockIntegrity": locked["integrity"]}, documents, terms)


def _attach_documents(components: list[dict[str, Any]], cargo_home: Path,
                      supplements: dict[tuple[str, str, str], list[dict[str, str]]],
                      npm_documents: dict[tuple[str, str, str], tuple[list[dict[str, str]], bool]]) -> list[dict[str, Any]]:
    total = 0
    attached = []
    for item in components:
        key = item["ecosystem"], item["name"], item["version"]
        if item["ecosystem"] == "npm":
            documents, terms = npm_documents[key]
        else:
            files, archive = _cargo_archive(cargo_home, item["name"], item["version"], item["lockChecksum"])
            package = archive.get("package")
            if not isinstance(package, dict) or package.get("name") != item["name"] or package.get("version") != item["version"]:
                raise ValueError("Cargo archive identity does not match Cargo.lock")
            item["archiveLicense"] = _safe_text(package.get("license"), "Cargo archive license")
            item["licenseDeclarationMatch"] = item["license"] == item["archiveLicense"]
            documents, terms = _documents(files, "verified-cargo-archive")
            supplied = supplements.get(key, [])
            if supplied:
                if terms:
                    raise ValueError("Supplement cannot alter package with verified local license terms")
                commit = _vcs_commit(files)
                if commit is None or any(record["originCommit"] != commit for record in supplied):
                    raise ValueError("Supplement commit does not match verified Cargo archive VCS receipt")
                documents.extend(supplied)
                terms = any(record["kind"] == "license-terms" for record in supplied)
        total += sum(len(document["text"].encode("utf-8")) for document in documents)
        if total > MAX_NOTICE_BYTES:
            raise ValueError("Third-party notice output exceeds byte limit")
        status = "documentary-coverage"
        if not item["licenseDeclarationMatch"]:
            status = "license-declaration-mismatch"
        elif not item["license"] or not terms:
            status = "missing-license-terms"
        attached.append({**item, "licenseDocuments": documents, "noticeStatus": status})
    return attached


def _canonical_graph(metadata: dict[str, Any]) -> dict[str, Any]:
    packages, resolve, defaults = metadata.get("packages"), metadata.get("resolve"), metadata.get("workspace_default_members")
    if not isinstance(packages, list) or not isinstance(resolve, dict) or not isinstance(defaults, list):
        raise ValueError("Cargo metadata cannot be canonicalized")
    package_rows = []
    for item in packages:
        if not isinstance(item, dict) or not isinstance(item.get("id"), str):
            raise ValueError("Cargo metadata package is invalid")
        package_rows.append((item["id"], item.get("name"), item.get("version"), item.get("source")))
    nodes = resolve.get("nodes")
    if not isinstance(nodes, list):
        raise ValueError("Cargo metadata resolve nodes are invalid")
    node_rows = []
    for node in nodes:
        if not isinstance(node, dict) or not isinstance(node.get("id"), str) or not isinstance(node.get("deps"), list):
            raise ValueError("Cargo metadata resolution node is invalid")
        deps = []
        for dep in node["deps"]:
            if not isinstance(dep, dict) or not isinstance(dep.get("pkg"), str) or not isinstance(dep.get("name"), str) or not isinstance(dep.get("dep_kinds"), list):
                raise ValueError("Cargo metadata dependency is invalid")
            if not all(isinstance(kind, dict) for kind in dep["dep_kinds"]):
                raise ValueError("Cargo metadata dependency kinds are invalid")
            kinds = tuple(sorted(json.dumps(kind, sort_keys=True, separators=(",", ":")) for kind in dep["dep_kinds"]))
            deps.append((dep["name"], dep["pkg"], kinds))
        node_rows.append((node["id"], tuple(sorted(deps)), tuple(sorted(node.get("features", [])))))
    return {"workspaceDefaultMembers": sorted(defaults), "packages": sorted(package_rows), "nodes": sorted(node_rows)}


def _verify_cli_graph(platform: str, supplied: dict[str, Any]) -> dict[str, str]:
    command = ["cargo", "metadata", "--offline", "--locked", "--format-version", "1", "--filter-platform", platform,
               "--manifest-path", "desktop/src-tauri/Cargo.toml"]
    completed = subprocess.run(command, cwd=ROOT, env=environment("license-graph"), stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if completed.returncode != 0:
        raise ValueError(f"Cargo metadata reproduction failed for {platform}: {completed.stderr.decode('utf-8', 'replace').strip()}")
    try:
        fresh = json.loads(completed.stdout.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("Cargo metadata reproduction returned invalid JSON") from error
    if _canonical_graph(supplied) != _canonical_graph(fresh):
        raise ValueError(f"Supplied Cargo metadata does not match locked default-feature graph for {platform}")
    version = subprocess.run(["cargo", "--version"], cwd=ROOT, env=environment("license-graph"), stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True).stdout.decode("utf-8").strip()
    return {"command": " ".join(command), "cargoVersion": version}


def build_manifest(metadata_inputs: list[tuple[str, Path]], *, cargo_home: Path, cargo_lock: Path,
                   npm_lock: Path, desktop_package: Path, npm_cache: Path, npm_runtime: list[str],
                   supplemental_license: list[Path] | None = None,
                   graph_verifier: Callable[[str, dict[str, Any]], dict[str, str]] | None = None) -> dict[str, Any]:
    locked, cargo_lock_hash = _cargo_lock(cargo_lock)
    components = []
    metadata_hashes: dict[str, str] = {}
    graph_receipts: dict[str, dict[str, str]] = {}
    seen: set[str] = set()
    for platform, path in metadata_inputs:
        if platform not in ("aarch64-apple-darwin", "x86_64-apple-darwin") or platform in seen:
            raise ValueError("Each supported macOS target metadata must be supplied exactly once")
        seen.add(platform)
        payload, raw = _load_json(path, MAX_METADATA_BYTES)
        metadata_hashes[platform] = hashlib.sha256(raw).hexdigest()
        if graph_verifier is not None:
            graph_receipts[platform] = graph_verifier(platform, payload)
        components.extend(_selected_cargo(payload, platform, locked))
    if seen != {"aarch64-apple-darwin", "x86_64-apple-darwin"}:
        raise ValueError("Both supported macOS target metadata inputs are required")
    lock, npm_lock_raw = _load_json(npm_lock, MAX_METADATA_BYTES)
    desktop, desktop_raw = _load_json(desktop_package, MAX_PACKAGE_JSON_BYTES)
    npm_documents: dict[tuple[str, str, str], tuple[list[dict[str, str]], bool]] = {}
    npm_components = []
    for name in sorted(set(npm_runtime)):
        component, documents, terms = _npm_component(lock, desktop, npm_cache, name)
        npm_components.append(component)
        npm_documents[(component["ecosystem"], component["name"], component["version"])] = documents, terms
    selected = _attach_documents(_merge_components(components) + npm_components, cargo_home, _supplements(supplemental_license or []), npm_documents)
    missing = [f"{item['ecosystem']}:{item['name']}@{item['version']}" for item in selected if item["noticeStatus"] != "documentary-coverage"]
    return {"schemaVersion": 1, "scope": "selected locked macOS Cargo graphs plus explicit runtime UI packages; not shipping-content proof",
            "legalReviewRequired": True, "noticeReady": not missing, "defaultGraphVerified": graph_verifier is not None,
            "metadataGraphAttestation": "operator-supplied graph; reproduction receipt required for CLI verification",
            "inputSha256": {"desktop/src-tauri/Cargo.lock": cargo_lock_hash, "desktop/package-lock.json": hashlib.sha256(npm_lock_raw).hexdigest(), "desktop/package.json": hashlib.sha256(desktop_raw).hexdigest()},
            "metadataSha256": metadata_hashes, "graphVerification": graph_receipts, "missingLicenseText": missing, "components": selected}


def _render_notices(manifest: dict[str, Any]) -> str:
    lines = ["# OpenDrobo third-party notices", "", "Generated from verified package archives selected by locked macOS graphs.", "This notice bundle is not legal clearance or shipping-content proof.", ""]
    for item in manifest["components"]:
        lines.extend([f"## {item['ecosystem']}:{item['name']}@{item['version']}", "", f"Declared license: {item['license'] or 'missing'}", f"Archive license: {item['archiveLicense'] or 'missing'}", f"Source: {item['sourceUrl']}", f"Platforms: {', '.join(item['platforms'])}", f"Usage path: {', '.join(item['usage'])}", ""])
        if not item["licenseDocuments"]:
            lines.extend(["License terms missing from verified package archive.", ""])
        for document in item["licenseDocuments"]:
            lines.extend([f"### {document['file']} (SHA-256 {document['sha256']})", "", "```text", *([f"Provenance: {document['provenance']}", f"Immutable source: {document['sourceUrl']}", f"Origin commit: {document['originCommit']}", ""] if document["provenance"] == "supplemental-reviewed" else []), document["text"].rstrip("\n"), "```", ""])
    return "\n".join(lines)


def write_outputs(outdir: Path, manifest: dict[str, Any]) -> tuple[Path, Path]:
    outdir.mkdir(parents=True, exist_ok=True)
    manifest_path, notices_path = outdir / "third-party-manifest.json", outdir / "THIRD_PARTY_NOTICES.md"
    with manifest_path.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    with notices_path.open("x", encoding="utf-8") as stream:
        stream.write(_render_notices(manifest))
    return manifest_path, notices_path


def _metadata_argument(value: str) -> tuple[str, Path]:
    platform, separator, path = value.partition("=")
    if not separator or not path:
        raise argparse.ArgumentTypeError("metadata must use target=path")
    return platform, Path(path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--metadata", action="append", required=True, type=_metadata_argument)
    parser.add_argument("--npm-runtime", action="append", default=["@tauri-apps/api"])
    parser.add_argument("--supplemental-license", action="append", type=Path, default=[])
    parser.add_argument("--outdir", required=True, type=Path)
    args = parser.parse_args()
    manifest = build_manifest(args.metadata, cargo_home=ROOT / "tools/cargo-home", cargo_lock=ROOT / "desktop/src-tauri/Cargo.lock",
                              npm_lock=ROOT / "desktop/package-lock.json", desktop_package=ROOT / "desktop/package.json",
                              npm_cache=ROOT / "tools/npm-cache", npm_runtime=args.npm_runtime,
                              supplemental_license=args.supplemental_license, graph_verifier=_verify_cli_graph)
    print(*write_outputs(args.outdir, manifest), sep="\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
