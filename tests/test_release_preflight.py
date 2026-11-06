"""Release preflight tests use fixture metadata and mocked readonly inspectors."""
import json
import hashlib
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from release_preflight import preflight
from release_notices import NOTICE_INPUTS, read_notice_bundle


class ReleasePreflightTests(unittest.TestCase):
    def write_repo_metadata(self, root: Path):
        desktop = root / "desktop"
        (desktop / "src-tauri").mkdir(parents=True)
        for name, value in (("package.json", {"version": "1.0.0"}),
                            ("package-lock.json", {"version": "1.0.0", "packages": {"": {"version": "1.0.0"}}}),
                            ("src-tauri/tauri.conf.json", {"version": "1.0.0", "bundle": {"macOS": {"minimumSystemVersion": "14.0"}}})):
            (desktop / name).write_text(json.dumps(value))
        (desktop / "src-tauri/Cargo.toml").write_text("[package]\nversion = \"1.0.0\"\n")

        (desktop / "src-tauri/Cargo.lock").write_text("fixture locked dependencies")
        (root / "LICENSE").write_text("OpenDrobo MIT fixture")

    def write_app(self, root: Path, **build):
        app = root / "OpenDrobo.app"
        resources = app / "Contents/Resources"
        executable = app / "Contents/MacOS/OpenDrobo"
        resources.mkdir(parents=True)
        executable.parent.mkdir(parents=True)
        executable.write_bytes(b"fixture")
        (app / "Contents/Info.plist").write_bytes(plistlib.dumps({
            "CFBundleExecutable": "OpenDrobo", "CFBundleShortVersionString": "1.0.0",
            "CFBundleVersion": "1.0.0",
            "LSMinimumSystemVersion": "14.0"}))
        defaults = {"build_profile": "release", "fskit_research": False,
                    "artwork_source": "generated-fallback",
                    "signing_identity": "Developer ID Application: OpenDrobo",
                    "architecture": "arm64", "target": "aarch64-apple-darwin",
                    "minimum_macos": "14.0"}
        licenses = resources / "licenses"
        licenses.mkdir()
        manifest = {"schemaVersion": 1, "components": [{"name": "fixture"}], "missingLicenseText": [],
                    "noticeReady": True, "legalReviewRequired": True, "defaultGraphVerified": True,
                    "graphVerification": {"aarch64-apple-darwin": {"command": "cargo metadata --offline --locked --format-version 1 --filter-platform aarch64-apple-darwin --manifest-path desktop/src-tauri/Cargo.toml"}},
                    "metadataSha256": {"aarch64-apple-darwin": "a" * 64},
                    "inputSha256": {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in NOTICE_INPUTS}}
        (licenses / "third-party-manifest.json").write_text(json.dumps(manifest))
        (licenses / "THIRD_PARTY_NOTICES.md").write_text("Copyright fixture\nLicense terms\n")
        (resources / "LICENSE.txt").write_bytes((root / "LICENSE").read_bytes())
        defaults["third_party_notices"] = read_notice_bundle(licenses, root=root)[0]
        defaults.update(build)
        (resources / "BUILD_INFO.json").write_text(json.dumps(defaults))
        return app

    @staticmethod
    def macho(_):
        return [{"architecture": "arm64", "minimum_macos": "14.0.0"}]

    @staticmethod
    def developer_id(_):
        return {"verified": True, "team_id": "TEAM123", "authority": "Developer ID Application: OpenDrobo"}

    @staticmethod
    def release_proof(arguments, **_):
        return subprocess.CompletedProcess(arguments, 0)

    def test_distribution_receipt_passes_only_all_public_gates(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            self.write_repo_metadata(root)
            receipt = preflight(self.write_app(root), root=root, inspect_macho=self.macho,
                                inspect_codesign=self.developer_id, check_stapler=True,
                                check_spctl=True, command=self.release_proof)
            self.assertTrue(receipt["distributionReady"])

    def test_unrun_notarization_or_gatekeeper_check_blocks_distribution_ready(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            self.write_repo_metadata(root)
            receipt = preflight(self.write_app(root), root=root, inspect_macho=self.macho,
                                inspect_codesign=self.developer_id)
            self.assertFalse(receipt["distributionReady"])
            failed = {gate["gate"] for gate in receipt["gates"] if not gate["passed"]}
            self.assertEqual(failed, {"stapler", "spctl"})

    def test_public_preflight_rejects_debug_research_vendor_and_missing_team(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            self.write_repo_metadata(root)
            receipt = preflight(self.write_app(root, build_profile="debug", fskit_research=True,
                                               artwork_source="local-vendor"), root=root,
                                inspect_macho=self.macho,
                                inspect_codesign=lambda _: {"verified": True, "team_id": None, "authority": None})
            self.assertFalse(receipt["distributionReady"])
            failed = {gate["gate"] for gate in receipt["gates"] if not gate["passed"]}
            self.assertTrue({"release-profile", "no-research-module", "no-local-vendor-artwork", "developer-id-team"} <= failed)

    def test_local_structural_never_claims_distribution_ready(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            self.write_repo_metadata(root)
            receipt = preflight(self.write_app(root), root=root, local_structural=True,
                                inspect_macho=self.macho, inspect_codesign=self.developer_id)
            self.assertFalse(receipt["distributionReady"])
            self.assertTrue(receipt["localStructural"])

    def test_mislabeled_target_or_architecture_blocks_distribution(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            self.write_repo_metadata(root)
            receipt = preflight(self.write_app(root, architecture="x86_64"), root=root,
                                inspect_macho=self.macho, inspect_codesign=self.developer_id,
                                check_stapler=True, check_spctl=True, command=self.release_proof)
            self.assertFalse(receipt["distributionReady"])
            failed = {gate["gate"] for gate in receipt["gates"] if not gate["passed"]}
            self.assertTrue({"build-target", "macho"} <= failed)

    def test_deployment_floor_or_tool_errors_block_with_receipt(self):
        def unavailable(*_, **__):
            raise FileNotFoundError("tool unavailable")

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            self.write_repo_metadata(root)
            receipt = preflight(self.write_app(root, minimum_macos="15.0"), root=root,
                                inspect_macho=lambda _: [{"architecture": "arm64", "minimum_macos": "15.0.0"}],
                                inspect_codesign=unavailable, check_stapler=True,
                                check_spctl=True, command=unavailable)
            self.assertFalse(receipt["distributionReady"])
            failed = {gate["gate"] for gate in receipt["gates"] if not gate["passed"]}
            self.assertTrue({"build-target", "macho", "codesign-verify", "developer-id-team", "stapler", "spctl"} <= failed)

    def test_bundle_floor_above_macos_14_blocks_distribution(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            self.write_repo_metadata(root)
            app = self.write_app(root)
            info_path = app / "Contents/Info.plist"
            info = plistlib.loads(info_path.read_bytes())
            info["LSMinimumSystemVersion"] = "15.0"
            info_path.write_bytes(plistlib.dumps(info))
            receipt = preflight(app, root=root, inspect_macho=self.macho,
                                inspect_codesign=self.developer_id, check_stapler=True,
                                check_spctl=True, command=self.release_proof)
            self.assertFalse(receipt["distributionReady"])
            failed = {gate["gate"] for gate in receipt["gates"] if not gate["passed"]}
            self.assertIn("bundle-minimum-macos", failed)

    def test_missing_or_changed_notices_block_otherwise_valid_distribution(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            self.write_repo_metadata(root)
            app = self.write_app(root)
            (app / "Contents/Resources/licenses/THIRD_PARTY_NOTICES.md").unlink()
            receipt = preflight(app, root=root, inspect_macho=self.macho,
                                inspect_codesign=self.developer_id, check_stapler=True,
                                check_spctl=True, command=self.release_proof)
            self.assertFalse(receipt["distributionReady"])
            failed = {gate["gate"] for gate in receipt["gates"] if not gate["passed"]}
            self.assertEqual(failed, {"third-party-notices"})
