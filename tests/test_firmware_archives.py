from pathlib import Path
from tempfile import TemporaryDirectory
import importlib.util
import stat
import unittest
from unittest.mock import patch
import zipfile

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/acquire_firmware.py"
SPEC = importlib.util.spec_from_file_location("acquire_firmware", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class FirmwareArchiveTests(unittest.TestCase):
    def reject_entries(self, entries):
        with TemporaryDirectory() as folder:
            root = Path(folder)
            archive = root / "firmware.zip"
            with zipfile.ZipFile(archive, "w"):
                pass
            with patch.object(zipfile.ZipFile, "infolist", return_value=entries):
                with self.assertRaises(ValueError):
                    MODULE.extract_regular_files(archive, root / "output")

    def exercise(self, name, *, symlink=False, existing=False):
        with TemporaryDirectory() as folder:
            root = Path(folder)
            archive = root / "firmware.zip"
            with zipfile.ZipFile(archive, "w") as bundle:
                if symlink:
                    info = zipfile.ZipInfo(name)
                    info.external_attr = (stat.S_IFLNK | 0o777) << 16
                    bundle.writestr(info, "../../outside")
                else:
                    bundle.writestr(name, b"firmware")
            destination = root / "output"
            if existing:
                destination.mkdir()
                (destination / name).write_bytes(b"original")
            return MODULE.extract_regular_files(archive, destination)

    def test_regular_file(self):
        self.assertEqual(self.exercise("firmware.tdf"), ["firmware.tdf"])

    def test_traversal(self):
        with self.assertRaises(ValueError): self.exercise("../outside")

    def test_absolute_path(self):
        with self.assertRaises(ValueError): self.exercise("/tmp/outside")

    def test_backslash_path(self):
        with self.assertRaises(ValueError): self.exercise("..\\outside")

    def test_symlink(self):
        with self.assertRaises(ValueError): self.exercise("link", symlink=True)

    def test_overwrite(self):
        with self.assertRaises(FileExistsError): self.exercise("firmware.tdf", existing=True)

    def test_fifo_and_device_modes(self):
        for mode in (stat.S_IFIFO, stat.S_IFCHR):
            with self.subTest(mode=mode):
                entry = zipfile.ZipInfo("special")
                entry.external_attr = (mode | 0o600) << 16
                self.reject_entries([entry])

    def test_duplicate_paths(self):
        self.reject_entries([zipfile.ZipInfo("same"), zipfile.ZipInfo("same")])

    def test_per_file_limit(self):
        entry = zipfile.ZipInfo("oversized")
        entry.file_size = 1_073_741_825
        self.reject_entries([entry])

    def test_total_limit(self):
        entries = [zipfile.ZipInfo(name) for name in ("one", "two", "three")]
        entries[0].file_size = entries[1].file_size = 1_073_741_824
        entries[2].file_size = 1
        self.reject_entries(entries)


if __name__ == "__main__":
    unittest.main()
