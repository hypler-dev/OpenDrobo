# Capacity reporting in macOS and Finder

Date: November 5, 2026. Goal: report protected usable, used, and free capacity instead of repeating Drobo's virtual 64 TB ceiling.

## Current result

Original service and firmware records are recovered. Portable Rust decoder, native read-only transport bridge, and macOS FSKit read-only view are implemented and directly checked. **Physical byte units are not certified yet.** Live adapter therefore returns capacity unavailable, and FSKit statistics stay unavailable rather than displaying guessed values. Finder reporting is unchanged because extension has not been installed, activated, mounted, or tested with Drobo.

Local Mac has no attached enclosure. Exact model, home macOS version, filesystem, LUN count, and adapter chain remain unverified. Spare hardware is reserved for separate read-only lab plan.

## Recovered vendor record

`DDService64d` parses page `0x7a`, subpage `0x02`, with big-endian values at offsets `0x04` protected free, `0x0c` protected used, `0x14` protected total, and `0x1c` unprotected total. Firmware constructs protected total as protected free plus protected used. PT extension fields remain uninterpreted.

Recovered original user client is `com_TrustedData_UserClient_SCSIType00`. Bounded bridge uses selector 0 to open, selector 7 for MODE SENSE(10), and selector 1 to close. It reads LUN-info page `4` first and requires one LUN, then reads capacity page `2`. It accepts no caller-supplied commands and exposes no write selector.

## Static recovery evidence

Transport ABI details are tracked in [MACOS_MANAGEMENT_ABI.md](../research/MACOS_MANAGEMENT_ABI.md).

Original `DDService64d` SHA-256: `6348d0374c0193c628380426bb7c9a814c41f67bc02c4304c0eaf4116efd5732`. REA/Ghidra recovered `ESAProtocol::ESACapacityInfo::parse` at `0x10009c8e6`, print method at `0x10009ca34`, and size getter at `0x10009ca1a`. Parser accepts page `0x7a`, subpage `0x02`, then reads four big-endian 64-bit values:

| Page offset | Meaning |
| --- | --- |
| `0x04` | Protected free capacity |
| `0x0c` | Protected used capacity |
| `0x14` | Protected total capacity |
| `0x1c` | Unprotected total capacity |

Basic record is 36 bytes (`0x24`). PT parser at `0x10009caf4` reads additional values at `0x24`, `0x2c`, and `0x34`; fields remain uninterpreted. 5D firmware image SHA-256: `f2c79c0cdfcb9695f45d95b94eeed249032d0da28d5ddbd3c45d62a86a06df14`; `HAManager::GetCapacityInfo` at `0x123610` constructs page `0x7a/0x02` with 56-byte payload and obtains array values through `GetArrayCapacityInfo` at `0xc5ee4`. Separate host-used path exists at `0xc5fa0`.

Static boundary mismatch: original basic parser checks 28 bytes but reads through byte 35; PT parser similarly checks 28 but reads through byte 59. Caller allocation behavior is not established. OpenDrobo decoder requires complete declared 32- or 56-byte bodies and rejects truncation. These are static observations, not live captures or proof of byte units.

5D image is ARM32 with 10,598 named symbols; 5D3 inventory has 10,628. Outer TDIH lengths and CRCs match recorded containers, but vendor authenticity is not established. Raw decompilation and receipts remain ignored under `artifacts/reports/`.

## Finder boundary

Existing HFS+ mount reports logical address space, not proof of protected pool capacity. FSKit implementation is separate read-only view whose statistics can later use verified protected values. It does not transparently replace Apple's existing HFS+ provider, alter READ CAPACITY, edit partition metadata, or change disk contents.

Module currently supports ordinary read-only files and directories, path confinement, inode-stable held descriptors, and conservative symlink handling. Resource forks, extended attributes, writable operations, and broader metadata fidelity remain open work.

## Evidence gates

Before any Finder claim, spare enclosure test must establish:

1. Byte units from device records against known pack geometry.
2. Registry identity, physical pack identity, and single-LUN scope.
3. Agreement between decoded protected values and repeated read-only captures.
4. Mount, `statfs`, Foundation statistics, and Finder display on same read-only view.
5. Disconnect, remount, stale-record, and shared-pool behavior.

Raw capture uses `--capture-capacity-json <registry-id>` for lab work. It is hexadecimal evidence only and labels unit validation unverified. No destructive disk operation is authorized by this document.

Rust core has 16 native ARM unit tests covering page/body bounds, scope flags, overlap-safe C ABI decoding, count validation, and record expiry. Historical `experiments/capacity` prototype retained 91 offline checks, including unit/scope/expiry rejection; it is not live transport path.

## Driver-independence follow-up

The [transport recovery](../research/THUNDERBOLT_TRANSPORT.md) now traces the
separate storage, transport-manager, system-control, and in-driver paths. The
original READ CAPACITY implementation derives geometry from the logical LUN
table. The new port decoder therefore labels its byte totals as virtual
ceilings; it never uses them to satisfy the protected-capacity confirmation.

An ARM64 DriverKit task adapter now compiles against the installed SDK. Its
read-only command policy distinguishes those geometry queries from the two
vendor MODE SENSE pages. It performs no MMIO or DMA and is not a live backend.
The application and FSKit provider still use the existing management bridge
and retain the unconfirmed-unit gate.

The [RMR follow-up](../research/BEYONDRAID_LAYOUT.md) resolves the metadata
checksum window and supports offline header-candidate inspection. It does not
recover pool usage or supply filesystem capacity. Member-image reconstruction
and enclosure transport are distinct paths; neither offline result changes
Finder reporting.
