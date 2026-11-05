# Independent Thunderbolt transport recovery

Research date: November 5, 2026. This records recovered host-controller structures and independently written offline components. It does not describe an activated replacement driver. The application still uses the original management driver; [the work record](../docs/WORK_RECORD.md) owns acceptance and hardware limits.

## Evidence

REA 6.8.0 with Ghidra 12.1.4 inspected the x86_64 slice of DroboTBT 1.4, SHA-256 `cf21793cafa3d7cded2a07b743cb610415b2b44b88f8a79f15ae8493de8b1662`. Selected directory offsets and queue framing were then corroborated against its arm64e slice, SHA-256 `a0c5bb4e5b7bb853ed92973182fbc390ccc3f45713aedf5c8af6b41fbd8a569e`, using the installed Xcode LLVM disassembler. This does not establish complete ARM semantic equivalence or runtime compatibility. Addresses below refer to x86_64 unless marked ARM; none are disk locations or BAR offsets unless explicitly identified as such.

Local ignored receipts under `artifacts/reports/independence/thunderbolt` contain the source identity, exact queries, evidence IDs, pseudocode, and selected complete assembly. `receipts.json` indexes them. Decompiled output remains local; the public code was written independently from the recovered field behavior.

## BARs and port directory

`InitializeController` at `0x0d8c` maps device memory indices 0 and 1. It uses BAR0 for interrupt/register access and BAR1 for the `DroboPort` directory. The previously recovered host-ready doorbell is BAR0 offset `0x21878`, bit `0x200000`. A disconnected read can produce `0xffffffff`. These observations do not establish DMA ownership, mapping lifetimes, or a safe activation sequence.

`IsDroboPortReady` at `0x5d68` checks a non-null address and compares against the C string `DroboPort` with a maximum length of 16. The terminating NUL is significant; bytes after that NUL are not required to be zero. The initialized firmware port has these little-endian, 32-bit relative offsets:

| Directory offset | Target relative to the BAR1 port base | Corroboration |
| --- | --- | --- |
| `0x18` | LUN table | Base-channel initializer `0x42ce` |
| `0x1c` | 64-byte capabilities block | Base-channel initializer `0x42ce` |
| `0x20`, `0x24` | Storage request and response lists | SCSI initializer `0x4d96` |
| `0x28`, `0x2c` | System-control request and response lists | System initializer `0xba98` |
| `0x30`, `0x34` | Transport-manager request and response lists | TM initializer `0xb0a6` |
| `0x38`, `0x3c` | PCI and endpoint interrupt-definition blocks | SCSI initializer `0x4d96` |
| `0x40` | Inquiry identity data | In-driver initializer `0x9f96` |

The capabilities block starts with ten recovered 32-bit values: interface version, transfer mode, maximum transfer bytes, maximum outstanding transactions, maximum LUNs, flags, maximum burst, stall size, maximum SGL entries, and host interface version. `DumpCapabilities` at `0x473a` supplies the labels. Base initialization writes host interface version 1 at capability offset `0x24`; the offline parser never performs that write. The remaining six words are not interpreted.

Selected ARM corroboration: base initialization at `0xd024` loads the same capability/LUN offsets at `0xd088`/`0xd0a0`; SCSI initialization at `0x10080` loads response/request offsets `0x24`/`0x20` at `0x1012c`/`0x10140`; TM initialization at `0x1a888` loads response/request offsets `0x34`/`0x30` at `0x1a928`/`0x1a93c`. ARM request publication at `0x13c54..0x13c80` uses the same 16-byte descriptors. ARM completion selection at `0x141e4..0x14200` selects 32- versus 8-byte entries and reads queue index/transaction ID. Exact excerpts and the bounded claim list are retained in `arm64e-*.asm` and `arm64e-corroboration.json` beside the REA receipts.

Each LUN record is 16 bytes: a nonzero enabled word, a 32-bit block size, and a 64-bit logical block count. The original initial scan examines 32 slots. All of these are host-visible logical geometry, not protected BeyondRAID capacity. The new decoder uses checked multiplication and explicitly calls the product a virtual byte ceiling.

The bounded parser verifies supported interface versions 1–4, offset alignment and bounds, complete capabilities and fixed LUN-table coverage, and nonzero/nonoverflowing geometry for enabled records. Other directory targets are reported as offsets; their entire contents and consistency are not certified by parsing the directory. A decoded snapshot is not a ready device.

## Requests and completions

`ProcessRequests` at `0x7a06` uses an eight-byte list header, then 16-byte descriptors. Header word 0 is the firmware-provided request window; word 1 is the submitted count. Descriptor words are DMA address low, DMA address high, request length, and transaction ID. The original path also writes a queue index and transaction ID into the associated DMA request buffer before ringing the ready interrupt.

`ProcessCompleteRequests` at `0x7fae` rejects the removed-device count sentinel and counts exceeding the matching request window. It selects completion layout by interface version:

| Entry field | Versions 1–3 | Version 4 |
| --- | --- | --- |
| Entry size | 8 bytes | 32 bytes |
| Queue index | `+0x00`, u32le | `+0x00`, u32le |
| Transaction ID | `+0x04`, u32le | `+0x04`, u32le |
| Realized byte count | Read from a separate request extension | `+0x08`, u64le |
| SRB response word | Read from a separate request extension | `+0x14`, u32le |
| SCSI status word | Read from a separate request extension | `+0x18`, u32le |

Version-4 words at `+0x10` and `+0x1c` remain uninterpreted. The original driver treats SRB values 1 and `0x12` as successful service responses, then passes the separate SCSI status onward. Matching an entry or SRB value alone does not establish successful data I/O. The independent codec checks count/allocation limits, queue and transaction association, duplicate entries, and oversized transfers, while retaining version-specific status for the higher-level command policy. Legacy entries cannot supply a realized count or final status by themselves.

These codecs serialize copied bytes only. They neither allocate/dereference DMA addresses nor submit frames. Their offline resource limits are parser policy, not a claim about a device's queue capacity.

## Management is a separate channel

`GetRequestType` at `0x302c`, together with `ProcessParallelTask` at `0x2dc6`, separates storage reads, system-control operations, in-driver emulation, and transport-manager commands. MODE SENSE(10), including the protected-capacity and LUN-information pages, goes to the transport-manager channel. `READ CAPACITY` is handled separately by the in-driver path at `0xa60c`, using the logical LUN table.

This explains why replacing READ CAPACITY with a smaller protected total would address the wrong layer for an existing filesystem. A replacement storage driver must preserve its logical address space. A filesystem view can later report verified protected allocation through its statistics callback, without changing that geometry.

The TM request builder at `0xb338` uses a `0x58`-byte fixed request/response extension before optional 16-byte SGL entries. Recovered fields include transfer flags at `0x20`, data length at `0x24`, buffer/SGL pointer at `0x28`, and the CDB at `0x30`. Data-in starts with flags `0x410020`; the observed single-segment direct-buffer branch adds flag `0x08`. Response helpers read realized count at `0x40`, SRB response at `0x4c`, and SCSI status at `0x50`. This is additional layout evidence, not a complete DMA request encoder: mapping ownership, address translation, synchronization, and all response semantics still need validation.

## DriverKit path and present implementation

Apple documents [IOUserSCSIParallelInterfaceController](https://developer.apple.com/documentation/scsicontrollerdriverkit/iouserscsiparallelinterfacecontroller) as the base for a PCI-backed SCSI controller. Its required personality uses `IOPCIDevice` as provider. The [PCI transport entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.transport.pci) scopes supported PCI devices, and the [SCSI controller family entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.family.scsicontroller) covers the controller family. Declaring these keys does not grant provisioning or activation approval.

The installed DriverKit 25.2 SDK exposes PCI BAR access, interrupt configuration, I/O buffer descriptors, DMA preparation/completion, and SCSI task/response interfaces for ARM64. A replacement still needs the full controller lifecycle, target management, DMA ownership, ordering, interrupt delivery, timeout/cancellation, reset, sleep/wake, disconnect, and completion implementation.

`platform/macos/driverkit/ReadOnlyTaskAdapter.cpp` projects the actual SDK task type into an independently defined input record. Its pure C policy permits only a bounded initial set of reads and inline read queries. It rejects management writes, firmware/vendor opcodes, power changes, unsupported target/LUN scope, bad directions/counts, and 32-bit DMA-range overflow. READ CAPACITY plans are explicitly marked as virtual geometry. This policy currently supports target 0/LUN 0 and 512-byte block reads only.

`scripts/check_driverkit.py` compiles both the C++ adapter and C policy into ARM64 DriverKit objects against that SDK, then runs the pure command policy with AddressSanitizer and UndefinedBehaviorSanitizer on the host. It produces no driver bundle, registration, activation, MMIO, DMA, or device command. There is no new live transport backend yet.

## Relation to BeyondRAID and Finder

The enclosure firmware already converts its protected array into logical LUN data. A replacement host transport can use that interface without first reimplementing on-device BeyondRAID. Independent member-image recovery is a separate path; [the RMR recovery note](BEYONDRAID_LAYOUT.md) records its newly resolved checksum boundary.

Finder reporting still needs a verified capacity record, device/LUN binding, and a working mounted filesystem view. The current application and FSKit module retain their unconfirmed-unit gate. No hardware, original-driver-free connection, mount, or Finder capacity change has been demonstrated by this offline slice.
