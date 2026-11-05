# OpenDrobo

OpenDrobo is an independent effort to preserve Drobo management and investigate BeyondRAID interoperability. It uses a portable Rust core, a shared Tauri interface, and a macOS read-only bridge to the original management driver.

The interface follows Dashboard navigation with dark glass surfaces, animated selection glow, and optional locally extracted artwork. It discovers registry endpoints, performs bounded read-only capacity queries, and checks releases from [hypler-dev/OpenDrobo](https://github.com/hypler-dev/OpenDrobo). It does not install a driver, replace the Thunderbolt PCI/DMA path, write device state, flash firmware, or implement BeyondRAID recovery.

## Build and check

Tested local target: Apple silicon on macOS 26 with Xcode 26, Python 3.11+, Node 24+, npm, and Rust 1.98.0. Workspace-pinned ARM tools are used when present under `tools/`.

```sh
npm --prefix desktop ci
make check
make build
open dist/OpenDrobo.app
```

Frontend checks can also run directly with `npm --prefix desktop ci`, then `npm --prefix desktop run check` and `npm --prefix desktop run build`.

`make check` runs portable Rust tests, frontend type/build checks, desktop policy tests, native transport sanitizer checks, and direct macOS FSKit checks. `make build` creates a local debug Tauri app, bundles an inactive `OpenDroboFS.appex`, and ad-hoc signs the result. It does not install or activate the extension. The build is not notarized or ready for public distribution.

The registry-only command does not open a device or send management commands:

```sh
dist/OpenDrobo.app/Contents/MacOS/opendrobo-desktop --scan-json
```

Release checks are opt-in. They query only this project's latest GitHub release, reject drafts, prereleases, malformed versions, and links outside this repository, then open a validated release page for manual download. There is no automatic download or installation.

Controlled lab capture uses `--capture-capacity-json <registry-id>`. It returns the raw page as hexadecimal with an unverified-units label. No capacity number is presented as physical bytes until a named enclosure and spare-unit test establish units, device identity, and LUN scope.

## Capacity and Finder boundary

The Rust decoder accepts the recovered `0x7a/0x02` protected-capacity record and rejects truncation, stale data, inconsistent counts, unknown scope, and unconfirmed units. The macOS bridge uses only the recovered MODE SENSE path through the original user client: open, check LUN information, read the capacity page, and close. It rejects multiple LUNs and ambiguous physical-device scope.

The FSKit module implements an initial separate read-only filesystem view. It binds before and after opening the backing path, keeps held descriptors tied to their original inodes, confines symlinks, and withholds capacity statistics when units are unverified. It is compiled and directly tested, but has not been activated, mounted, or validated in Finder. It does not override an existing Apple HFS+ mount or change disk geometry. Swift is retained for this macOS-specific extension; the older SwiftUI app and isolated Swift capacity experiment remain reference code.

## Platform scope

The Rust core and Tauri UI are intended for Linux and Windows. Their device adapters, filesystem modules, installers, and hardware compatibility are not implemented or tested. The macOS bridge still depends on the original Drobo management driver; this repository does not ship a replacement kernel or DriverKit storage driver.

## Independent offline inspection

New source components decode copied Thunderbolt port/queue frames and inspect
an explicitly located BeyondRAID RMR header candidate. The offline probe does
not require a Drobo driver. It reads regular files only; it does not scan live
devices, map PCI memory, mount volumes, reconstruct array data, or establish
protected capacity.

```sh
cargo build --locked -p opendrobo-probe
cargo run --locked -p opendrobo-probe -- port path/to/copied-bar1.bin
cargo run --locked -p opendrobo-probe -- rmr path/to/member-image.bin 0xOFFSET
```

Replace `0xOFFSET` with an independently established byte offset. The tool
reads exactly one 8192-byte RMR window; it does not discover member locations
or generations. An unbound result remains a candidate with unverified identity
and `arrayValidated: false`. Port snapshots report virtual LUN ceilings and
keep protected capacity unavailable. Native execution has been checked only
on the local Apple silicon Mac. The probe rejects unverified host platforms
until their regular-file opening policy is implemented.

The non-loading DriverKit components now include DMA ownership, firmware-defined
interrupt operations, a version-4 read-only management queue, and SDK PCI/interrupt
resource adapters. Seven ARM64 SDK objects compile; four sanitizer suites check
mocked mapping, publication, completion, timeout, and removal failures. DMA writes
use the SDK's prepared mapping. Uncertain preparations retain resources for
cleanup; published timeouts and removals quarantine their mappings.
The app still uses the original driver: these components have no storage-controller
service/bundle, hardware activation, or Finder capacity proof. See the
[transport findings](research/THUNDERBOLT_TRANSPORT.md) for exact scope and remaining work.

Original installers, firmware, driver binaries, extracted artwork, decompilation output, and tool runtimes remain ignored local research artifacts. Optional artwork is local input recorded in [research/vendor-artwork.json](research/vendor-artwork.json); redistribution rights are unverified, so vendor assets are excluded from public source. Builds without local artwork use project-owned fallbacks. OpenDrobo is not affiliated with Drobo or its successors.

See the [work record](docs/WORK_RECORD.md), [capacity reporting](docs/CAPACITY_REPORTING.md), [recovered management ABI](research/MACOS_MANAGEMENT_ABI.md), [independent transport recovery](research/THUNDERBOLT_TRANSPORT.md), [RMR layout](research/BEYONDRAID_LAYOUT.md), [original-software findings](research/REVERSE_ENGINEERING.md), and [earlier Thunderbolt/BeyondRAID research](research/STATIC_RECOVERY.md).
