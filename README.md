# OpenDrobo

An independent project to preserve Drobo management on modern macOS and investigate BeyondRAID interoperability.

The native app follows the original Drobo Dashboard navigation and layout. It currently provides local, read-only registry discovery and checks for app releases from [hypler-dev/OpenDrobo](https://github.com/hypler-dev/OpenDrobo). It does **not** yet provide Thunderbolt storage transport, hardware health readings, firmware updates, or a BeyondRAID reader/writer.

## Build and run

Requirements: macOS 26 or newer, Xcode 26 with command-line tools, and Python 3.11 or newer. The app builds as a universal `arm64` / `x86_64` binary. Only Apple silicon execution has been checked locally.

```sh
make check
make build
open dist/OpenDrobo.app
```

No external source checkout is required. Builds are ad-hoc signed for local testing; no Developer ID provisioning, notarization, driver installation, or system changes occur.

The command-line discovery mode returns registry metadata, without opening a device or sending SCSI commands:

```sh
dist/OpenDrobo.app/Contents/MacOS/OpenDrobo --scan-json
```

## Updates

Dashboard Preferences checks `https://api.github.com/repos/hypler-dev/OpenDrobo/releases/latest`. Automatic checks are opt-in. The checker rejects draft/prerelease records, uncomparable tags, and links outside this project's GitHub release pages. Available releases open in the browser for manual download. Automatic download/install and signed update delivery are future work.

No release is published yet. The correct current response is “No app releases have been published yet.” App updates and enclosure firmware updates are separate capabilities.

## Artwork and source

The build accepts optional original artwork in `artifacts/artwork`. Original assets are used only in the local research build and carry recorded provenance in [research/vendor-artwork.json](research/vendor-artwork.json). Their redistribution license has not been verified, so they are excluded from source control. Builds without them use native fallback symbols. Extracted Drobo installers, drivers, firmware, decompilation outputs, and tool runtimes are also local ignored artifacts.

The app compiles this repository's native source. Device protocol parsing, management transport, and BeyondRAID implementation remain future work. Research tools and external reference repositories are recorded in [research/upstreams.json](research/upstreams.json); they are not application build dependencies.

## Research

- [Current work record and validation](docs/WORK_RECORD.md)
- [Original software and firmware findings](research/REVERSE_ENGINEERING.md)
- [Thunderbolt and storage metadata recovery](research/STATIC_RECOVERY.md)
- [Platform/network capabilities and community backlog](docs/CAPABILITIES_AND_BACKLOG.md)

Offline firmware inspection is reproducible:

```sh
python3 scripts/acquire_firmware.py
python3 scripts/inspect_firmware.py path/to/firmware.tdf --output-dir artifacts/new-inventory
```

The inspector checks TDIH lengths and CRCs, inventories ARM ELF images, and can extract embedded images and their symbol tables. It never executes or installs firmware. A CRC match establishes consistency, not vendor authenticity.

OpenDrobo is not affiliated with Drobo or its successors. New code is MIT licensed; original third-party notices remain applicable.
