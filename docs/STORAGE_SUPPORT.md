# Storage support matrix

OpenDrobo currently scopes volume and share reporting to verified Drobo-mounted paths on macOS. Other providers are future adapter work.

When evaluating a future adapter, keep three questions separate:

1. Can the host mount and report a filesystem?
2. Can OpenDrobo read or manage a provider's array metadata?
3. Can OpenDrobo assemble or reconstruct member disks?

A positive answer to one does not imply the others.

| Storage or format | Host-mounted visibility | OpenDrobo management | Member assembly/recovery |
| --- | --- | --- | --- |
| Observed Drobo 5D over macOS Thunderbolt; other models and paths unverified | Observed through the original macOS management driver; filesystem remains host-mounted | Read-only inventory, protected-pool snapshot, volume metadata and drive/SSD telemetry; experimental controls are mock-tested only | Not supported |
| HFS+ on a verified Drobo mount | Supported through native macOS statistics and Drobo binding | No filesystem mutation | Not applicable |
| APFS on a verified Drobo mount | Supported when macOS mounts it and Drobo binding is present | No filesystem mutation | Not applicable |
| Non-Drobo NTFS, exFAT, ext-family, XFS, Btrfs, ZFS | Not shown in current OpenDrobo product flow | No OpenDrobo filesystem or array management | Not supported |
| Linux MD RAID, Windows Storage Spaces, Apple RAID, other arrays | Future provider-adapter work; not current product support | No OpenDrobo RAID management or metadata parser | Not supported |

Drobo documentation and device research contain HFS+/NTFS references for supported desktop workflows. A separate brochure reference to EXT3 does not prove Drobo 5D Thunderbolt support on Linux. OpenDrobo therefore labels Linux Thunderbolt and native EXT access unverified and keeps them outside current product flow.

Protected-pool capacity and OS logical capacity are different measurements. A thin-provisioned or multi-LUN logical disk can show a large host ceiling while protected pool has smaller actual total. OpenDrobo displays both values and never shrinks READ CAPACITY or rewrites existing HFS+ geometry to make Finder appear different.

BeyondRAID work remains offline research. Repository contains bounded parsers and evidence notes for candidate headers and transport fields. No parser currently proves member order, generation selection, parity reconstruction, or safe write-back.

Spare-unit test preparation is available through the [lab scaffold](SPARE_UNIT_LAB.md). It generates offline manifests and ordered evidence plans before hardware arrives. Format/reset, member reconstruction and write-back remain future manual research gates; preparing a plan does not add those capabilities to OpenDrobo.
