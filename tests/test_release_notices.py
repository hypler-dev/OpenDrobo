"""Notice resources must match the current dependency inputs and exact receipt."""
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
from contextlib import redirect_stderr
import io

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from release_notices import NOTICE_INPUTS, bounded_file, inspect_packaged_notices, read_notice_bundle
from build_desktop import main as build_main


class ReleaseNoticesTests(unittest.TestCase):
    def fixture(self, root):
        inputs = {}
        for name in NOTICE_INPUTS:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("locked-public-fixture")
            inputs[name] = hashlib.sha256(path.read_bytes()).hexdigest()
        (root / "LICENSE").write_text("OpenDrobo MIT fixture")
        directory = root / "resources/licenses"
        directory.mkdir(parents=True)
        manifest = {"schemaVersion": 1, "components": [{"name": "example"}], "missingLicenseText": [],
                    "noticeReady": True, "legalReviewRequired": True, "defaultGraphVerified": True,
                    "graphVerification": {"aarch64-apple-darwin": {"command": "cargo metadata --offline --locked --format-version 1 --filter-platform aarch64-apple-darwin --manifest-path desktop/src-tauri/Cargo.toml"}},
                    "metadataSha256": {"aarch64-apple-darwin": "a" * 64},
                    "inputSha256": inputs}
        (directory / "third-party-manifest.json").write_text(json.dumps(manifest))
        (directory / "THIRD_PARTY_NOTICES.md").write_text("# Required notices\nCopyright example\nLicense terms\n")
        (root / "resources/LICENSE.txt").write_bytes((root / "LICENSE").read_bytes())
        return directory, manifest

    def test_absent_and_incomplete_bundles_never_pass_distribution_gate(self):
        self.assertFalse(read_notice_bundle(None)[0]["complete"])
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            directory, manifest = self.fixture(root)
            manifest.update(noticeReady=False, missingLicenseText=["example"])
            (directory / "third-party-manifest.json").write_text(json.dumps(manifest))
            metadata, _ = read_notice_bundle(directory, root=root)
            ok, message = inspect_packaged_notices(root / "resources", {"third_party_notices": metadata, "target": "aarch64-apple-darwin"}, root=root)
            self.assertFalse(ok)
            self.assertIn("lack complete", message)

    def test_complete_documentary_bundle_is_bound_to_exact_receipt_and_own_license(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            directory, _ = self.fixture(root)
            metadata, _ = read_notice_bundle(directory, root=root)
            build = {"third_party_notices": metadata, "target": "aarch64-apple-darwin"}
            self.assertTrue(inspect_packaged_notices(root / "resources", build, root=root)[0])
            (directory / "THIRD_PARTY_NOTICES.md").write_text("Changed required notice")
            self.assertFalse(inspect_packaged_notices(root / "resources", build, root=root)[0])
            (root / "resources/LICENSE.txt").write_text("Other license")
            metadata, _ = read_notice_bundle(directory, root=root)
            self.assertFalse(inspect_packaged_notices(root / "resources", {"third_party_notices": metadata, "target": "aarch64-apple-darwin"}, root=root)[0])

    def test_changed_lock_or_unverified_graph_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            directory, manifest = self.fixture(root)
            (root / NOTICE_INPUTS[0]).write_text("changed lock")
            with self.assertRaisesRegex(ValueError, "does not match"):
                read_notice_bundle(directory, root=root)
            (root / NOTICE_INPUTS[0]).write_text("locked-public-fixture")
            manifest["defaultGraphVerified"] = False
            (directory / "third-party-manifest.json").write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, "graph verification"):
                read_notice_bundle(directory, root=root)

    @unittest.skipUnless(os.name == "posix", "packaging uses POSIX descriptors")
    def test_notice_links_parent_links_hardlinks_and_fifo_are_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            path = root / "notice"
            path.write_text("private-like fixture")
            link = root / "link"
            link.symlink_to(path)
            with self.assertRaises(OSError):
                bounded_file(link)
            parent = root / "parent"
            parent.symlink_to(root, target_is_directory=True)
            with self.assertRaises(OSError):
                bounded_file(parent / "notice")
            hard = root / "hard"
            os.link(path, hard)
            with self.assertRaises(ValueError):
                bounded_file(hard)
            pipe = root / "pipe"
            os.mkfifo(pipe)
            with self.assertRaises(ValueError):
                bounded_file(pipe)

    def test_developer_id_packaging_stops_before_build_or_signing_for_absent_and_partial_notices(self):
        for bundled in (False, True):
            arguments = ["build_desktop.py", "--signing-identity", "Developer ID Application: Fixture"]
            if bundled:
                arguments += ["--third-party-notices", "/reviewed/partial-fixture"]
            metadata = {"bundled": bundled, "complete": False, "legalReviewRequired": True}
            with self.subTest(bundled=bundled), \
                    patch("build_desktop.platform.system", return_value="Darwin"), \
                    patch("sys.argv", arguments), \
                    patch("build_desktop.read_notice_bundle", return_value=(metadata, {})), \
                    patch("build_desktop.run") as command, redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as stopped:
                    build_main()
                self.assertEqual(stopped.exception.code, 2)
                command.assert_not_called()

    def test_unsigned_partial_bundle_can_reach_only_the_mocked_local_build(self):
        metadata = {"bundled": True, "complete": False, "legalReviewRequired": True, "targets": ["aarch64-apple-darwin"]}
        with patch("build_desktop.platform.system", return_value="Darwin"), \
                patch("build_desktop.native_target", return_value="aarch64-apple-darwin"), \
                patch("sys.argv", ["build_desktop.py", "--third-party-notices", "/reviewed/partial-fixture"]), \
                patch("build_desktop.read_notice_bundle", return_value=(metadata, {})), \
                patch("build_desktop.clear_local_artwork"), \
                patch("build_desktop.run", side_effect=RuntimeError("mocked local build reached")) as command:
            with self.assertRaisesRegex(RuntimeError, "mocked local build reached"):
                build_main()
            command.assert_called_once()
            arguments = command.call_args.args[0]
            self.assertEqual(arguments[:5], ["npm", "run", "tauri", "--", "build"])
            self.assertNotIn("codesign", arguments)

    def test_notice_bundle_for_another_architecture_cannot_pass_preflight(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            directory, _ = self.fixture(root)
            metadata, _ = read_notice_bundle(directory, root=root)
            ok, message = inspect_packaged_notices(root / "resources", {
                "third_party_notices": metadata, "target": "x86_64-apple-darwin"}, root=root)
            self.assertFalse(ok)
            self.assertIn("artifact target", message)

    def test_cli_refuses_mismatched_target_before_build(self):
        metadata = {"bundled": True, "complete": True, "legalReviewRequired": True,
                    "targets": ["aarch64-apple-darwin"]}
        with patch("build_desktop.platform.system", return_value="Darwin"), \
                patch("build_desktop.native_target", return_value="aarch64-apple-darwin"), \
                patch("sys.argv", ["build_desktop.py", "--target", "x86_64-apple-darwin", \
                                   "--third-party-notices", "/reviewed/arm-only-fixture"]), \
                patch("build_desktop.read_notice_bundle", return_value=(metadata, {})), \
                patch("build_desktop.run") as command, redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as stopped:
                build_main()
            self.assertEqual(stopped.exception.code, 2)
            command.assert_not_called()
