"""Locked dependency inventory stays deterministic and never fills unknown licenses."""
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from dependency_inventory import cached_cargo_license, inventory, write_inventory, safe_license, MAX_TOML_BYTES


class DependencyInventoryTests(unittest.TestCase):
    def write_locks(self, root: Path):
        (root / "desktop/src-tauri").mkdir(parents=True)
        (root / "Cargo.lock").write_text('version = 4\n[[package]]\nname = "known"\nversion = "1.0.0"\n')
        (root / "desktop/src-tauri/Cargo.lock").write_text(
            'version = 4\n[[package]]\nname = "known"\nversion = "1.0.0"\n')
        (root / "desktop/package-lock.json").write_text(json.dumps({"packages": {
            "node_modules/unknown": {"version": "2.0.0"},
            "node_modules/known-npm": {"version": "3.0.0", "license": "MIT"},
        }}))

    def test_known_unknown_conflict_and_dedup_are_deterministic(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.write_locks(root)
            cache = root / "cargo/registry/src/index/known-1.0.0"
            cache.mkdir(parents=True)
            (cache / "Cargo.toml").write_text('[package]\nlicense = "Apache-2.0"\n')
            bom = inventory(root, cargo_home=root / "cargo")
            self.assertEqual([(item["name"], item["version"]) for item in bom["components"]],
                             [("known", "1.0.0"), ("known-npm", "3.0.0"), ("unknown", "2.0.0")])
            known = bom["components"][0]
            self.assertEqual(known["licenses"], [{"license": {"name": "Apache-2.0"}}])
            unknown = bom["components"][2]
            self.assertNotIn("licenses", unknown)
            self.assertIn("unknown", unknown["properties"][0]["value"])
            self.assertFalse(any(item["properties"][2]["value"] == "true" for item in bom["components"]))
            second = inventory(root, cargo_home=root / "cargo")
            self.assertEqual(json.dumps(bom, sort_keys=True), json.dumps(second, sort_keys=True))

    def test_conflicting_overlapping_declared_licenses_are_marked(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.write_locks(root)
            metadata = root / "metadata.json"
            metadata.write_text(json.dumps({"packages": [{"name": "known", "version": "1.0.0", "license": "MIT"}]}))
            cache = root / "cargo/registry/src/index/known-1.0.0"
            cache.mkdir(parents=True)
            (cache / "Cargo.toml").write_text('[package]\nlicense = "Apache-2.0"\n')
            known = inventory(root, cargo_home=root / "cargo", cargo_metadata=metadata)["components"][0]
            self.assertEqual(known["properties"][2]["value"], "true")

    def test_cache_path_rejects_unsafe_name_and_output_preserves_other_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.write_locks(root)
            with self.assertRaises(ValueError):
                cached_cargo_license(root / "cargo", "../../private", "1.0.0")
            outdir = root / "out"
            outdir.mkdir()
            keep = outdir / "keep.txt"
            keep.write_text("keep")
            output = write_inventory(outdir, inventory(root, cargo_home=root / "missing-cache"))
            self.assertTrue(output.is_file())
            self.assertEqual(keep.read_text(), "keep")
            written = json.loads(output.read_text())
            self.assertEqual((written["bomFormat"], written["specVersion"]), ("CycloneDX", "1.6"))
            self.assertEqual(written["metadata"]["lifecycles"], [{"phase": "pre-build"}])

    def test_unsafe_license_text_and_oversize_cache_remain_unknown(self):
        for value in ["SEE LICENSE IN https://private.example/token", "/Users/private/license", "MIT\nSECRET", "MIT\nOR\nApache-2.0", "MIT\tOR Apache-2.0", "x" * 257, "LicenseRef-private-secret"]:
            self.assertIsNone(safe_license(value))
        self.assertEqual(safe_license("MIT OR Apache-2.0"), "MIT OR Apache-2.0")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "registry/src/index/pkg-1.0.0"
            source.mkdir(parents=True)
            (source / "Cargo.toml").write_bytes(b" " * (MAX_TOML_BYTES + 1))
            self.assertIsNone(cached_cargo_license(root, "pkg", "1.0.0"))

    def test_existing_inventory_receipt_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = write_inventory(root, {"original": True})
            with self.assertRaises(FileExistsError): write_inventory(root, {"original": False})
            self.assertEqual(json.loads(first.read_text()), {"original": True})


if __name__ == "__main__":
    unittest.main()
