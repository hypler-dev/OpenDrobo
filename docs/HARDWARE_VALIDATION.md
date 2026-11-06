# Hardware validation handoff

The user has a separate spare Drobo 5D and disposable drives. That is sufficient for the first 5D test sequence; additional models are not required before starting. It does not certify every 5D board/firmware revision or other Drobo models. The existing home production pack remains excluded.

## What the operator must provide

- The spare 5D, its correct power supply and a separately identified disposable disk pack. Record model, firmware, member identities and any cache SSD locally. Keep the existing home pack and its member disks out of the experiment.
- An Apple-silicon Mac with its exact chip/model and macOS version, physical console access and a recovery path. A compiled ARM64 app is not device-connectivity evidence. No specific Apple-silicon test Mac has yet been confirmed for this plan.
- A known-working Thunderbolt cable and the complete adapter chain. Where required, Apple's [Thunderbolt 3-to-2 adapter](https://support.apple.com/en-us/111753) supports Thunderbolt/Thunderbolt 2 connections; a similar-looking Mini DisplayPort cable is not equivalent. Record what is actually connected. Use one direct connection first.
- A compatible USB data cable for a separate baseline if available. USB and Thunderbolt observations are separate cases; success on one does not certify the other.
- The original software version and a baseline captured while the spare is healthy, plus an explicit list of tests approved for the disposable pack. Do not send serials or disk contents publicly; use the private [lab manifest](SPARE_UNIT_LAB.md).

## Ordered acceptance

1. **Identity and isolation:** verify spare and protected-unit/member exclusions, Mac/OS, firmware, cable chain and pending lab manifest. This step sends no control command and grants no destructive authority.
2. **Current dashboard, read-only:** inspect discovery, exactly one enclosure, mounted volumes, protected capacity, SSD telemetry, events/history, bay indicators and stale/disconnect handling. Capture separate USB and Thunderbolt receipts. Measure the physical LEDs against the UI; never infer validation from a decorative background image.
3. **Model-specific controls:** only on the explicitly selected spare, test each allowlisted light operation and record the actual result. Power tests require all relevant volumes safely ejected, physical access and a separately accepted sequence. Keep partial/unknown outcomes unknown and do not retry automatically.
4. **Replacement-driver research:** first complete the review/provisioning and hardware-quiescence gates below. Preserve an original-driver rollback path. Begin with the separately approved read-only hardware bring-up plan. The existing hard-disabled prototype must not be enabled merely because a spare has arrived.
5. **Destructive pack research:** format/reset, replacement, redundancy changes, migration and recovery require a separate current plan and explicit execution authority. Make and hash member images before reconstruction experiments. The present scaffold implements none of these operations.

## Compiled driver versus working replacement

`platform/macos/driverkit/` is an independently written open-source prototype informed by analysis of the original driver and bridge firmware. `scripts/build_driverkit.py` builds `OpenDroboStorage.dext` for `arm64-apple-driverkit25.0`. The receipt explicitly states that hardware is disabled, the module is not installed/registered/activated, and it is not signed for distribution. This is not a recompile of recovered vendor source and is not evidence of working storage I/O or macOS-version compatibility.

Before activation, requirements include an approved signing/provisioning route, the appropriate Apple DriverKit PCI and SCSI-controller entitlements, a reviewed host activation/deactivation workflow, verified device matching, and actual evidence for initialization, interrupts, DMA ownership/quiescence, cancellation and read-only storage completion. The required hardware gate remains zero. Apple's [DriverKit entitlement process](https://developer.apple.com/system-extensions/) is separate from ordinary dashboard signing. This plan does not disable system protections or authorize installation.

A replacement host transport can use logical LUNs supplied by the existing enclosure firmware; it does not first require an independent BeyondRAID member-disk implementation. The firmware still owns the array, so that transport work does not itself open-source BeyondRAID.

The current Intel dashboard still uses the original driver and DDService. Finder capacity changes and independent BeyondRAID member assembly remain separate unimplemented paths. See [transport findings](../research/THUNDERBOLT_TRANSPORT.md) for recovered facts and missing proof.

## Extending model coverage

Group discovery/management adapters by verified transport rather than enclosure appearance. Same model names or shared protocol families do not prove identical boards, firmware, DMA semantics or support.

- 5D is the initial Thunderbolt target; other Thunderbolt models need their own identity, firmware, cable and behavior receipts.
- USB storage and USB configuration/service interfaces are different roles. Do not assume a visible USB socket exposes an array as host storage.
- NAS file sharing and management use a separate network adapter. The original [B800fs datasheet](https://content.etilize.com/Manufacturer-Brochure/1022617193.pdf) lists dual Ethernet and SMB/AFP; the [original family guide](https://guardiannetworksolutions.com/wp-content/uploads/2014/01/FS_B800fs_GSG_Rev02-US.pdf) lists B800i as iSCSI plus administration-only USB. A user-observed B800fs USB port needs exact role verification before any USB-storage claim.
- Community testers can provide additional model receipts later. New support requires implementation plus validation; a photograph, port count or matching board label alone is insufficient.
