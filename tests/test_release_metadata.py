"""Release names and checked-in version parity stay deterministic."""
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from build_desktop import signing_arguments, tauri_build_command, tauri_target
from release_metadata import MetadataError, artifact_filename, desktop_metadata
from release_preflight import macho_metadata


class ReleaseMetadataTests(unittest.TestCase):
    def write_metadata(self, root: Path, *, package_version="1.0.0"):
        desktop = root / "desktop"
        (desktop / "src-tauri").mkdir(parents=True)
        (desktop / "package.json").write_text(json.dumps({"version": package_version}))
        (desktop / "package-lock.json").write_text(json.dumps({
            "version": package_version, "packages": {"": {"version": package_version}}}))
        (desktop / "src-tauri/tauri.conf.json").write_text(json.dumps({
            "version": package_version,
            "bundle": {"macOS": {"minimumSystemVersion": "14.0"}},
        }))
        (desktop / "src-tauri/Cargo.toml").write_text(
            f"[package]\nname = 'opendrobo-desktop'\nversion = \"{package_version}\"\n")

    def test_checked_metadata_has_one_version(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.write_metadata(root)
            metadata = desktop_metadata(root)
            self.assertEqual(metadata["version"], "1.0.0")
            self.assertEqual(metadata["minimum_macos"], "14.0")

    def test_version_disagreement_is_a_release_gate(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.write_metadata(root)
            path = root / "desktop/src-tauri/Cargo.toml"
            path.write_text(path.read_text().replace("1.0.0", "1.0.1"))
            with self.assertRaisesRegex(MetadataError, "parity failed"):
                desktop_metadata(root)

    def test_noncanonical_or_oversized_version_is_a_release_gate(self):
        for version in ("1.0", "1.00.0", "1.0.0-beta", "18446744073709551616.0.0"):
            with self.subTest(version=version), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                self.write_metadata(root, package_version=version)
                with self.assertRaisesRegex(MetadataError, "Invalid desktop version"):
                    desktop_metadata(root)

    def test_public_and_nonpublic_artifact_names_differ(self):
        self.assertEqual(artifact_filename("1.0.0", "arm64"), "OpenDrobo-1.0.0-macos-arm64.zip")
        self.assertEqual(artifact_filename("1.0.0", "arm64", local=True),
                         "OpenDrobo-1.0.0-local-macos-arm64.zip")
        self.assertIn("debug-local", artifact_filename("1.0.0", "arm64", debug=True))
        self.assertIn("fskit-research-local",
                      artifact_filename("1.0.0", "arm64", fskit_research=True))

    def test_release_command_omits_debug_unless_explicit(self):
        self.assertNotIn("--debug", tauri_build_command(debug=False, explicit_target=None))
        self.assertEqual(tauri_build_command(debug=True, explicit_target="x86_64-apple-darwin")[-3:],
                         ["--debug", "--target", "x86_64-apple-darwin"])
        self.assertEqual(signing_arguments("-", runtime=False), ["codesign", "--force", "--sign", "-"])
        self.assertIn("--timestamp", signing_arguments("Developer ID Application: Test", runtime=True))

    def test_explicit_native_target_uses_active_toolchain_without_cross_target_probe(self):
        native = "aarch64-apple-darwin"
        self.assertIsNone(tauri_target(native, native))
        self.assertNotIn("--target", tauri_build_command(debug=False, explicit_target=tauri_target(native, native)))
        cross = "x86_64-apple-darwin"
        self.assertEqual(tauri_target(cross, native), cross)
        self.assertIn(cross, tauri_build_command(debug=False, explicit_target=tauri_target(cross, native)))

    def test_macho_reader_reports_architecture_and_macos_floor_without_execution(self):
        # One little-endian 64-bit arm64 header with LC_BUILD_VERSION macOS 14.0.
        header = struct.pack("<IiiIIIII", 0xfeedfacf, 0x0100000C, 0, 2, 1, 24, 0, 0)
        command = struct.pack("<IIIIII", 0x32, 24, 1, 0x000e0000, 0, 0)
        with tempfile.TemporaryDirectory() as temporary:
            executable = Path(temporary) / "OpenDrobo"
            executable.write_bytes(header + command)
            self.assertEqual(macho_metadata(executable), [{"architecture": "arm64", "minimum_macos": "14.0.0"}])
