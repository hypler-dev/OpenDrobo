# Truthful capacity reporting in macOS and Finder

Date: November 5, 2026. Accepted intent: report the installed pack's real protected usable/used/free capacity in OpenDrobo **and macOS/Finder**, rather than treating the virtual 64 TB volume ceiling as installed storage. This file owns the capacity-reporting slice; the main [work record](WORK_RECORD.md) owns overall status.

Current result: original capacity fields recovered, independent decoder/statistics prototype compiled, and 91 offline checks passed on the local Apple silicon Mac. **Finder reporting is unchanged.** A live capacity transport and complete filesystem module are not implemented by this slice.

## Why reading the OS total does not solve it

The vendor-authored [5D3 guide, preserved PDF](https://content.etilize.com/User-Manual/1040150862.pdf), page 38, explicitly separates virtual volume size from actual storage and gives an example of a 64 TB volume backed by 2 TB of drives. The large address space permits later expansion. Reading that filesystem total would reproduce the misleading number.

These quantities need separate sources:

| Quantity | Source / meaning |
| --- | --- |
| Virtual volume ceiling | Address space exposed by the logical device/filesystem; not installed protected capacity |
| Protected usable total | Enclosure-reported capacity available for protected data |
| Protected used / free | Enclosure's allocation accounting; not automatically identical to per-filesystem file usage |
| Unprotected total | A separate value in the vendor capacity record; do not label it protected usable capacity |
| Filesystem used / available | Mounted filesystem accounting; shared pools, reservations, and reclaimable space can change interpretation |

Adding disk sizes or subtracting the largest drive is an estimate. It is not the source for the requested actual-capacity report.

## Direct recovery from original software

This recovery uses original vendor binaries and firmware, not the removed third-party application dependency. Raw decompilation remains local under `artifacts/reports/capacity`.

`DDService64d` SHA-256: `6348d0374c0193c628380426bb7c9a814c41f67bc02c4304c0eaf4116efd5732`. REA 6.8.0 / Ghidra 12.1.4 inspected its x86_64 `ESAProtocol::ESACapacityInfo::parse` at `0x10009c8e6`, its print method at `0x10009ca34`, and the size getter at `0x10009ca1a`. Selected parse instructions were independently inspected as assembly.

The parser accepts page `0x7a`, subpage `0x02`, and reads big-endian 64-bit values:

| Offset within the page | Vendor print label |
| --- | --- |
| `0x04` | Free Capacity (Protected) |
| `0x0c` | Used Capacity (Protected) |
| `0x14` | Total Capacity (Protected) |
| `0x1c` | Total Capacity (Unprotected) |

The basic page size is `0x24` (36 bytes). The separate PT parser at `0x10009caf4` reads three additional values at `0x24`, `0x2c`, and `0x34`. PT interpretation is unresolved and excluded from the reporting policy.

The 5D firmware's embedded storage executable has SHA-256 `f2c79c0cdfcb9695f45d95b94eeed249032d0da28d5ddbd3c45d62a86a06df14`. Its `HAManager::GetCapacityInfo` at `0x123610` builds page `0x7a/0x02` with a 56-byte declared payload, including the four basic values and the PT extension. It calls `DiskUtilizationDaemon::GetArrayCapacityInfo` at `0xc5ee4` and constructs protected total as protected free plus protected used. A separate `GetHostUsedCapacityInfo` at `0xc5fa0` shows that host-used accounting is another path. These are concrete static observations, not a live capture from the user's unit.

The original basic parser's length guard accepts 28 bytes but then reads through byte 35. The PT parser similarly checks 28 while reading through byte 59. This is a statically observed boundary mismatch; whether callers always provide a larger allocation has not been established. Our experimental decoder requires the entire declared supported payload and rejects truncated or inconsistent input.

Capacity byte units, record-to-device association, freshness, and pool/LUN scope still require controlled hardware confirmation. The prototype reporting adapter requires those confirmations explicitly; it never substitutes filesystem total or the virtual ceiling when they are missing.

Local receipts include `ddservice-mcp-receipts.json`, `capacity-firmware-mcp-receipts.json`, corresponding `capacity-*.c/.asm`, `array-capacity*`, and `host-capacity-response*`. Addresses are executable virtual addresses, not member-disk locations. Some ARM decompiler paths contain incorrect logging strings and incomplete control flow; caller, assembly, and live-record corroboration remain necessary.

## Where Finder must obtain corrected values

[Apple's `statfs` documentation](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/statfs.2.html) describes mounted-filesystem block totals and available space. Apple's [FSStatFSResult](https://developer.apple.com/documentation/fskit/fsstatfsresult) provides total, used, free, and available byte/block properties for an FSKit filesystem. [FSPathURLResource](https://developer.apple.com/documentation/fskit/fspathurlresource) can represent a path in the existing file space on the current macOS target.

The proposed path is a separate filesystem view with verified capacity statistics. A new FSKit module would own its file operations and volume-statistics callback, using the existing mounted volume as a candidate backing resource. This is an architectural proposal. It is not a documented switch for overriding an already-mounted Apple filesystem, and a statistics object alone creates no mount or Finder change.

Changing a block device's reported size is a different operation. Existing partitions and filesystems were created for the original address space. This slice does not alter READ CAPACITY replies, partition tables, filesystem headers, maximum LUN size, or disk contents. The large underlying address space remains intact while a future filesystem view supplies truthful allocation information.

The macOS 26 SDK exposes the relevant FSKit APIs. A working extension still needs complete file-operation behavior, identity binding, lifecycle/error handling, appropriate entitlement/signing, and mount/unmount validation. No extension has been installed, activated, or mounted.

## Experimental code and acceptance boundary

`experiments/capacity` is an isolated, offline prototype. It is not linked into the native app and does not change the user's existing Finder volume. It contains an independently written capacity-page decoder and an FSKit statistics adapter with synthetic tests. It must reject absent/invalid records, unknown byte units, unconfirmed device binding, ambiguous shared-pool scope, and stale/future observations. It must not manufacture a physical total from a 64 TB logical ceiling.

The first adapter deliberately supports only an explicitly confirmed single-volume binding. Multiple LUNs can share a pack; reporting the entire free pool independently as separate guaranteed capacities would be misleading. A later allocation policy must account for this rather than sum per-view totals.

Next gates:

1. Collect read-only capacity records from the identified spare enclosure and establish byte units, pool/LUN identity, and agreement with known disk/layout state.
2. Implement a read-only filesystem view, including file access and FSKit volume statistics. Keep existing volume geometry unchanged.
3. Prove `statfs`, Foundation volume values, and Finder agree on real total/used/free. Compare exposed files byte-for-byte with the backing volume.
4. Exercise disconnects, stale records, remounts, updates after adding capacity, and shared-pool cases. Unknown data must produce an explicit unavailable/error state.
5. Evaluate writable behavior separately after integrity and allocation-budget validation. Accurate display alone is not a write-safety mechanism.

The home Mac and enclosure are unavailable in this workspace. Exact model, home macOS, current filesystem, LUN count, and signing/extension prerequisites are not yet verified. Those are required before claiming Finder compatibility or changing the user's mount workflow.
