"""Read checked-in desktop release metadata without changing repository state."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

from desktop_tools import ROOT


class MetadataError(ValueError):
    """Checked-in package metadata is absent or disagrees."""


def canonical_version(version: str) -> str:
    """Allow only updater-compatible numeric X.Y.Z release versions."""
    components = version.split(".")
    if len(components) != 3:
        raise MetadataError(f"Invalid desktop version: {version}")
    limit = 2**64 - 1
    for component in components:
        if not component.isascii() or not component.isdecimal() or (len(component) > 1 and component[0] == "0"):
            raise MetadataError(f"Invalid desktop version: {version}")
        if int(component) > limit:
            raise MetadataError(f"Invalid desktop version: {version}")
    return version


def _json_version(path: Path, *keys: str) -> str:
    try:
        value = json.loads(path.read_text())
        for key in keys:
            value = value[key]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise MetadataError(f"Cannot read version from {path}") from error
    if not isinstance(value, str) or not value:
        raise MetadataError(f"Invalid version in {path}")
    return value


def _cargo_version(path: Path) -> str:
    in_package = False
    try:
        for line in path.read_text().splitlines():
            stripped = line.strip()
            if stripped.startswith("["):
                in_package = stripped == "[package]"
            elif in_package and stripped.startswith("version"):
                _, value = stripped.split("=", 1)
                version = value.strip().strip('"')
                if version:
                    return version
    except OSError as error:
        raise MetadataError(f"Cannot read version from {path}") from error
    raise MetadataError(f"Cannot read version from {path}")


def desktop_metadata(root: Path = ROOT) -> dict[str, object]:
    """Return one verified version plus sources needed for a release receipt."""
    desktop = root / "desktop"
    sources = {
        "desktop/package.json": _json_version(desktop / "package.json", "version"),
        "desktop/src-tauri/tauri.conf.json": _json_version(
            desktop / "src-tauri/tauri.conf.json", "version"),
        "desktop/src-tauri/Cargo.toml": _cargo_version(desktop / "src-tauri/Cargo.toml"),
        "desktop/package-lock.json": _json_version(desktop / "package-lock.json", "version"),
        "desktop/package-lock.json#root": _json_version(
            desktop / "package-lock.json", "packages", "", "version"),
    }
    versions = set(sources.values())
    if len(versions) != 1:
        detail = ", ".join(f"{path}={version}" for path, version in sources.items())
        raise MetadataError(f"Desktop version parity failed: {detail}")
    version = canonical_version(versions.pop())
    try:
        config = json.loads((desktop / "src-tauri/tauri.conf.json").read_text())
        minimum_macos = config["bundle"]["macOS"]["minimumSystemVersion"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise MetadataError("Cannot read macOS deployment target") from error
    if not isinstance(minimum_macos, str) or not minimum_macos:
        raise MetadataError("Invalid macOS deployment target")
    return {"version": version, "versions": sources, "minimum_macos": minimum_macos}


def artifact_filename(version: str, architecture: str, *, debug: bool = False,
                      fskit_research: bool = False, local: bool = False) -> str:
    """Name artifacts so debug and research bundles cannot resemble public builds."""
    labels = ["OpenDrobo", version]
    if debug:
        labels.append("debug")
    if fskit_research:
        labels.append("fskit-research")
    if local or debug or fskit_research:
        labels.append("local")
    labels.extend(("macos", architecture))
    return "-".join(labels) + ".zip"


def sha256_file(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def artifact_manifest(artifacts: list[Path], root: Path) -> dict[str, object]:
    """Create deterministic local hash rows; this is not an SBOM."""
    rows = []
    for artifact in sorted(artifacts, key=lambda item: item.name):
        rows.append({"path": artifact.relative_to(root).as_posix(), "sha256": sha256_file(artifact)})
    return {"schema": "opendrobo-artifact-manifest-v1", "artifacts": rows}
