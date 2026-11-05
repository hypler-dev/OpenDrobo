# Read-only macOS management ABI recovery

Research date: November 5, 2026. This describes a userspace bridge to the original management driver. It is not a replacement Thunderbolt PCI/DMA driver or proof of hardware compatibility. Overall acceptance is in [the work record](../docs/WORK_RECORD.md); byte accounting and Finder limits are in [capacity reporting](../docs/CAPACITY_REPORTING.md).

## Original evidence

REA 6.8.0 and Ghidra 12.1.4 inspected the original `TrustedDataSCSIDriver` 1.9.1 and `DDService64d` from Dashboard 3.6.1. The extracted x86_64 driver slice has SHA-256 `a84272a6d55c76548e35512acd3f6d10117321b63ca367652085de86d7f66ae0`; the service hash is `6348d0374c0193c628380426bb7c9a814c41f67bc02c4304c0eaf4116efd5732`. The driver package also contains arm64e code, but the recovered methods below were inspected in x86_64. Matching the request layout in a native ARM build does not establish the original ARM driver's runtime behavior.

The original personality names the service class `com_TrustedData_driver_VendorSpecificType00` and user-client class `com_TrustedData_UserClient_SCSIType00`. `externalMethod` at `0x3740` uses the dispatch table at `0xb950`. The service's `IOCSCSISession::execute` at `0x1000ad55c` and `reopen` at `0x1000ad774` corroborate the selected calls.

| Selector | Recovered operation | OpenDrobo use |
| --- | --- | --- |
| `0` | Open the management connection | Open before the bounded query |
| `1` | Close the management connection | Close every successfully opened query |
| `7` | Vendor-specific data-in pass-through | Fixed read-only MODE SENSE commands only |

`vendorSpecificPassThroughIn` at `0x45b6`, including its assembly, consumes a 32-byte structure and returns a four-byte realized transfer length. It bounds the transfer at `0x8000`, requires a nonzero user address, and uses an I/O memory descriptor for the data buffer.

| Byte offset | Field |
| --- | --- |
| `0` | Page metadata copied from CDB byte 2 |
| `1` | Subpage metadata copied from CDB byte 3 |
| `2..17` | Sixteen-byte CDB storage |
| `18` | CDB length |
| `19` | Reserved byte |
| `20..23` | Native-endian 32-bit transfer length |
| `24..31` | Native-endian 64-bit user-buffer address |

OpenDrobo's C static assertions enforce the recovered size and offsets. There is no exposed write selector or caller-supplied CDB. The legacy selector `2` was deliberately excluded: its wrapper and declared scalar-output count disagree, and its fixed output handles only the basic capacity shape. No exploit or caller behavior is inferred from that mismatch.

## Implemented bounded path

`platform/macos/transport/CapacityWire.c` emits MODE SENSE(10), opcode `0x5a`, DBD `0x08`, page `0x3a`, with a 512-byte allocation. It first reads LUN information, subpage `4`, and requires the count at page byte `4` to equal one. It then reads capacity, subpage `2`. The SPF response page byte is `0x7a`.

The decoder accounts for the eight-byte MODE SENSE header and its declared block-descriptor length. It rejects short responses, bad headers, unsupported body sizes, and inconsistent realized lengths. Capacity pages must contain the complete 32- or 56-byte body. The portable Rust decoder then handles the four big-endian 64-bit protected/unprotected fields and consistency/freshness checks.

`OpenDroboTransport.c` enumerates registry entries without opening a device. An explicit query rechecks the service class and Drobo identity before `IOServiceOpen`. Disk Arbitration links a mounted volume to its actual management service. Mounted volumes are grouped by the known Drobo PCI controller or USB host-device ancestor, so another LUN on the same pack cannot be mistaken for a separate free pool. Both the configured LUN count and mounted-volume scope must be one. Binding is checked again after a capture.

Capacity byte scale remains unverified on hardware. The native reporting adapters therefore withhold the byte-unit confirmation and cannot publish a usable byte total. `--capture-capacity-json <registry-id>` exports only the bounded raw page as hexadecimal with `unitValidation: "unverified"`; it does not bypass that gate. No model profile is currently certified.

## FSKit boundary

The native ARM preview bundles an inactive Swift FSKit extension under `Contents/Extensions/OpenDroboFS.appex`. It uses the Rust C ABI and this same C bridge. Its initial file view forwards ordinary read-only file/directory access through held descriptors to an existing mounted volume. It does not parse HFS+ itself, change block geometry, or alter the statistics of the existing Apple filesystem mount.

Metadata behavior is deliberately limited: writable operations fail with `EROFS`, special files are unsupported, absolute or parent-traversing symlinks are refused, and extended attributes/resource forks are not implemented. A failed identity, unit, freshness, or scope check remains unavailable. A zeroed statistics result is an unavailable result, not a measured zero-capacity device.

The extension follows [Apple's path-resource passthrough example](https://developer.apple.com/documentation/fskit/building-a-passthrough-file-system). Its path resource requires macOS 26. Compilation, direct callback tests, and ad-hoc signature verification do not prove activation, entitlements on another Mac, mounting, or Finder behavior. None has been attempted against an enclosure here.

## Local receipts and next proof

Ignored receipts under `artifacts/reports/transport` include `trusted-mcp-*`, `trusted-details-mcp-*`, `trusted-passthrough-mcp-*`, and `service-transport-mcp-*`. They contain exact queries, executable identities, dispatch details, pseudocode, and selected assembly. Original binaries and raw decompilation are not distributed in the source repository.

The spare-unit test must identify the exact model, firmware, adapter chain, host macOS, filesystem, and LUN layout. First capture raw records read-only and establish byte scale against known protected capacity. A reviewed model profile can then enable byte reporting. Only afterward should a signed/activated read-only view be tested for `statfs`, Foundation, and Finder agreement and byte-for-byte file access. Disconnects, stale data, and mount replacement need separate tests. Writable BeyondRAID, array repair, firmware flashing, and direct replacement Thunderbolt storage transport remain outside this preview.
