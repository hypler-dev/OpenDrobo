#!/usr/bin/env python3
"""Inspect TDIH firmware containers and embedded ELF files without execution."""
from pathlib import Path
import argparse
import hashlib
import json
import struct
import zlib

TDIH = struct.Struct(">II4sI16sIIIII256sI")
ELF32 = struct.Struct("<16sHHIIIIIHHHHHH")
SECTION = struct.Struct("<IIIIIIIIII")
SYMBOL = struct.Struct("<IIIBBH")


def parse_container(data):
    if len(data) < TDIH.size:
        raise ValueError("Truncated TDIH header")
    fields = TDIH.unpack_from(data)
    header_size, version, magic = fields[:3]
    if magic != b"TDIH" or version != 5:
        raise ValueError("Unrecognized firmware container")
    if not TDIH.size <= header_size <= len(data) or header_size + fields[8] != len(data):
        raise ValueError("Firmware length mismatch")
    header = bytearray(data[:header_size])
    header[308:312] = b"\0" * 4
    if zlib.crc32(header) & 0xffffffff != fields[11]:
        raise ValueError("Firmware header CRC mismatch")
    body = data[header_size:]
    if zlib.crc32(body) & 0xffffffff != fields[9]:
        raise ValueError("Firmware body CRC mismatch")
    return {
        "header_length": header_size, "header_version": version,
        "target": fields[4].split(b"\0", 1)[0].decode("ascii", "replace"),
        "payload_length": len(body), "header_crc_valid": True, "body_crc_valid": True,
        "sha256": hashlib.sha256(data).hexdigest(),
        "authenticity": "CRC consistency does not establish vendor authenticity",
    }, body


def elf_inventory(data):
    def bounded(offset, size):
        if offset < 0 or size < 0 or offset + size > len(data):
            raise ValueError("ELF range exceeds available bytes")
        return data[offset:offset + size]

    if len(data) < ELF32.size or data[:7] != b"\x7fELF\x01\x01\x01":
        raise ValueError("Expected a little-endian ELF32 image")
    header = ELF32.unpack_from(data)
    if header[2] != 40 or header[11] != SECTION.size or not 0 < header[12] <= 4096:
        raise ValueError("Unsupported ELF architecture or section table")
    if header[13] >= header[12]:
        raise ValueError("Invalid ELF section-name table")
    table = bounded(header[6], header[11] * header[12])
    sections = [SECTION.unpack_from(table, index * SECTION.size) for index in range(header[12])]
    names_section = sections[header[13]]
    names = bounded(names_section[4], names_section[5])
    extent = max(ELF32.size, header[6] + len(table))
    if header[10]:
        if header[9] != 32 or header[10] > 4096:
            raise ValueError("Invalid ELF program table")
        programs = bounded(header[5], header[9] * header[10])
        extent = max(extent, header[5] + len(programs))
        for index in range(header[10]):
            program = struct.unpack_from("<IIIIIIII", programs, index * 32)
            bounded(program[1], program[4])
            extent = max(extent, program[1] + program[4])
    inventory, symbols = [], []
    for section in sections:
        if section[0] >= len(names):
            raise ValueError("Invalid ELF section name")
        name = names[section[0]:].split(b"\0", 1)[0].decode("ascii", "replace")
        inventory.append({"name": name, "type": section[1], "size": section[5]})
        if section[1] != 8:
            bounded(section[4], section[5])
            extent = max(extent, section[4] + section[5])
        if section[1] != 2:
            continue
        if section[9] != SYMBOL.size or section[5] % SYMBOL.size or section[6] >= len(sections):
            raise ValueError("Invalid ELF symbol table")
        strings_section = sections[section[6]]
        strings = bounded(strings_section[4], strings_section[5])
        table = bounded(section[4], section[5])
        for offset in range(0, len(table), SYMBOL.size):
            entry = SYMBOL.unpack_from(table, offset)
            if entry[0] >= len(strings):
                raise ValueError("Invalid ELF symbol name")
            name = strings[entry[0]:].split(b"\0", 1)[0].decode("utf8", "replace")
            if name:
                symbols.append({"name": name, "address": hex(entry[1]),
                                "size": entry[2], "type": entry[3] & 15})
    return {"entry": hex(header[4]), "machine": "ARM", "length": extent,
            "sections": inventory, "named_symbols": len(symbols)}, symbols


def inspect(data):
    container, body = parse_container(data)
    images = []
    offset = 0
    while True:
        offset = body.find(b"\x7fELF", offset)
        if offset < 0:
            break
        try:
            metadata, symbols = elf_inventory(body[offset:])
        except ValueError:
            offset += 4
            continue
        images.append((offset, metadata, symbols))
        offset += 4
    return container, body, images


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("firmware", type=Path)
    parser.add_argument("--output-dir", type=Path, help="New local directory for extracted images and symbols")
    args = parser.parse_args()
    if not args.firmware.is_file():
        parser.error("Input must be a regular firmware file")
    with args.firmware.open("rb") as stream:
        data = stream.read(67_108_865)
    if len(data) > 67_108_864:
        parser.error("Firmware exceeds the 64 MiB inspection limit")
    container, body, images = inspect(data)
    report = {"container": container, "images": [dict(offset=offset, **metadata)
                                                  for offset, metadata, _ in images]}
    if args.output_dir:
        args.output_dir.mkdir(parents=True, exist_ok=False)
        for offset, metadata, symbols in images:
            prefix = args.output_dir / f"image-{offset:08x}"
            prefix.with_suffix(".elf").write_bytes(body[offset:offset + metadata["length"]])
            prefix.with_suffix(".symbols.json").write_text(json.dumps(symbols, indent=2) + "\n")
        (args.output_dir / "inventory.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
