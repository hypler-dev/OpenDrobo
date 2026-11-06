# macOS management ABI and original-service protocol

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

## Local original-service protocol

The original `SkyPCManager::Listen` binds loopback port 4502. Its session header is 16 bytes on the wire, sent from offset two of an 18-byte C++ object. `TMSessionHeader` constructors at `0x10000c908`/`0x10000c938`, Send at `0x10000c974`, and DataLength at `0x10000ca44` establish this layout:

| Wire offset | Meaning |
| --- | --- |
| 0..7 | ASCII `DRIDDSVC` |
| 8 | Opcode |
| 9 | Protocol version 1 |
| 10 | Update event type, or zero for command requests/results |
| 11 | Reserved zero |
| 12..15 | Big-endian payload length |

A subscription sends opcode 1 with no payload. It registers a temporary update observer; no disk or configuration command is sent. Disconnect closes the client socket. Original proxy culling unregisters listeners and destroys owned threads/sockets, but immediate server-side cleanup timing has not been established. Avoid describing subscription as literally no service state change.

Status replies use opcode 7: event 1 or 3 carries `ESATMUpdate`, and event 4 carries `ESATMDevices`. Original rebroadcast sets the header event byte from its update-type argument. Event 2 is a removal notification; the bounded snapshot reader waits for a complete device list and update set and otherwise reports unavailable/stale rather than inferring health. Payloads are UTF-8 XML, normally NUL terminated. OpenDrobo validates header/root correspondence, counts, decimal integer bounds and duplicate critical fields, and rejects entities/DOCTYPE. It publishes a pool once per internal ESA identity, with separate LUN and slot arrays. Raw identifiers, serials and credential fields are not exported.

### Fixed GUI control channel

Opcode 3 carries a NUL-terminated `TMCmd` XML document. The command router reads `CmdID`, `ESAID` and `Params`; opcode 6 returns a `TMCmd` containing the matching `CmdID`, `Result` and optional `ResultDetails`. No arbitrary XML, command ID, path, shell operation or CDB is accepted from the frontend. MCP exposes only control availability.

| Action | CmdID | Fixed parameter policy |
| --- | --- | --- |
| Identify | 26 | `IdentifyInterval` follows original Dashboard: 0 when feature `0x10000000` is set, otherwise 900; interval units remain unknown |
| LED preset | 8 | `Value` from recovered ten-entry map: 1, 2, 5, 7, 9, 11, 15, 25, 40, 59 for UI preset 0..9 |
| Read dim level | 9 | No caller-selected parameter; result is a raw device scale, not a percentage |
| Restart | 21 | `ForceRestart=0` |
| Standby | 22 | No force parameter |
| Shutdown | 23 | `ForceShutdown=0` |

The `TMCmdNames` table at `0x1004cf6e0` establishes command identifiers. `SetDimming` at `0x100066dac` reads `Params/Value`; the original Dashboard's `kDimmingInfo` table and `GeneralSettingsPresenter::GetUpdatedXml` establish preset mapping. The Dashboard binary SHA-256 is `4abe55422533ed9ca2220927cdbd26dd98c9e2f4a1d411224ccc9aed546e5d64`. `ESADevice::processCommand` at `0x100079da4` establishes identify and restart parameters; `ESABlockDevice::processCommand` at `0x10006c2e0` establishes standby/shutdown routes. Restart checks safety, then calls `UnmountAllVolumes`; unmount failure prevents progress. OpenDrobo additionally requires an exact analyzed service hash, a freshly resolved hashed target, the reviewed 5D Thunderbolt path, typed confirmation and no detected mounted Drobo filesystem for power operations.

Mock tests check framing, fixed parameters, target/range rejection, non-forced flags and reply validation. No light-setting or power command has been tested on the home pack. Service acceptance does not prove the hardware transition. Partial send, malformed reply or timeout produces unknown outcome; never retry automatically. Reset, format, firmware, repair/rebuild, shares/users and arbitrary original command routes are not executable.

Ignored exact evidence is under `artifacts/reports/home-capacity/service/` and `dashboard-controls/`; it is not redistributed as vendor source. See the product [management surface](../docs/MANAGEMENT_FEATURES.md), [developer guide](../docs/DEVELOPER_GUIDE.md), and [storage matrix](../docs/STORAGE_SUPPORT.md).
