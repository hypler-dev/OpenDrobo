# OpenDrobo

OpenDrobo is an independent management console for Drobo storage, built around a portable core and shared interface. It brings device discovery, protected-pool capacity, mounted-volume statistics, drive and SSD telemetry, events, sharing visibility, and guarded management controls into one dark glass interface.

The first supported hardware path is a Drobo 5D over Thunderbolt to macOS. Broader 5D-class compatibility remains unverified. The console identifies enclosure and model, groups multiple logical volumes under one physical unit when host exposes that relationship, and keeps pool capacity separate from Finder filesystem view. It can show protected capacity recovered from original management service, OS-reported HFS+ usage, disk slots, and SSD life data when available.

OpenDrobo is not affiliated with Drobo. It is source-first software. No vendor binaries, firmware, artwork, credentials, or private host captures are distributed.

## What you can do

- Discover Drobo units and show brand, model, reported firmware, drive make/model, connection, enclosure state, and drive-slot presence.
- View protected pool total, used, and free bytes alongside each logical volume's host-reported total and usage.
- Inspect SSD capacity and reported life remaining without inventing cache usage when device does not expose it.
- Review bounded application and device events, console records, service state, time-range capacity statistics, allocation changes, recording gaps, and Drobo sharing metadata.
- Inspect native macOS SMB/NFS service status, list SMB exports whose configured paths fall beneath Drobo mount roots, and open macOS File Sharing settings. This path classification does not resolve symlink targets or grant file access. Non-Drobo volumes and unrelated SMB exports stay outside this product flow; macOS accounts and ACLs remain authoritative.
- Connect a trusted local assistant or script through optional read-only MCP for inventory, capacity, pool, storage, sharing, capabilities, console, events, control availability, and capacity history. See the [MCP guide](docs/MCP.md) for data access and trust boundaries.
- View experimental light/identify presets and power controls recovered from the original interface. They are offline-recovered and mock-tested only; no live hardware light or power control has been validated.

## Current support

macOS 14 or later is active desktop target. Intel macOS 14 and Apple silicon builds use shared Rust/Tauri application. Original Drobo management driver remains required for current Thunderbolt management path. Replacement DriverKit and filesystem research components compile or test in isolation; they are not installed, activated, or used to change Finder geometry.

Linux and Windows are planned adapter targets. Drobo Thunderbolt transport adapters, installers, device-management backends, and OpenDrobo host-volume adapters remain unimplemented there. Host-mounted filesystem support comes from each operating system, not OpenDrobo.

## Filesystem and RAID support

OpenDrobo currently reports Drobo volumes that macOS has mounted, using native host statistics and verified Drobo binding. HFS+ and APFS are visible when mounted on macOS. Non-Drobo volumes, generic host filesystem inventory, and unrelated SMB exports are outside current product flow.

OpenDrobo can inspect Drobo management data and BeyondRAID research, but it does not currently assemble member disks, reconstruct an array, format a disk pack, or repair a damaged volume. Other array providers and RAID formats are future adapter work, not current OpenDrobo support. Support matrix in [docs/STORAGE_SUPPORT.md](docs/STORAGE_SUPPORT.md) records boundaries.

## Get started

```sh
npm --prefix desktop ci
make check
make build
open dist/OpenDrobo.app
```

Run `--scan-json` for registry-only discovery. Run `--mcp-stdio` to expose fixed read-only MCP service to a local MCP client. Release checks are manual and point only to releases published in this repository; there is no automatic download or installation.

Read [docs/MANAGEMENT_FEATURES.md](docs/MANAGEMENT_FEATURES.md) for management surface, [docs/STORAGE_SUPPORT.md](docs/STORAGE_SUPPORT.md) for filesystem and RAID boundaries, and [docs/DEVELOPER_GUIDE.md](docs/DEVELOPER_GUIDE.md) for architecture, build, protocol, testing, and contribution details. Capacity evidence and Finder limits live in [docs/CAPACITY_REPORTING.md](docs/CAPACITY_REPORTING.md). [Firmware capabilities](docs/FIRMWARE_CAPABILITIES.md) explains further telemetry opportunities and the limits of using macOS RAID with Drobo.

OpenDrobo remains conservative around hardware. It does not install drivers, activate extensions, mount filesystems, flash firmware, reset arrays, format disks, erase data, or claim Finder support without hardware and filesystem evidence.
