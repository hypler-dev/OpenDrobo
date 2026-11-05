# Thunderbolt and storage metadata recovery

Research date: November 4, 2026. This is a static-analysis notebook, not a disk-format specification or a working replacement driver. See [the work record](../docs/WORK_RECORD.md) for acceptance and [acquisition findings](REVERSE_ENGINEERING.md) for artifact provenance.

## Evidence and confidence

REA 6.8.0 with Ghidra 12.1.4 inspected the original x86_64 DroboTBT driver and the embedded ARM storage executable from 5D firmware 4.2.3. Firmware addresses below belong only to `5D-runesa.elf`, SHA-256 `f2c79c0cdfcb9695f45d95b94eeed249032d0da28d5ddbd3c45d62a86a06df14`. They are virtual addresses, not offsets on a member disk. The 5D3 image is inventoried but these functions have not been cross-validated there.

- **Assembly corroborated:** a specific instruction sequence or literal supports the claim.
- **Decompiler observed:** pseudocode supports the claim; recovered types, calling conventions, and logging strings can be wrong.
- **Unresolved:** disk placement, units, lifecycle, or runtime behavior still needs evidence.

Original binaries and raw decompilation stay in ignored local artifacts. Public notes contain observations and independently described formulas. Nothing here was executed against an enclosure or disk.

## Thunderbolt handshake and channels

| Function / x86_64 address | Observation | Confidence |
| --- | --- | --- |
| `TriggerHostBarInterrupt`, `0x12fc` | Sets bit `0x200000` in MMIO at mapped BAR base plus `0x21878`; polls for the bit to clear, with 500 ms sleep calls | Assembly corroborated for offset, bit, and sleep argument |
| `InitializeDroboChannels`, `0x140e` | Checks readiness and a `DroboPort` signature; creates in-driver SCSI, task-management, system-control, and normal SCSI channels; failure cleanup clears readiness | Decompiler observed |
| `IsDroboPortReady`, `0x5d68` | Requires a non-null pointer and a bounded string comparison with `DroboPort`; this is not merely a prefix match | Decompiler observed |
| `HostSleepEvent`, `0x67f6` | Coordinates outstanding I/O before sleep; contains a 30,000 comparison constant and 1,000 ms sleep calls | Decompiler observed; clock prototype and timeout semantics unresolved |
| `HostWokeUp`, `0x6968` | Has version/flag-dependent initialization handling, port/signature and serial checks, and a bounded poll path containing 200 ms sleeps | Decompiler observed; precise timing and applicable versions unresolved |
| `ProcessCompletionQueue`, `0x894e` | Drains completions, reduces the outstanding-I/O count, and can acknowledge a pending power-state transition when the count reaches zero | Decompiler observed |
| `BuildDroboRequest`, `0x6d9a` | Contains DMA/scatter-gather construction and SCSI-opcode-specific branches | Decompiler observed; packet layout not yet specified |

These are separate readiness stages. A future diagnostic view should distinguish PCI enumeration, BAR mapping, handshake completion, channel creation, SCSI storage availability, and management-record availability. That design is an inference from the layer boundaries; it does not explain any particular community incident yet.

The original driver already includes arm64e code, while the extracted `DDService64d` is x86_64. Neither fact proves compatibility with current macOS. OpenSSL-related symbols occur in the service; library presence does not establish which management connections use TLS or how peers authenticate.

## OnDiskIndex: structure and version handling

`OnDiskIndex::Configure` at `0x4f617c` copies eight bytes `CS-ODIH\0` and writes little-endian fields. Its explicit byte stores and the constructor/getter assembly corroborate byte order. This is the observed object prefix; it is not yet a complete serialized header.

The pointer literal at `0x4f6304` resolves to `0x693af8`, whose eight file-backed bytes are `43532d4f44494800`. The copy call at `0x4f61a0` targets `0x5f5920`, named `memcpy` in the ELF symbol table. Local `index-signature-literal.json` records the source hash, PT_LOAD address mapping, exact bytes, and callee identity; this confirms the firmware literal, not its location on a member disk.

| Offset relative to index object | Observed use |
| --- | --- |
| `0x00` | Eight-byte signature including NUL |
| `0x08` | Configure writes `5`; the load path dispatches on this field as a metadata version |
| `0x0c`, `0x10`, `0x14` | Three Configure arguments; getter at `0x4f5314` calls the last one the main-data zone number |
| `0x1c`, `0x20` | Configure writes `5`, `64`; used by ASB location arithmetic |
| `0x25`, `0x29` | Configure writes `0x45`, `64`; used by DST location arithmetic |
| `0x31` | Configure writes `0x85`; used by ABUB-store location arithmetic |
| `0x35` | Configure writes its span argument minus `0x85` |
| `0x39` | Used as a record count by `FindAbubRecord` |
| `0x200` | Start of 16-byte records examined by `FindAbubRecord`; constructor fills `0x200..0x3ff` with `0xff` |

Let `U(o)` mean a little-endian 32-bit load at object offset `o`. Observed getter formulas are:

```text
ASB(i)      = U(0x0c) + U(0x1c) + i * (U(0x20) >> 5)
DST(i)      = U(0x0c) + U(0x25) + i * (U(0x29) >> 5)
ABUB store  = U(0x0c) + U(0x31)
```

Getter addresses are `0x4f5e5c`, `0x4f5ec4`, and `0x4f54ac`. Names and formulas do not establish sector/zone units, bounds, or safe physical locations. `GetLocationOfAbub` at `0x4f85a4` has misleading pseudocode caused by an output-parameter inference failure; its apparent all-ones result must not be turned into a format rule. Assembly shows values returned through a record output from `FindAbubRecord` at `0x4f83f4`.

`OnDiskStore::LoadDiskPack` at `0x503dd8` ties the object to metadata loading: a `ReadThroughCache` call targets the index object, then `IsSignatureValid` checks it. Assembly shows the Configure call at `0x503ee0`, index-version comparisons at `0x504260..0x504288`, and migration calls. Version `2` has a named V2-to-V3 path; versions `3`, `4`, and `5` reach `MigrateToV5Meta`; an unsupported version takes error `0x306`.

Crucially, this load path can also call `Zone0InfoTable::CreateEntry` and `Configure` after failed identity checks. A read-only recovery tool must not reproduce the legacy initialization/migration workflow. It must report invalid or unsupported input without repairing it. The pseudocode contains incomplete paths and incorrect logging strings, so migration behavior still needs caller/callee analysis.

## Production RMR validation and CRC

The production `MetadataManager::validateRMRHeader` function at `0x394904` checks the following values relative to its header pointer:

| Offset | Observed check |
| --- | --- |
| `0x00`, `0x04` | 32-bit words `0x726d7268`, `0x4246432d` |
| `0x08` | Revision in `1..2` |
| `0x0c` | Size equals `0x2000` (8192) |
| `0x10` | Stored CRC compared with `rmrCRC` |
| `0x14`, `0x18` | Compared with caller-provided identity/geometry values |
| `0x20` | Shift field in `12..14` |
| `0x88`, `0x8c` | Zero for revision 1; compared with current disk-pack identity for revision 2 |

It also checks a caller argument for 512-byte alignment and performs deeper array/geometry validation. Observed error paths include `0xB09` and `0xB0A`; assigning one human diagnosis to every occurrence would be premature. The two initial words would spell `hrmr-CFB` if read as raw little-endian bytes at that pointer. Disk images must establish whether those are the actual serialized bytes; do not search for an assumed display spelling and call the result a valid header.

`rmrCRC` at `0x33f42c`, corroborated by assembly, saves and zeroes four bytes at **header pointer + `0x10`**, calls `GetCRC` through the buffer pointer with length `0x2000`, then restores the saved bytes. Those pointers come from RMRInfo offsets `+0x08` and `+0x04`. Follow-up assembly of `InitRMRInfo` now proves that this production initialization path stores the same DataWindow buffer address in both fields. For this recovered shape, the checksum window starts at header byte zero and its checksum field is at byte `0x10`. [The RMR layout note](BEYONDRAID_LAYOUT.md) records the exact stores and remaining identity/location limits; it does not establish member-disk placement.

The CRC helpers are not interchangeable:

| Helper / address | Initial value | Final XOR | Polynomial / processing |
| --- | --- | --- | --- |
| `GetCRC`, `0x283c68` | `0` | None | Reflected byte-at-a-time table recurrence |
| `getCRC32`, `0x283d4c` | `0xffffffff` | `0xffffffff` | Same reflected table recurrence |
| `InitCRC`, `0x283be0` | Table generator | — | Reflected polynomial `0xedb88320`, eight shifts per table entry |

`GetCRC` initialization and return are assembly corroborated. `InitCRC` assembly and a complete 16-byte literal read at `0x283c60` corroborate the table address and polynomial. A direct read of the runtime table at `0x7c1784` returned **zero bytes, incomplete**; it was not a table dump. An uninitialized runtime table is consistent with this result; the actual initialization sequence has not been traced.

Using ordinary CRC-32 defaults for RMR would use the wrong seed and final XOR. A future parser needs an independently implemented helper plus known-image test vectors, rather than assuming a library default validates this metadata.

## Virtual and logical block maps

Production `VirtualSpaceMap::translateVtoLcontiguous` at `0x340544` derives a chunk index from a geometry shift, looks up a forward mapping, and clamps the contiguous length to a boundary. Pseudocode shows 20-byte entries with validity information, a 64-bit mapped base, and a logical-disk index. Missing or out-of-range mappings return `0xB08`.

The inverse at `0x341a54` uses logical-disk-grouped 12-byte entries and similar validity/boundary handling. These are in-memory map observations. Their disk serialization, generation selection, redundancy, parity algorithm, and recovery rules remain unresolved. Missing mappings must not silently become zero-filled successful reads.

The executable contains both `BFCProductionParams` and `BFCUnitTestParams` implementations. The addresses above are production instances. Name similarity does not make unit-test templates evidence of shipped production behavior.

## Reproduction receipts and next boundary

Local ignored receipts under `artifacts/reports/round2` include:

- `driver-queries.json` and `tb-*.c/.json`: original driver queries; `artifacts/reports/thunderbolt-x86_64.asm` independently corroborates the MMIO sequence.
- `ddservice-architecture.json`: exact extracted service path, source hash, and `file`/`lipo` architecture inspection.
- `initial-firmware-queries.json`: index constructor/location queries.
- `index-signature-literal.json`: immutable file-backed signature/pointer bytes and copy-callee symbol identity.
- `firmware-mcp-receipts.json`: production validator, mapping, and index assembly queries.
- `followup-mcp-receipts.json`: GetCRC, InitCRC, literal bytes, load path, and output-parameter follow-up.

One persistent MCP session reused Ghidra analysis for successive queries. The installed REA 6.8.0 runtime imported an ephemeral project even when newer source documentation suggested saved-project environment overrides; process receipts, not those newer docs, determined actual behavior.

The [transport follow-up](THUNDERBOLT_TRANSPORT.md) now records the BAR directory, logical LUN geometry, separate management channel, request/completion layouts, and initial non-loading DriverKit task adapter. The portable core also has offline port/frame codecs and an RMR header-candidate inspector. No member-disk images are available in this workspace. Next work is to establish metadata placement and generation selection, validate these shapes against real captures, and reconstruct known virtual ranges byte-for-byte. Parity reconstruction, a writable BeyondRAID engine, replacement firmware, and a complete modern Thunderbolt driver remain unimplemented.
