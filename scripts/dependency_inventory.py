#!/usr/bin/env python3
"""Create a deterministic pre-build dependency inventory from locked metadata only."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import re
import tomllib
from pathlib import Path
from urllib.parse import quote

from desktop_tools import ROOT

MAX_LOCK_BYTES = 2 * 1024 * 1024
MAX_TOML_BYTES = 64 * 1024
SAFE_NAME = re.compile(r"^[A-Za-z0-9@][A-Za-z0-9._@/-]{0,200}$")
SAFE_VERSION = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._+:-]{0,200}$")
LOCKS = (("Cargo.lock", "cargo"), ("desktop/src-tauri/Cargo.lock", "cargo"),
         ("desktop/package-lock.json", "npm"))


def _read(path: Path, limit: int) -> bytes:
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_size > limit:
            raise ValueError(f"Input exceeds byte limit or is not regular: {path.name}")
        data = os.read(descriptor, limit + 1)
        if len(data) > limit or len(data) != info.st_size:
            raise ValueError(f"Input changed or exceeds byte limit: {path.name}")
        return data
    finally:
        os.close(descriptor)


SAFE_LICENSE_IDS = frozenset("MIT Apache-2.0 BSD-2-Clause BSD-3-Clause ISC MPL-2.0 0BSD Zlib CC0-1.0 BSL-1.0 Unlicense Unicode-3.0 Unicode-DFS-2016 Unicode-TOU GPL-2.0-only GPL-2.0-or-later GPL-3.0-only GPL-3.0-or-later LGPL-2.1-only LGPL-2.1-or-later LGPL-3.0-only LGPL-3.0-or-later Apache-2.0 LLVM-exception Classpath-exception-2.0".split())


def safe_license(value: object) -> str | None:
    # Unknown/custom declarations require manual review; never serialize paths,
    # URLs or arbitrary cached text into a public dependency receipt.
    if not isinstance(value, str) or not 0 < len(value) <= 256 or any(ord(char) < 32 or ord(char) > 126 for char in value):
        return None
    tokens = value.replace("(", " ( ").replace(")", " ) ").split()
    if not tokens or any(token not in SAFE_LICENSE_IDS | {"AND", "OR", "WITH", "(", ")"} for token in tokens):
        return None
    return value


def _valid(name: str, version: str) -> None:
    if not SAFE_NAME.fullmatch(name) or not SAFE_VERSION.fullmatch(version):
        raise ValueError("Locked package name or version is unsafe")


def _license_from_toml(data: bytes) -> str | None:
    try:
        license_name = tomllib.loads(data.decode("utf-8")).get("package", {}).get("license")
    except (UnicodeDecodeError, tomllib.TOMLDecodeError):
        return None
    return safe_license(license_name)


def cached_cargo_license(cargo_home: Path, name: str, version: str) -> str | None:
    """Read only exact cache package manifests under Cargo's registry source root."""
    _valid(name, version)
    cache_root = (cargo_home / "registry/src").resolve()
    if not cache_root.is_dir():
        return None
    for registry in sorted(cache_root.iterdir()):
        candidate = registry / f"{name}-{version}" / "Cargo.toml"
        try:
            resolved = candidate.resolve(strict=True)
            resolved.relative_to(cache_root)
        except (OSError, ValueError):
            continue
        if resolved.is_file():
            try:
                return _license_from_toml(_read(resolved, MAX_TOML_BYTES))
            except (OSError, ValueError):
                return None
    return None


def cargo_metadata_licenses(path: Path | None) -> dict[tuple[str, str], str]:
    if path is None:
        return {}
    try:
        payload = json.loads(_read(path, MAX_LOCK_BYTES))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError("Cannot read supplied Cargo metadata") from error
    result: dict[tuple[str, str], str] = {}
    for package in payload.get("packages", []):
        if not isinstance(package, dict):
            continue
        name, version, license_name = package.get("name"), package.get("version"), package.get("license")
        if isinstance(name, str) and isinstance(version, str) and isinstance(license_name, str) and license_name:
            _valid(name, version)
            sanitized = safe_license(license_name)
            if sanitized: result[(name, version)] = sanitized
    return result


def cargo_components(path: Path, licenses: dict[tuple[str, str], str], cargo_home: Path) -> list[tuple[str, str, set[str]]]:
    try:
        payload = tomllib.loads(_read(path, MAX_LOCK_BYTES).decode("utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        raise ValueError(f"Cannot read lockfile: {path.name}") from error
    result = []
    for package in payload.get("package", []):
        name, version = package.get("name"), package.get("version")
        if not isinstance(name, str) or not isinstance(version, str):
            raise ValueError("Cargo lock package lacks name or version")
        _valid(name, version)
        declared = {value for value in (licenses.get((name, version)),
                                         cached_cargo_license(cargo_home, name, version)) if value}
        result.append((name, version, declared))
    return result


def npm_components(path: Path) -> list[tuple[str, str, set[str]]]:
    try:
        payload = json.loads(_read(path, MAX_LOCK_BYTES))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"Cannot read lockfile: {path.name}") from error
    result = []
    for location, package in payload.get("packages", {}).items():
        if not location or not isinstance(package, dict) or "node_modules/" not in location:
            continue
        name = location.rsplit("node_modules/", 1)[1]
        version, license_name = package.get("version"), package.get("license")
        if not isinstance(version, str):
            raise ValueError("Npm lock package lacks version")
        _valid(name, version)
        sanitized = safe_license(license_name)
        result.append((name, version, {sanitized} if sanitized else set()))
    return result


def component(ecosystem: str, name: str, version: str, license_names: set[str]) -> dict[str, object]:
    encoded = quote(name, safe="/")
    purl = f"pkg:{ecosystem}/{encoded}@{quote(version, safe='')}"
    declared = sorted(license_names) or ["unknown"]
    item: dict[str, object] = {"type": "library", "name": name, "version": version,
                               "purl": purl, "bom-ref": purl,
                               "properties": [
                                   {"name": "opendrobo:declared-license", "value": " | ".join(declared)},
                                   {"name": "opendrobo:license-audit-ready", "value": "false"},
                                   {"name": "opendrobo:license-conflict", "value": str(len(license_names) > 1).lower()},
                               ]}
    if license_names:
        item["licenses"] = [{"license": {"name": value}} for value in sorted(license_names)]
    return item


def inventory(root: Path = ROOT, *, cargo_home: Path | None = None,
              cargo_metadata: Path | None = None) -> dict[str, object]:
    """Inventory all lock entries. It is build input, not shipping-content proof."""
    cargo_home = cargo_home or root / "tools/cargo-home"
    metadata_licenses = cargo_metadata_licenses(cargo_metadata)
    entries: dict[tuple[str, str, str], set[str]] = {}
    hashes = {}
    for relative, ecosystem in LOCKS:
        lock = root / relative
        hashes[relative] = hashlib.sha256(_read(lock, MAX_LOCK_BYTES)).hexdigest()
        source = (cargo_components(lock, metadata_licenses, cargo_home) if ecosystem == "cargo"
                  else npm_components(lock))
        for name, version, license_names in source:
            entries.setdefault((ecosystem, name, version), set())
            entries[(ecosystem, name, version)].update(license_names)
    components = [component(ecosystem, name, version, licenses)
                  for (ecosystem, name, version), licenses in sorted(entries.items())]
    return {"bomFormat": "CycloneDX", "specVersion": "1.6", "version": 1,
            "metadata": {"lifecycles": [{"phase": "pre-build"}]},
            "components": components,
            "properties": [
                {"name": "opendrobo:license-audit-ready", "value": "false"},
                {"name": "opendrobo:inventory-scope", "value": "locked build inputs; not proof of shipping contents"},
                {"name": "opendrobo:full-schema-validation", "value": "not-run"},
                {"name": "opendrobo:source-lockfile-sha256", "value": json.dumps(hashes, sort_keys=True)},
            ]}


def write_inventory(outdir: Path, bom: dict[str, object]) -> Path:
    outdir.mkdir(parents=True, exist_ok=True)
    output = outdir / "opendrobo-prebuild.cdx.json"
    with output.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(bom, indent=2, sort_keys=True) + "\n")
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--outdir", required=True, type=Path)
    parser.add_argument("--cargo-home", type=Path)
    parser.add_argument("--cargo-metadata", type=Path)
    args = parser.parse_args()
    output = write_inventory(args.outdir, inventory(cargo_home=args.cargo_home, cargo_metadata=args.cargo_metadata))
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
