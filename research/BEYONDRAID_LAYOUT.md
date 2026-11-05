# Recovered RMR layout boundary

Research date: November 5, 2026. This is a bounded static-analysis result for an offline, read-only metadata parser. It is not a BeyondRAID format specification, a member-disk location map, a mount implementation, or an authorization to read or alter a Drobo enclosure or disk.

## Evidence scope

All addresses below are ARM virtual addresses in the production implementation inside the ignored local firmware artifact `artifacts/firmware/5D-runesa.elf`, SHA-256 `f2c79c0cdfcb9695f45d95b94eeed249032d0da28d5ddbd3c45d62a86a06df14`. They are not disk LBAs or byte offsets on any member disk. The firmware executable was inspected offline. No enclosure, member disk/image, driver runtime, or filesystem mount was accessed.

The primary REA 6.8/Ghidra receipt is `artifacts/reports/independence/beyondraid/rmr-layout-receipts.json`. It records successful `procedure_assembly` queries for:

- `rmrCRC` at `0x33f42c`, evidence ID `ev_d2d2733266478809442ef1897bd3e85ef4b036b44e031a508056286d7911de48`;
- `updateRMRHeader` at `0x343b70`, evidence ID `ev_308764cc818618950f32e85786493c3b7114e0902fa08ae96170892e524989e7`;
- `validateRMRHeader` at `0x394904`, evidence ID `ev_a604a6f92388821dfb8723fd85aaa1bf79adba1144220afac95909207e7eb2f6`; and
- `GetCRC` at `0x283c68`, evidence ID `ev_c03829eb3e46fb9e7beb45b19a612824fe1a7da3e5ce715cf80cae1e44c101d0`.

The RMRInfo initialization instruction sequence was independently checked against the same ELF's symbolized ARM disassembly:

```text
BFC<BFCProductionParams>::MetadataManager::InitRMRInfo
0x350448  str r5, [r7, #0x8]
0x35044c  str r5, [r7, #0x4]
```

`r5` is the same non-null pointer yielded from the initialized 8192-byte `DataWindow` path immediately before these stores. This exact alias resolves the prior uncertainty between the RMRInfo header pointer and the CRC-window pointer.

## Recovered facts

### CRC window and checksum

`rmrCRC` at `0x33f42c` loads the header pointer from `RMRInfo + 0x08`, saves four bytes at header offset `0x10`, zeroes those four bytes, calls `GetCRC` with the pointer from `RMRInfo + 0x04` and literal length `0x2000`, then restores the saved bytes.

`InitRMRInfo` writes the same buffer pointer to both of those RMRInfo fields. Therefore, for an RMR buffer constructed through this production path:

```text
CRC window: bytes [0x0000, 0x2000)
checksum field: little-endian u32 at byte 0x0010
CRC calculation input: the full 8192-byte window with bytes 0x10..0x13 set to zero
```

`GetCRC` at `0x283c68` is a reflected byte-at-a-time table recurrence. Its assembly initializes the accumulator to zero and returns it directly; it does not use the usual CRC-32 initial or final `0xffffffff` XOR. `InitCRC` elsewhere in the firmware uses reflected polynomial `0xedb88320`, but a parser should implement the observed `GetCRC` behavior and require image test vectors before treating a match as format compatibility.

### Validated header gates

`validateRMRHeader` at `0x394904` reconstructs little-endian words from the shared buffer and enforces these gates before accepting a header:

| Buffer offset | Required condition | Assembly evidence |
| --- | --- | --- |
| `0x00` | `u32le == 0x726d7268` | `0x394924..0x39496c` |
| `0x04` | `u32le == 0x4246432d` | `0x394938..0x394bcc` |
| `0x08` | `u32le` is 1 or 2 | `0x394bd0..0x394bf4` |
| `0x0c` | `u32le == 0x2000` | `0x394bf8..0x394c18` |
| `0x10` | `u32le == rmrCRC(buffer)` | `0x394c1c..0x394c48` |
| `0x14` | Equals one caller-supplied 32-bit value | `0x394c54..0x394c78` |
| `0x18` | Equals a second caller-supplied 32-bit value | `0x394c7c..0x394ca0` |
| caller value for `0x18` | Its low nine bits must be zero | `0x394ca4..0x394cb0` |
| `0x20` | `u32le` is in 12 through 14 inclusive | `0x394cb4..0x394cdc` |

The byte sequence at offsets `0x00..0x07` is `68 72 6d 72 2d 43 46 42`. It must be treated as raw bytes or the stated little-endian words; this note does not assign a display spelling beyond that evidence.

The caller-provided comparison values at `0x14` and `0x18` have no recovered field names or units. In particular, they must not be labeled an LBA, member-disk position, capacity, generation, or sector count from this evidence alone.

### Revision and disk-pack identity gates

For revision 1, the validator requires bytes `0x88..0x8f` to form an all-zero 64-bit value and requires `u32le(0x1c) == 1`; see `0x394db8..0x394e18`.

For revision 2, it constructs an `IOTracker`, calls `DiskPackManager::getDiskPackId` at `0x394d24`, and requires the recovered eight-byte value to equal bytes `0x88..0x8f`; see `0x394d10..0x394d90` and the final comparison at `0x394f54..0x394f58`. This proves an identity check exists, but does not establish disk-pack ID serialization outside this header or a safe way to discover the expected ID from untrusted image data.

`createRMR` at `0x3989fc` also writes the `getDiskPackId` result into bytes `0x88..0x8f` at `0x398d28..0x398d88`, corroborating that relationship. The meaning of `u32le(0x1c)` is still unknown; it must not be named from the revision-1 comparison alone.

## Safe offline parser boundary

The next parser slice may accept an **explicit caller-supplied candidate buffer only**. It may report a candidate as structurally valid only when all of the following are provided or verified:

1. Exactly 8192 readable bytes are available for the candidate window.
2. The fixed fields and CRC rule above match.
3. The expected values for offsets `0x14` and `0x18` are supplied by a separately evidenced discovery step; they are not inferred from the candidate.
4. Revision-1 rules are satisfied, or an expected disk-pack ID obtained from a separately evidenced source is supplied for revision 2.

The parser must report `insufficient identity evidence`, `unsupported revision`, or `invalid checksum` rather than infer missing values. It must remain read-only and must not run migration, repair, allocation, zeroing, format, or write paths.

## Still unknown

This recovery does not establish any of the following:

- where RMR windows live on individual members, including LBA/byte units and redundancy locations;
- how a candidate generation is selected or reconciled across disks;
- the names, units, and discovery mechanism for values at `0x14` and `0x18`;
- the disk-pack ID discovery source suitable for a standalone image parser;
- array, bitmap, virtual-to-logical map, parity, striping, encryption, or data-reconstruction rules;
- a replacement macOS Thunderbolt/SCSI transport, DriverKit implementation, filesystem, or Finder capacity change.

No member images are present to validate these gates against real metadata. Those missing facts keep BeyondRAID read/parity reconstruction and independent Finder reporting out of scope.
