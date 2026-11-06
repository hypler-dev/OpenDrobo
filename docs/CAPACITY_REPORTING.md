# Capacity reporting in macOS and Finder

Date: November 5, 2026. Goal: report protected usable, used, and free capacity instead of repeating Drobo's virtual 64 TB ceiling.

## Current result

OpenDrobo can now read protected-pool total, used and free capacity from the original local Dashboard service. A read-only home-Mac observation confirmed a Drobo 5D over Thunderbolt with one protected pool, two HFS+ logical volumes, five hard drives and an SSD/cache slot. The service's explicit capacity fields are displayed as **service-reported bytes**. OS filesystem usage remains a separate measurement. Exact host readings and raw service frames stay in ignored private receipts.

This does not certify a raw-wire model profile. The direct user-client decoder still passes `byteUnitsConfirmed: false`, and its single-volume gate remains closed for a multi-LUN pack. A separate pool diagnostic accepts bounded LUN evidence without assigning the pool total to an individual filesystem. Both direct opens on the home Mac returned `kIOReturnExclusiveAccess` while the original service owned the connection; OpenDrobo did not stop that service or force the connection.

Finder reporting remains unchanged. The current separate-view FSKit research module requires macOS 26, while the home Mac runs Intel macOS 14.7.7. The module has not been activated or mounted. It does not override Apple's HFS+ provider or alter the existing 64 TiB logical geometry. A compatible filesystem presentation, per-volume accounting policy and named spare-unit validation remain required.

## Three accounting layers

| Layer | Meaning | Current presentation |
| --- | --- | --- |
| Protected pool | Device/service accounting shared by all LUNs in the disk pack | Pool total, allocated and free bytes; source and observation age shown |
| OS filesystem | Used/free space in each mounted HFS+ or other host filesystem | Native cached filesystem statistics in Volumes; logical ceilings remain labeled |
| Physical slots | Raw capacity of each installed HDD or SSD | Per-slot capacity and SSD life; not added together as protected usable capacity |

Service protected usage and OS filesystem usage need not match. Metadata, protection and allocation accounting belong to different layers. Never add the pool total once per LUN, subtract one LUN's OS usage from a shared pool and call the result that LUN's free space, or present the SSD's raw capacity as writable user storage. Cache-used/free values remain unknown because the recovered record does not establish them.

Capacity → Usage shows protected allocation, protected free space and usable total together. OS filesystem usage remains a separate reading; raw installed drive capacity is secondary hardware information. Physical protection overhead and cache occupancy remain unavailable. Mounted volumes group only by native enclosure ancestry and show their own OS-used bytes. Thin-provisioned logical totals and free space stay in labeled details and never replace protected free capacity.

## Original-service observer

`desktop/src-tauri/src/service.rs` uses a fixed read-only subscription to `127.0.0.1:4502`. It validates frame identity, version, event type, reserved byte, declared lengths, strict XML and complete advertised device membership. Bounds are 64 frames, 512 KiB per XML record, 2 MiB per subscription and a 35-second absolute deadline. Internal ESA identifiers correlate LUNs to a shared pool; UI/MCP identifiers are SHA-256 pseudonyms. These identifiers are not authentication credentials.

The GUI pauses five seconds between bounded service reads and reuses successful observations for at most five seconds in the same process. A full original-service update can require up to 35 seconds; the displayed observation time is preserved. A fresh observation means a recently received service cache, not an independently timed hardware measurement. Read failures retain old data as stale. Confirmed service removal and unavailability remain different conditions. Device events record changes seen by this process; they do not recover the device's historical onboard event log.

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
2. Registry identity, physical pack identity, and a validated single-volume or shared-pool accounting policy.
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

An ARM64 DriverKit task adapter compiles against the installed SDK. Its
read-only command policy distinguishes those geometry queries from the two
vendor MODE SENSE pages. It performs no MMIO or DMA and is not a live backend.
The guarded raw-capacity and FSKit provider paths retain the existing management bridge and unconfirmed-unit gate. The separate service observer supplies UI/MCP pool values without enabling that gate.

The follow-up includes separate version-4 storage/management queues, DMA lease
ownership, firmware-defined interrupts, a real SDK/IIG SCSI service, target/task
callbacks, and a locally linked ARM64 research `.dext`. The service can encode
READ(10/16) and the allowlisted capacity/LUN-info requests against mocks. It also
constructs bounded inline READ CAPACITY and REPORT LUNS responses from validated
logical geometry. Those responses preserve the original virtual address space;
they do not substitute protected pool capacity or shrink an existing HFS+ volume.

The service is disabled in source before PCI open and is not wired into the
application bridge. Original 5D bridge firmware distinguishes the direct-buffer
field at SRB `+0x38` from the inline SGL at `+0x80`; the selected read flags use
the latter. Graceful stop now rejects new work while polling matched completions,
then closes sources only after owned actions and mapping leases finish. A fault
closes sources conservatively; unfinished mappings/actions remain pinned.
This is per-request ownership accounting, not a global PCI/DMA quiescence proof.
Other firmware paths, complete SCSI/power behavior, provisioning, activation, and
hardware validation remain open. Neither a linked bundle nor successful mock completion certifies byte
units, measures OS file usage, or changes Finder statistics. The FSKit mount and
unit/scope evidence gates above remain required.

The [RMR follow-up](../research/BEYONDRAID_LAYOUT.md) resolves the metadata
checksum window and supports offline header-candidate inspection. It does not
recover pool usage or supply filesystem capacity. Member-image reconstruction
and enclosure transport are distinct paths; neither offline result changes
Finder reporting.
