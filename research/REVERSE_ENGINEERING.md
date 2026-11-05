# Original Drobo system: findings

Research date: November 4, 2026. Status and acceptance are maintained in [the work record](../docs/WORK_RECORD.md).

## Scope and evidence

The user owns a Thunderbolt-connected Drobo on the home Mac and has a separate enclosure and spare disks available for a future lab plan. The exact model and home Mac's macOS version remain unverified. No home Mac connection, disk access, driver installation, flashing, or format experiment has occurred.

The original Dashboard installer was downloaded from the surviving vendor endpoint:

`https://files.drobo.com/webrelease/dashboard/Drobo-Dashboard-3.6.1.dmg`

SHA-256: `65379a749af92e520d59560c5270e4e29888553f14f29626db90ea89f9e87fab`.

The disk image was mounted read-only, and its package was expanded without running installer scripts. The extracted Dashboard application itself was launched to inspect its interface. It reached the All Drobos and Dashboard Preferences screens, with no enclosure present locally. This verifies UI launch, not management or transport compatibility.

REA 6.8.0 performed directory and Mach-O inspection of the original application. REA with Ghidra 12.1.4 then decompiled original driver and storage-firmware functions. Full local receipts are under `artifacts/reports`; raw proprietary artifacts and decompilation outputs are excluded from the public source repository.

## The system has separate layers

```mermaid
flowchart LR
    UI[Dashboard UI] --> S[DDService64d]
    S --> M[Management SCSI layer]
    M --> T[Thunderbolt PCI transport]
    T --> F[Enclosure firmware / runesa.vxe]
    F --> B[BeyondRAID metadata and block mapping]
    B --> D[Member disks]
```

This is a working dependency map derived from package contents, imported interfaces, symbols, and decompilation. It is not an end-to-end runtime trace.

## macOS components

| Component | Observed architecture | Role |
| --- | --- | --- |
| Drobo Dashboard 3.6.1 | x86_64 | Original GUI |
| DDService64d | x86_64 | Background management service |
| TrustedDataSCSIDriver 1.9.1 | x86_64, arm64e | SCSI peripheral management layer |
| DroboTBT 1.4 | x86_64, arm64e | PCI/SCSI Thunderbolt transport |

The legacy Thunderbolt driver already contains Apple silicon code. Native GUI compilation alone does not establish that this driver works on newer macOS.

Its original PCI personalities match vendor `0x4452`, devices `0x3844` and `0x5570`. It subclasses `IOSCSIParallelInterfaceController` and matches `IOPCIDevice`. REA decompilation of `InitializeController` at x86_64 address `0x0d8c` shows PCI mapping, BAR0/BAR1 access, firmware-readiness signaling, channel initialization, and a DMA command pool. This is a transport port, not a UI-only change.

Apple provides [PCIDriverKit](https://developer.apple.com/documentation/pcidriverkit) and [SCSIControllerDriverKit](https://developer.apple.com/documentation/scsicontrollerdriverkit). Their existence identifies candidate APIs; feasibility of replacing this specific Drobo transport is still unproven.

## Firmware containers

Firmware 4.2.3 archives were acquired from the community preservation repository [dhomas1/DroboApps](https://github.com/dhomas1/DroboApps). Their exact URLs and observed hashes are in the local acquisition receipt. No vendor-published firmware digest was verified.

| Model image | Target | Header | ARM payload length | Header/body CRC |
| --- | --- | --- | --- | --- |
| 5D | `5D/Mini` | TDIH v5, 556 bytes | 20,187,920 bytes | Both match |
| 5D3 | `Marvin` | TDIH v5, 556 bytes | 20,775,109 bytes | Both match |

The TDIH header is big-endian. The payloads are little-endian ELF32 ARM executables. The outer images contain VxWorks-related symbols, `.wrs_build_vars`, and DWARF sections. The embedded storage applications have symbol tables but do not have the outer image's DWARF sections.

## Embedded storage application

Each outer firmware payload contains another ELF associated with the `runesa.vxe` strings and runtime path `/bd0/runesa.vxe`.

| Firmware | Embedded offset within outer payload | Extracted ELF length | Named symbols |
| --- | --- | --- | --- |
| 5D | `0x303278` | 11,898,048 bytes | 10,598 |
| 5D3 | `0x302ef8` | 12,488,696 bytes | 10,628 |

Recovered names include `DiskPackManager`, `OnDiskIndex`, `VirtualSpaceMap`, `MetadataManager`, and production/unit-test variants of BFC classes. Names establish navigation targets; they do not establish correct recovered behavior.

Important 5D embedded-image addresses:

| Address | Named function | Research use |
| --- | --- | --- |
| `0x4f5f2c` | `OnDiskIndex::IsSignatureValid` | Signature validation |
| `0x4f617c` | `OnDiskIndex::Configure` | Metadata layout parameters |
| `0x4f85a4` | `OnDiskIndex::GetLocationOfAbub` | Metadata location arithmetic |
| `0x08a3c0` | `DiskPackManager::getPackLayoutVersion` | Pack-version selection |
| `0x340544` | Production `VirtualSpaceMap::translateVtoLcontiguous` | Virtual-to-logical address translation |

REA/Ghidra decompilation of `OnDiskIndex::IsSignatureValid` compares the leading string against **`CS-ODIH`**. This is a concrete index-signature candidate. It has not been verified against member-disk images, and finding that string alone would not validate an array, its version, or its checksums.

The separate `OnDiskIndex::Configure` decompilation corroborates the signature with an eight-byte copy of `CS-ODIH` plus its terminating NUL. It writes little-endian 32-bit values at offsets `0x0c`, `0x10`, and `0x14` from its three integer arguments. It also writes the 32-bit constant `5` at offset `0x08`. Argument meanings, units, the role of that constant, serialization boundaries, and checksum rules remain unresolved. These are decompiler-observed writes, not a verified disk-header specification.

Some Ghidra analysis paths emitted ARM p-code/disassembly warnings. Pseudocode is a tool interpretation and must be checked against assembly, callers, and controlled disk images before it becomes a format specification or implementation.

## Follow-up recovery

The [static recovery notebook](STATIC_RECOVERY.md) records deeper driver and production storage analysis: the host-ready MMIO bit, channel/power-transition paths, packed index fields and metadata-version migration branches, RMR header validation, distinct CRC initializations, and virtual/logical map behavior. Assembly corroborates selected critical values. These remain static observations, with disk placement and runtime behavior explicitly unresolved.

The [capability and community backlog](../docs/CAPABILITIES_AND_BACKLOG.md) separates legacy NAS networking from Thunderbolt host-agent proposals and maps attributed community symptoms to testable improvements. It does not claim an incident has been reproduced or fixed.

## Next research slice

1. Trace `Configure`, the constructor, and read/write callers to identify serialization, byte order, field offsets, versioning, and checksums.
2. Inspect pack-layout/version selection and the metadata-store location functions.
3. Capture labeled, read-only images from a separately approved spare-unit experiment with known test content.
4. Build a candidate metadata parser that rejects unknown layouts and bad checksums.
5. Reconstruct a read-only logical block view and compare known files byte-for-byte.
6. Evaluate single/dual parity, missing members, cache behavior, migration, and interrupted updates before considering writable support.

A native Dashboard, a Thunderbolt transport replacement, a host-side BeyondRAID reader, and replacement enclosure firmware are distinct deliverables. None of the latter three is complete.
