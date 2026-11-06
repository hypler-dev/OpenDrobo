import contextlib
import io
import importlib.util
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "spare_unit_lab.py"
NOW_MS = 1_800_000_000_000
SPEC = importlib.util.spec_from_file_location("spare_unit_lab", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(MODULE)


def fingerprint(value):
    return "sha256:" + hashlib.sha256(value.encode()).hexdigest()


def manifest(**changes):
    result = {
        "schemaVersion": 1,
        "target": {"model": "Drobo 5D", "firmware": "4.2.3", "operatingSystem": "macOS 26", "transport": "Thunderbolt"},
        "unit": {"opaqueFingerprint": fingerprint("unit-sha256-0123456789abcdef")},
        "disposableMemberDisks": [fingerprint("disk-sha256-0123456789abcdef"), fingerprint("disk-sha256-fedcba9876543210")],
        "exclusions": {
            "protectedUnitFingerprints": [],
            "homeUnitFingerprints": [],
            "protectedDiskFingerprints": [],
        },
    }
    result.update(changes)
    return result


def envelope(unit, disks, *, captured_at_ms=NOW_MS, target=None, payload=None):
    return {
        "capturedAtMs": captured_at_ms,
        "unitFingerprint": unit,
        "memberDiskFingerprints": disks,
        "target": target or manifest()["target"],
        "payload": {"copied": "raw-cli-json"} if payload is None else payload,
    }


class SpareUnitLabTests(unittest.TestCase):
    def test_init_writes_a_gated_template_without_hardware(self):
        with tempfile.TemporaryDirectory() as folder:
            out_dir = Path(folder) / "receipts"
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(MODULE.main(["init", "--out-dir", str(out_dir)]), 0)
            template = json.loads((out_dir / MODULE.MANIFEST_FILE).read_text())
            self.assertEqual(template["unit"]["opaqueFingerprint"], "pending")
            with self.assertRaisesRegex(ValueError, "confirmed non-whitespace"):
                MODULE.validate_manifest(template)

    def test_wrong_or_home_unit_is_refused(self):
        home = manifest(exclusions={"protectedUnitFingerprints": [], "homeUnitFingerprints": [fingerprint("unit-sha256-0123456789abcdef")], "protectedDiskFingerprints": []})
        with self.assertRaisesRegex(ValueError, "protected or home"):
            MODULE.validate_manifest(home)
        protected = manifest(exclusions={"protectedUnitFingerprints": [fingerprint("unit-sha256-0123456789abcdef")], "homeUnitFingerprints": [], "protectedDiskFingerprints": []})
        with self.assertRaisesRegex(ValueError, "protected or home"):
            MODULE.validate_manifest(protected)

    def test_protected_member_disk_is_refused(self):
        protected_member = manifest(exclusions={"protectedUnitFingerprints": [], "homeUnitFingerprints": [], "protectedDiskFingerprints": [fingerprint("disk-sha256-0123456789abcdef")]})
        with self.assertRaisesRegex(ValueError, "protected disk"):
            MODULE.validate_manifest(protected_member)

    def test_duplicate_member_and_missing_identity_are_refused(self):
        duplicate = manifest(disposableMemberDisks=[fingerprint("disk-sha256-0123456789abcdef"), fingerprint("disk-sha256-0123456789abcdef")])
        with self.assertRaisesRegex(ValueError, "duplicate"):
            MODULE.validate_manifest(duplicate)
        missing = manifest(unit={"opaqueFingerprint": "pending"})
        with self.assertRaisesRegex(ValueError, "sha256:"):
            MODULE.validate_manifest(missing)
        with self.assertRaisesRegex(ValueError, "schemaVersion"):
            MODULE.validate_manifest(manifest(schemaVersion=True))
        with self.assertRaisesRegex(ValueError, "schemaVersion"):
            MODULE.validate_manifest(manifest(schemaVersion=1.0))
        for invalid in ["registry-id-0123456789", "1234567890123456789", "sha256:" + "A" * 64, "sha256:" + "0" * 63]:
            with self.assertRaisesRegex(ValueError, "sha256:"):
                MODULE.validate_manifest(manifest(unit={"opaqueFingerprint": invalid}))

    def test_replaced_baseline_blocks_plan(self):
        checked = MODULE.validate_manifest(manifest())
        baseline = MODULE.baseline_state(checked, [("pool", envelope(fingerprint("unit-sha256-replaced-abcdef"), checked["disposableMemberDisks"]), {"envelopeSha256": "a", "bytes": 1})], NOW_MS)
        self.assertEqual(baseline["state"], "BLOCKED")
        self.assertIn("wrong unit", baseline["mismatches"][0])
        changed_members = MODULE.baseline_state(checked, [("pool", envelope(checked["unitFingerprint"], [fingerprint("disk-sha256-replaced-abcdef")]), {"envelopeSha256": "b", "bytes": 1})], NOW_MS)
        self.assertEqual(changed_members["state"], "BLOCKED")
        self.assertIn("member inventory changed", changed_members["mismatches"][0])

    def test_unbound_stale_future_and_wrong_target_evidence_blocks_plan(self):
        checked = MODULE.validate_manifest(manifest())
        cases = [
            ("unbound", {"payload": {}}, "missing required"),
            ("empty-payload", envelope(checked["unitFingerprint"], checked["disposableMemberDisks"], payload={}), "nonempty copied raw CLI JSON"),
            ("stale", envelope(checked["unitFingerprint"], checked["disposableMemberDisks"], captured_at_ms=NOW_MS - 86_400_001), "older than 24 hours"),
            ("future", envelope(checked["unitFingerprint"], checked["disposableMemberDisks"], captured_at_ms=NOW_MS + 300_001), "future"),
            ("wrong-target", envelope(checked["unitFingerprint"], checked["disposableMemberDisks"], target={**checked["target"], "firmware": "other"}), "target metadata changed"),
        ]
        for name, supplied, expected in cases:
            with self.subTest(name=name):
                baseline = MODULE.baseline_state(checked, [("pool", supplied, {"envelopeSha256": name, "bytes": 1})], NOW_MS)
                self.assertEqual(baseline["state"], "BLOCKED")
                self.assertIn(expected, baseline["mismatches"][0])

    def test_plan_is_deterministic_no_hardware_and_ignores_manifest_claim(self):
        checked = MODULE.validate_manifest(manifest(destructiveAuthorized="false"))
        first = MODULE.plan(checked, MODULE.baseline_state(checked, []))
        second = MODULE.plan(checked, MODULE.baseline_state(checked, []))
        self.assertEqual(first, second)
        self.assertEqual(first["baseline"]["state"], "PENDING_HARDWARE")
        self.assertFalse(first["baseline"]["ready"])
        self.assertTrue(first["manifestAuthorizationIgnored"])
        self.assertTrue(first["manifestDestructiveAuthorization"]["present"])
        self.assertFalse(first["manifestDestructiveAuthorization"]["trueValue"])
        self.assertTrue(first["execution"].startswith("BLOCKED"))

    def test_template_plan_has_no_invented_identity_or_evidence(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(MODULE.main(["plan", "--template", "--out-dir", str(root)]), 0)
            generated = json.loads((root / MODULE.PLAN_FILE).read_text())
            self.assertIsNone(generated["unitFingerprint"])
            self.assertEqual(generated["baseline"]["state"], "PENDING_HARDWARE")
            self.assertFalse(generated["baseline"]["ready"])
            evidence = root / "pool.json"
            evidence.write_text("{}")
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(MODULE.main(["plan", "--template", "--out-dir", str(root / "second"), "--pool-json", str(evidence)]), 2)

    def test_cli_binds_complete_envelope_as_operator_asserted_receipt(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "manifest.json"
            copied = root / "copied-pool.json"
            source.write_text(json.dumps(manifest()))
            copied.write_text(json.dumps(envelope(fingerprint("unit-sha256-0123456789abcdef"), manifest()["disposableMemberDisks"])))
            with patch.object(MODULE.time, "time", return_value=NOW_MS / 1000), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(MODULE.main(["plan", "--manifest", str(source), "--out-dir", str(root / "out"), "--pool-json", str(copied)]), 0)
            generated = json.loads((root / "out" / MODULE.PLAN_FILE).read_text())
            self.assertEqual(generated["baseline"]["state"], "PENDING_HARDWARE")
            receipt = generated["baseline"]["receipts"][0]
            self.assertEqual(receipt["identityTrust"], "operator-asserted-not-attested")
            self.assertIsInstance(receipt["bytes"], int)
            self.assertEqual(len(receipt["envelopeSha256"]), 64)

    def test_cli_refuses_execution_flag_and_stale_output(self):
        for flag in ("--execute", "--format", "--reset", "--mount", "--driver-command"):
            with contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):
                    MODULE.main(["plan", flag])
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "manifest.json"
            source.write_text(json.dumps(manifest()))
            linked = root / "linked.json"
            linked.symlink_to(source)
            with self.assertRaisesRegex(ValueError, "non-symlink"):
                MODULE.read_json(str(linked))
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(MODULE.main(["plan", "--manifest", str(source), "--out-dir", str(root)]), 0)
                self.assertEqual(MODULE.main(["plan", "--manifest", str(source), "--out-dir", str(root)]), 2)


if __name__ == "__main__":
    unittest.main()
