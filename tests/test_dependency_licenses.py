"""Selected dependency notice receipts use locked archive bytes, never source trees."""
from __future__ import annotations

import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from dependency_licenses import _verify_cli_graph, build_manifest, main, write_outputs

REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"


def _tar(entries: dict[str, bytes], *, symlink: str | None = None) -> bytes:
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode="w:gz") as archive:
        for name, data in entries.items():
            member = tarfile.TarInfo(name); member.size = len(data)
            archive.addfile(member, io.BytesIO(data))
        if symlink:
            member = tarfile.TarInfo(symlink); member.type = tarfile.SYMTYPE; member.linkname = "outside"
            archive.addfile(member)
    return output.getvalue()


def _metadata(root: str) -> dict:
    desktop = f"path+file:///{root}/desktop#opendrobo-desktop@1.0.0"
    alpha = f"{REGISTRY}#alpha@1.0.0"
    build = f"{REGISTRY}#build-only@2.0.0"
    packages = [
        {"id": desktop, "name": "opendrobo-desktop", "version": "1.0.0", "source": None, "license": "MIT"},
        {"id": alpha, "name": "alpha", "version": "1.0.0", "source": REGISTRY, "license": "MIT"},
        {"id": build, "name": "build-only", "version": "2.0.0", "source": REGISTRY, "license": "Apache-2.0"},
    ]
    return {"workspace_default_members": [desktop], "packages": packages, "resolve": {"nodes": [
        {"id": desktop, "deps": [{"name": "alpha", "pkg": alpha, "dep_kinds": [{"kind": None, "target": None}]}, {"name": "build-only", "pkg": build, "dep_kinds": [{"kind": "build", "target": None}]}], "features": []},
        {"id": alpha, "deps": [], "features": []}, {"id": build, "deps": [], "features": []},
    ]}}


class DependencyLicenseTests(unittest.TestCase):
    def _crate(self, root: Path, name: str, version: str, license_value: str, docs: dict[str, str], *, vcs: str | None = None, symlink: str | None = None) -> str:
        prefix = f"{name}-{version}/"
        entries = {prefix + "Cargo.toml": f'[package]\nname = "{name}"\nversion = "{version}"\nlicense = "{license_value}"\n'.encode()}
        entries.update({prefix + key: value.encode() for key, value in docs.items()})
        if vcs:
            entries[prefix + ".cargo_vcs_info.json"] = json.dumps({"git": {"sha1": vcs}}).encode()
        data = _tar(entries, symlink=prefix + symlink if symlink else None)
        directory = root / "cargo/registry/cache/test-index"; directory.mkdir(parents=True, exist_ok=True)
        (directory / f"{name}-{version}.crate").write_bytes(data)
        return hashlib.sha256(data).hexdigest()

    def _npm(self, root: Path, docs: dict[str, str], *, license_value: str = "ISC") -> str:
        entries = {"package/package.json": json.dumps({"name": "@scope/pkg", "version": "3.0.0", "license": license_value}).encode()}
        entries.update({"package/" + key: value.encode() for key, value in docs.items()})
        data = _tar(entries)
        digest = hashlib.sha512(data).digest(); encoded = digest.hex()
        location = root / "npm/_cacache/content-v2/sha512" / encoded[:2] / encoded[2:4]
        location.mkdir(parents=True); (location / encoded[4:]).write_bytes(data)
        return "sha512-" + base64.b64encode(digest).decode()

    def _fixture(self, root: Path, *, cargo_docs: dict[str, str] | None = None, npm_docs: dict[str, str] | None = None) -> dict[str, Path | list[tuple[str, Path]]]:
        alpha = self._crate(root, "alpha", "1.0.0", "MIT", {"LICENSE-MIT": "Copyright Alpha\nMIT License\n"} if cargo_docs is None else cargo_docs, vcs="a" * 40)
        build = self._crate(root, "build-only", "2.0.0", "Apache-2.0", {"COPYING": "Copyright Build\nApache License\n"})
        lock = root / "Cargo.lock"
        lock.write_text("version = 4\n\n[[package]]\nname = \"opendrobo-desktop\"\nversion = \"1.0.0\"\n\n" + "\n".join([
            f'[[package]]\nname = "alpha"\nversion = "1.0.0"\nsource = "{REGISTRY}"\nchecksum = "{alpha}"\n',
            f'[[package]]\nname = "build-only"\nversion = "2.0.0"\nsource = "{REGISTRY}"\nchecksum = "{build}"\n',
        ]))
        integrity = self._npm(root, {"LICENSE": "Copyright Npm\nISC License\n"} if npm_docs is None else npm_docs)
        npm_lock = root / "package-lock.json"
        npm_lock.write_text(json.dumps({"packages": {"": {"dependencies": {"@scope/pkg": "3.0.0"}}, "node_modules/@scope/pkg": {"version": "3.0.0", "license": "ISC", "integrity": integrity}}}))
        desktop = root / "package.json"; desktop.write_text(json.dumps({"dependencies": {"@scope/pkg": "3.0.0"}}))
        paths = []
        for target in ("aarch64-apple-darwin", "x86_64-apple-darwin"):
            path = root / f"{target}.json"; path.write_text(json.dumps(_metadata(str(root)))); paths.append((target, path))
        return {"metadata": paths, "cargo": root / "cargo", "lock": lock, "npm_lock": npm_lock, "desktop": desktop, "npm": root / "npm"}

    def _build(self, fixture: dict[str, Path | list[tuple[str, Path]]], **kwargs: object) -> dict:
        return build_manifest(fixture["metadata"], cargo_home=fixture["cargo"], cargo_lock=fixture["lock"], npm_lock=fixture["npm_lock"], desktop_package=fixture["desktop"], npm_cache=fixture["npm"], npm_runtime=["@scope/pkg"], **kwargs)

    def test_verified_archives_produce_documentary_coverage(self):
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary)); manifest = self._build(fixture)
            self.assertTrue(manifest["noticeReady"])
            self.assertFalse(manifest["defaultGraphVerified"])
            self.assertTrue(manifest["legalReviewRequired"])
            self.assertEqual(len(manifest["components"]), 3)
            self.assertEqual(set(manifest["inputSha256"]), {"desktop/src-tauri/Cargo.lock", "desktop/package-lock.json", "desktop/package.json"})
            self.assertEqual(next(item for item in manifest["components"] if item["name"] == "alpha")["licenseDocuments"][0]["provenance"], "verified-cargo-archive")
            verified = self._build(fixture, graph_verifier=lambda target, _: {"command": target, "cargoVersion": "test"})
            self.assertTrue(verified["defaultGraphVerified"])

    def test_notice_only_never_satisfies_license_terms(self):
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary), cargo_docs={"NOTICE": "Copyright Alpha\n"})
            manifest = self._build(fixture)
            self.assertFalse(manifest["noticeReady"])
            self.assertIn("cargo:alpha@1.0.0", manifest["missingLicenseText"])

    def test_british_licence_is_terms_document(self):
        with tempfile.TemporaryDirectory() as temporary:
            fixture = self._fixture(Path(temporary), cargo_docs={"LICENCE": "Copyright Alpha\nMIT License\n"})
            alpha = next(item for item in self._build(fixture)["components"] if item["name"] == "alpha")
            self.assertEqual(alpha["licenseDocuments"][0]["file"], "LICENCE")

    def test_rejects_modified_archive_and_ignores_forged_source_tree(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); fixture = self._fixture(root)
            (root / "cargo/registry/src/test-index/alpha-1.0.0").mkdir(parents=True)
            (root / "cargo/registry/src/test-index/alpha-1.0.0/LICENSE").write_text("forged")
            archive = next((root / "cargo/registry/cache/test-index").glob("alpha-*.crate")); archive.write_bytes(archive.read_bytes() + b"changed")
            with self.assertRaisesRegex(ValueError, "checksum"):
                self._build(fixture)

    def test_rejects_archive_symlink_and_duplicate_target_metadata(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); fixture = self._fixture(root)
            checksum = self._crate(root, "alpha", "1.0.0", "MIT", {"LICENSE": "Copyright Alpha"}, symlink="LICENSE-link")
            lock = fixture["lock"]; text = lock.read_text(); old = re_search_checksum(text, "alpha"); lock.write_text(text.replace(old, checksum))
            with self.assertRaisesRegex(ValueError, "unsupported member"):
                self._build(fixture)
            with self.assertRaisesRegex(ValueError, "exactly once"):
                self._build({**fixture, "metadata": [fixture["metadata"][0], fixture["metadata"][0]]})

    def test_rejects_metadata_package_not_in_lock_and_npm_integrity_mismatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); fixture = self._fixture(root)
            data = json.loads(fixture["metadata"][0][1].read_text()); data["packages"][1]["version"] = "9.0.0"; fixture["metadata"][0][1].write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, "exactly locked"):
                self._build(fixture)
            fixture = self._fixture(root / "second")
            lock = json.loads(fixture["npm_lock"].read_text()); lock["packages"]["node_modules/@scope/pkg"]["integrity"] = "sha512-" + base64.b64encode(b"x" * 64).decode(); fixture["npm_lock"].write_text(json.dumps(lock))
            with self.assertRaisesRegex(ValueError, "integrity"):
                self._build(fixture)

    def test_rejects_archive_traversal_oversize_member_and_missing_lock_checksum(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); fixture = self._fixture(root)
            prefix = "alpha-1.0.0/"
            data = _tar({prefix + "Cargo.toml": b'[package]\nname = "alpha"\nversion = "1.0.0"\nlicense = "MIT"\n', prefix + "../outside": b"bad"})
            archive = next((root / "cargo/registry/cache/test-index").glob("alpha-*.crate")); archive.write_bytes(data)
            lock = fixture["lock"]; text = lock.read_text(); lock.write_text(text.replace(re_search_checksum(text, "alpha"), hashlib.sha256(data).hexdigest()))
            with self.assertRaisesRegex(ValueError, "member path"):
                self._build(fixture)
            checksum = self._crate(root, "alpha", "1.0.0", "MIT", {"LICENSE": "x" * (256 * 1024 + 1)})
            text = lock.read_text(); lock.write_text(text.replace(re_search_checksum(text, "alpha"), checksum))
            with self.assertRaisesRegex(ValueError, "selected member"):
                self._build(fixture)
            text = lock.read_text(); lock.write_text(text.replace(f'checksum = "{checksum}"', 'checksum = "not-a-checksum"'))
            with self.assertRaisesRegex(ValueError, "checksum"):
                self._build(fixture)

    def test_supplement_requires_matching_verified_vcs_and_outputs_are_exclusive(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); fixture = self._fixture(root, cargo_docs={})
            text = "Copyright Alpha\nMIT License\n"; supplement = root / "supplement.json"
            record = {"schemaVersion": 1, "supplements": [{"ecosystem": "cargo", "name": "alpha", "version": "1.0.0", "file": "LICENSE", "originCommit": "a" * 40, "sourceUrl": "https://raw.githubusercontent.com/example/repo/" + "a" * 40 + "/LICENSE", "text": text, "sha256": hashlib.sha256(text.encode()).hexdigest()}]}
            supplement.write_text(json.dumps(record)); manifest = self._build(fixture, supplemental_license=[supplement])
            self.assertTrue(manifest["noticeReady"])
            out = root / "out"; out.mkdir(); (out / "keep").write_text("keep")
            write_outputs(out, manifest)
            with self.assertRaises(FileExistsError): write_outputs(out, manifest)
            record["supplements"][0]["file"] = "NOTICE"
            supplement.write_text(json.dumps(record))
            notice_only = self._build(fixture, supplemental_license=[supplement])
            self.assertFalse(notice_only["noticeReady"])
            self.assertIn("cargo:alpha@1.0.0", notice_only["missingLicenseText"])
            record["supplements"][0]["file"] = "LICENSE"
            record["supplements"][0]["originCommit"] = "b" * 40
            record["supplements"][0]["sourceUrl"] = "https://raw.githubusercontent.com/example/repo/" + "b" * 40 + "/LICENSE"
            supplement.write_text(json.dumps(record))
            with self.assertRaisesRegex(ValueError, "VCS receipt"):
                self._build(fixture, supplemental_license=[supplement])
            local_terms = self._fixture(root / "local-terms")
            record["supplements"][0]["originCommit"] = "a" * 40
            record["supplements"][0]["sourceUrl"] = "https://raw.githubusercontent.com/example/repo/" + "a" * 40 + "/LICENSE"
            supplement.write_text(json.dumps(record))
            with self.assertRaisesRegex(ValueError, "cannot alter"):
                self._build(local_terms, supplemental_license=[supplement])

    def test_cli_refuses_cache_and_lock_overrides_before_cargo_execution(self):
        stderr = io.StringIO()
        arguments = ["dependency_licenses.py", "--metadata", "aarch64-apple-darwin=x", "--metadata", "x86_64-apple-darwin=y", "--outdir", "z", "--cargo-lock", "outside"]
        with patch.object(sys, "argv", arguments), contextlib.redirect_stderr(stderr):
            with self.assertRaises(SystemExit) as error:
                main()
        self.assertEqual(error.exception.code, 2)
        self.assertIn("unrecognized arguments: --cargo-lock outside", stderr.getvalue())

    def test_graph_reproduction_rejects_metadata_mismatch(self):
        supplied = _metadata("fixture")
        fresh = _metadata("fixture")
        fresh["resolve"]["nodes"][0]["features"] = ["unexpected"]
        responses = [SimpleNamespace(returncode=0, stdout=json.dumps(fresh).encode(), stderr=b""),
                     SimpleNamespace(returncode=0, stdout=b"cargo test", stderr=b"")]
        with patch("dependency_licenses.subprocess.run", side_effect=responses):
            with self.assertRaisesRegex(ValueError, "does not match"):
                _verify_cli_graph("aarch64-apple-darwin", supplied)


def re_search_checksum(text: str, name: str) -> str:
    import re
    match = re.search(rf'name = "{name}".*?checksum = "([0-9a-f]+)"', text, re.DOTALL)
    if match is None:
        raise AssertionError("test lock is malformed")
    return match.group(1)


if __name__ == "__main__":
    unittest.main()
