from pathlib import Path
from contextlib import redirect_stdout
import importlib.util
import io
import json
import struct
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch
import zlib

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/inspect_firmware.py"
SPEC = importlib.util.spec_from_file_location("inspect_firmware", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def container(body=b"payload"):
    header = bytearray(MODULE.TDIH.pack(556, 5, b"TDIH", 0, b"Test", 0, 0, 0,
                                      len(body), zlib.crc32(body), b"Synthetic", 0))
    header.extend(b"\0" * (556 - len(header)))
    struct.pack_into(">I", header, 308, zlib.crc32(header))
    return bytes(header) + body


class FirmwareContainerTests(unittest.TestCase):
    def test_valid_container(self):
        metadata, body = MODULE.parse_container(container())
        self.assertEqual(body, b"payload")
        self.assertTrue(metadata["header_crc_valid"] and metadata["body_crc_valid"])

    def test_truncated_header(self):
        with self.assertRaises(ValueError): MODULE.parse_container(b"TDIH")

    def test_truncated_payload(self):
        with self.assertRaises(ValueError): MODULE.parse_container(container()[:-1])

    def test_changed_header(self):
        data = bytearray(container()); data[100] ^= 1
        with self.assertRaisesRegex(ValueError, "header CRC"): MODULE.parse_container(data)

    def test_changed_payload(self):
        data = bytearray(container()); data[-1] ^= 1
        with self.assertRaisesRegex(ValueError, "body CRC"): MODULE.parse_container(data)

    def test_non_elf_is_not_inventoried(self):
        _, _, images = MODULE.inspect(container())
        self.assertEqual(images, [])

    def test_truncated_elf(self):
        with self.assertRaises(ValueError): MODULE.elf_inventory(b"\x7fELF\x01\x01\x01")

    @staticmethod
    def elf():
        names = b"\0.shstrtab\0"
        header = MODULE.ELF32.pack(b"\x7fELF\x01\x01\x01" + b"\0" * 9,
                                   2, 40, 1, 0, 0, 52, 0, 52, 0, 0, 40, 2, 1)
        empty = MODULE.SECTION.pack(*([0] * 10))
        strings = MODULE.SECTION.pack(1, 3, 0, 0, 132, len(names), 0, 0, 1, 0)
        return header + empty + strings + names

    def test_minimal_arm_elf(self):
        metadata, symbols = MODULE.elf_inventory(self.elf())
        self.assertEqual(metadata["length"], len(self.elf()))
        self.assertEqual(symbols, [])

    def test_section_table_outside_image(self):
        data = bytearray(self.elf()); struct.pack_into("<I", data, 32, 0xffffffff)
        with self.assertRaises(ValueError): MODULE.elf_inventory(data)

    def test_section_payload_outside_image(self):
        data = bytearray(self.elf()); struct.pack_into("<I", data, 52 + 40 + 16, 0xffffffff)
        with self.assertRaises(ValueError): MODULE.elf_inventory(data)

    def test_invalid_section_name_table(self):
        data = bytearray(self.elf()); struct.pack_into("<H", data, 50, 4)
        with self.assertRaises(ValueError): MODULE.elf_inventory(data)

    @staticmethod
    def symbol_elf():
        names = b"\0.shstrtab\0.symtab\0.strtab\0.text\0"
        strings = b"\0probe_function\0"
        names_offset = 52 + 32 + 5 * 40
        symbols_offset = names_offset + len(names)
        strings_offset = symbols_offset + 32
        text_offset = strings_offset + len(strings)
        header = MODULE.ELF32.pack(b"\x7fELF\x01\x01\x01" + b"\0" * 9,
                                   2, 40, 1, 0x1000, 52, 84, 0, 52, 32, 1, 40, 5, 1)
        program = struct.pack("<IIIIIIII", 1, text_offset, 0x1000, 0x1000, 4, 4, 5, 4)
        sections = [MODULE.SECTION.pack(*([0] * 10))]
        sections.append(MODULE.SECTION.pack(1, 3, 0, 0, names_offset, len(names), 0, 0, 1, 0))
        sections.append(MODULE.SECTION.pack(names.index(b".symtab"), 2, 0, 0, symbols_offset, 32, 3, 1, 4, 16))
        sections.append(MODULE.SECTION.pack(names.index(b".strtab"), 3, 0, 0, strings_offset, len(strings), 0, 0, 1, 0))
        sections.append(MODULE.SECTION.pack(names.index(b".text"), 1, 6, 0x1000, text_offset, 4, 0, 0, 4, 0))
        symbols = MODULE.SYMBOL.pack(0, 0, 0, 0, 0, 0) + MODULE.SYMBOL.pack(1, 0x1000, 4, 0x12, 0, 4)
        return header + program + b"".join(sections) + names + symbols + strings + b"\x1e\xff\x2f\xe1"

    def test_program_and_symbol_inventory(self):
        image = self.symbol_elf()
        metadata, symbols = MODULE.elf_inventory(image)
        self.assertEqual(metadata["length"], len(image))
        self.assertEqual(symbols, [{"name": "probe_function", "address": "0x1000", "size": 4, "type": 2}])

    def test_program_payload_outside_image(self):
        data = bytearray(self.symbol_elf()); struct.pack_into("<I", data, 56, 0xffffffff)
        with self.assertRaises(ValueError): MODULE.elf_inventory(data)

    def test_invalid_symbol_name(self):
        data = bytearray(self.symbol_elf())
        symbol_section = MODULE.SECTION.unpack_from(data, 84 + 2 * 40)
        struct.pack_into("<I", data, symbol_section[4] + 16, 0xffffffff)
        with self.assertRaisesRegex(ValueError, "symbol name"): MODULE.elf_inventory(data)

    def test_cli_extraction_preserves_exact_bytes(self):
        with TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "test.tdf"
            source.write_bytes(container(self.symbol_elf()))
            output = root / "output"
            with patch("sys.argv", [str(SCRIPT), str(source), "--output-dir", str(output)]):
                with redirect_stdout(io.StringIO()): MODULE.main()
            self.assertEqual((output / "image-00000000.elf").read_bytes(), self.symbol_elf())
            symbols = json.loads((output / "image-00000000.symbols.json").read_text())
            self.assertEqual(symbols[0]["name"], "probe_function")


if __name__ == "__main__":
    unittest.main()
