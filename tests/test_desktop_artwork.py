"""Optional artwork must never package a linked or non-image private file."""
import os
from pathlib import Path
import struct
import sys
import tempfile
import unittest
import zlib

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from build_desktop import MAX_ARTWORK_BYTES, artwork_bytes


def chunk(name, data):
    return struct.pack(">I", len(data)) + name + data + struct.pack(">I", zlib.crc32(name + data))


def image_bytes():
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(b"\0\0\0\0\0"))
            + chunk(b"IEND", b""))


@unittest.skipUnless(os.name == "posix", "macOS packaging uses POSIX no-follow opens")
class ArtworkInputs(unittest.TestCase):
    def test_regular_image_is_read_and_absent_artwork_is_optional(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary).resolve() / "image.png"
            self.assertIsNone(artwork_bytes(path, "png"))
            path.write_bytes(image_bytes())
            self.assertEqual(artwork_bytes(path, "png"), image_bytes())

    def test_final_links_parent_links_hardlinks_and_fifo_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            private = root / "private.png"
            private.write_bytes(image_bytes())
            linked = root / "linked.png"
            linked.symlink_to(private)
            with self.assertRaises(OSError):
                artwork_bytes(linked, "png")
            directory = root / "alias"
            directory.symlink_to(root, target_is_directory=True)
            with self.assertRaises(OSError):
                artwork_bytes(directory / "private.png", "png")
            hardlink = root / "hardlink.png"
            os.link(private, hardlink)
            with self.assertRaises(ValueError):
                artwork_bytes(hardlink, "png")
            fifo = root / "pipe.png"
            os.mkfifo(fifo)
            with self.assertRaises(ValueError):
                artwork_bytes(fifo, "png")

    def test_non_image_corruption_truncation_trailing_and_oversize_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary).resolve() / "image.png"
            valid = image_bytes()
            for data in [b"private text", valid[:-1], valid + b"private text",
                         valid[:40] + bytes([valid[40] ^ 1]) + valid[41:],
                         b"\0" * (MAX_ARTWORK_BYTES + 1)]:
                with self.subTest(length=len(data)):
                    path.write_bytes(data)
                    with self.assertRaises(ValueError):
                        artwork_bytes(path, "png")

    def test_icns_requires_its_own_magic_and_exact_length(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary).resolve() / "icon.icns"
            path.write_bytes(b"icns\0\0\0\x08")
            self.assertEqual(artwork_bytes(path, "icns"), b"icns\0\0\0\x08")
            path.write_bytes(b"icns\0\0\0\x08extra")
            with self.assertRaises(ValueError):
                artwork_bytes(path, "icns")


if __name__ == "__main__":
    unittest.main()
