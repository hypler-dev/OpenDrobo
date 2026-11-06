# OpenDrobo work record

Last updated: November 6, 2026. This is the canonical status and acceptance record.

## Latest accepted packet: device-focused Dashboard, statistics and MCP boundaries

The user requested a closer reconstruction of the original Dashboard's screen organization, component selection, enclosure graphics, capacity display, and settings locations, with new functionality kept accessible alongside it. The original local XAML resources, English labels, and artwork inventory provide layout evidence; they do not establish full functional parity. This packet replaces the generic status-card layout with a component inspector and selectable drive bays, restores Capacity/Usage and original Tools/General settings placement, and places additional diagnostics under Advanced. Unknown values and unsupported operations must remain explicitly unavailable. Runtime UI and device checks target the home Mac; local checks remain offline software validation.

The MCP assessment covers the current desktop source and its host/service boundaries. The independent baseline reported no candidate; a separate focused reviewer found an unauthenticated loopback-source boundary. Root validated that local port 4502 can supply structurally valid observations without peer authentication, and accepted one **low-severity, high-confidence CWE-345** finding: a local process that obtains the port while DDService is absent can forge displayed metadata and poison bounded capacity history. This does not establish remote exposure, MCP control execution, arbitrary file access, or a disk-pack write. `sourceTrust: unverified-loopback` records that limitation; it does not remediate peer authentication. The [MCP product guide](MCP.md) documents purpose, intentional data disclosure, and operational limits.

The Codex Security managed scan (`7bef81a8-678a-43f7-a184-a3773956952c`) could not save canonical artifacts. Its tool repeatedly returned `Scan output parent must not be group- or world-writable without the sticky bit.` The authoritative output path has a `/Volumes/Storage` ancestor with mode `0775`; all scan descendants were private. No existing-scan relocation tool was available. Root did not change volume permissions, weaken plugin checks, or write replacement canonical artifacts. The scan was marked failed for this confirmed storage blocker; no sealed report or successful scan completion is claimed. The original candidate, generated model, and source validation were saved only through the tool's temporary artifact storage and may be cleaned by the OS. This is a source-backed assessment with an incomplete managed publication lane, not a clean security certification. No runtime exploit was performed. Token usage was not returned.

The latest source passes all 22 software-check lanes, including 41 desktop backend tests, nine statistics tests, six Status-selection tests, four drive-indicator tests, 40 portable core tests, seven probe tests and 29 Python tests. Intel and ARM64 preview packages pass packaging checks. Ten SDK MCP tool calls pass against the final private Intel preview on the home Mac. The current runtime observations and remaining UI-input verification are recorded below. The original driver/service, Finder geometry, driver activation, filesystem mounts, live light/power tests and disk-pack mutation remain outside this packet.

The user subsequently accepted optional MCP and requested more elaborate statistics plus a firmware-grounded feature assessment and an explanation of macOS software RAID. Extend Statistics with actual time-range selection, protected/free/OS-used trends, utilization and signed changes, observation gaps, reported drive capacities and SSD life, and session-scoped activity. Byte arithmetic must remain exact, missing observations must not become zero or an outage, and growth rates must be described as observed averages rather than forecasts. Additional telemetry command names are research leads until their payloads, units and model capability are recovered. Neither original firmware nor the BeyondRAID on-disk format has been modified. No AppleRAID conversion, member-disk passthrough, formatting or array migration is authorized by this assessment. The follow-up question about connecting two matching Drobos is an architecture assessment: Thunderbolt chaining is connectivity, while a potential host-managed mirror or stripe would retain two separate onboard BeyondRAID pools. OpenDrobo has not implemented or validated this arrangement; virtual-member ancestry, mount guards, thin-provisioned capacity and failure recovery remain open. See [firmware capabilities](FIRMWARE_CAPABILITIES.md).

The user then identified the stretched Status selector in the current home-Mac view and requested visible options, better organization and device-specific settings. Replace that dropdown with visible System/Drive choices, remove the collapsed Advanced navigation group, and show only verified settings for the explicitly selected device. Remove irrelevant NAS forms from the 5D flow and retain native host Sharing as a separate page. Raw logical geometry remains closed technical detail, as previously requested.

The next accepted Status amendment automatically opens the sole observed Drobo; with multiple units it presents enclosure icons and opens System Information for the selected identity. Selection is read-only: automatic display changes must not grant a control target, and device disappearance clears any prior control authority. Multiple identical models need distinct opaque unit labels. The user authorized briefly resuming Screen Sharing control to verify the interface. Device actions remain outside the home test.

The user also requested original-style green/amber/red enclosure indicators. Recover the original per-slot LED selector before applying colors; keep unknown or stale observations gray, expose color-independent text, and leave physical lights unchanged. This amendment does not decode every health bit or authorize device-control execution.

## Accepted intent

The user authorized an independent OpenDrobo application with the original Dashboard interaction model, modern macOS/Apple silicon support, a Rust core, a shared Tauri UI, and offline reverse engineering of the original service, driver, and firmware. The user also authorized publication of the latest source updates to the public `hypler-dev/OpenDrobo` repository. Long-term scope includes BeyondRAID recovery and Linux/Windows adapters.

The former third-party application dependency has been removed. No external parser checkout or copied parser is part of the build. Original binaries, firmware, extracted artwork, and REA/Ghidra output remain ignored local research artifacts with recorded provenance. New parser code comes from direct analysis of the original vendor software.

## Current implementation

- `crates/opendrobo-core`: portable Rust capacity decoder/C ABI, copied Thunderbolt port/frame codecs, and an offline RMR header-candidate inspector.
- `crates/opendrobo-probe`: a bounded, file-only inspection CLI independent of the original Drobo driver.
- `desktop/`: shared Tauri 2 TypeScript/CSS interface with original-style navigation, dark glass surfaces, animated selection glow, empty states, and release lookup. The shared app targets macOS 14+; an Intel preview has run on macOS 14.7.7 with the optional macOS-26 FSKit module excluded.
- `desktop/src-tauri/`: Rust commands for registry discovery, guarded raw capacity, original-service protected-pool/slot observations, device events, sanitized Console history, read-only stdio MCP, OS storage/sharing inspection, fixed GUI controls and scoped GitHub release checks.
- `platform/macos/transport/`: C bridge for original user-client ABI. Query is open → LUN page 4 → capacity page 2 → close; no arbitrary or write commands.
- `platform/macos/fskit/`: Swift FSKit read-only separate-view module with path/inode binding, held descriptors, directory enumeration, and conservative symlink confinement.
- `platform/macos/driverkit/`: an ARM64 SDK/IIG SCSI service, read-only policy, DMA leases, interrupt interpreter, bounded version-4 storage/management engines, inline logical-geometry replies, completion-based graceful drain, and SDK resource adapters. The local `.dext` is disabled before hardware access and has not been installed or activated.

The native source intentionally passes `byteUnitsConfirmed: false`. Injected FSKit tests exercise a positive confirmed record, but live provider statistics remain unavailable until hardware evidence exists. The older `macos/` SwiftUI app and `experiments/capacity/` Swift prototype remain reference code; `make build` uses the Rust/Tauri application.

The macOS bridge depends on the original Drobo management driver already being loaded. It does not replace the Thunderbolt PCI/DMA or DriverKit storage driver. Linux and Windows currently have shared source direction only; neither target has been built or tested here.

## Capacity truth

Protected-pool values can now be read from the original local service and displayed as service-reported bytes, separately from OS logical volume totals/usage and SSD physical capacity. [CAPACITY_REPORTING.md](CAPACITY_REPORTING.md) explains these accounting layers; [MACOS_MANAGEMENT_ABI.md](../research/MACOS_MANAGEMENT_ABI.md) records both the raw user-client ABI and local service/control protocol. Raw device byte units are **not hardware-certified**, so the direct decoder/FSKit model-profile gate remains closed. Service values do not silently enable that separate path.

The FSKit view does not override an existing Apple HFS+ mount or change disk geometry. The extension is compiled and directly checked, but has not been activated, mounted, or validated in Finder. Its nonthrowing statistics callback returns zeroed fields when information is unavailable; those zeros are not a measured empty pack. The later authorized home-Mac capacity trial confirmed mounted Journaled HFS+ filesystems and one service pool spanning two LUNs, with five hard drives and an SSD. The earlier discovery-only record below is historical. Direct pool-diagnostic opens returned exclusive access while the original service owned the connection; that service was left running.

## Validation and packaging

The prior application checkpoint passed all 15 then-current `make check` lanes: 25 Python firmware/archive tests, 16 Rust core tests, 1 Tauri backend policy test, 25 legacy Swift policy assertions, 91 historical capacity-prototype assertions, formatting/build checks, native transport ASAN/UBSAN mocks, warning-clean C syntax, and FSKit typecheck/direct checks. After the final icon-rendering adjustment, frontend typechecking and production asset compilation passed again. The new driver-independence slice has the updated counts below. These are software checks, not hardware or Finder proof.

`make build` passed and produced the local ARM64 debug preview at `dist/OpenDrobo.app`. It embeds the inactive FSKit extension under `Contents/Extensions`, includes optional ignored local artwork, and ad-hoc signs the nested extension and host. Strict signature verification passed for both, and for the app extracted from its ZIP with `ditto`. The initial misplaced entitlements-file packaging failure was corrected before this final package. No installer, activation, notarization, public binary upload, or release occurred.

Prior reviewed application archive: `dist/OpenDrobo-0.2.0-preview-macos-arm64.zip`. SHA-256: `180547f9311a62fcf70d2d122e4e37b46e20a82cba3f5cea4eb6c8a3d72d65b1`. This source-only driver-independence slice does not replace that archive or change the running UI.

The controller observed a clean quit and final-bundle relaunch on macOS 26/Apple silicon. The 1120 × 800 render showed readable original navigation glyphs, dark glass panels, the faded product lineup, and RGB borders confined to their controls. List selection, registry refresh, unavailable management sections, and the Preferences panel were checked. Automatic release checks remained off; manual Check Now progressed through its disabled busy state to “No app releases have been published yet.” The empty local scan returned `{platform, devices, message}` with no devices. Invalid raw-capture arguments returned the expected errors before device access. Runtime OS Reduce Motion/Transparency settings were not changed, and no performance benchmark was run.

Local receipts:

- `artifacts/reports/desktop-checks/results.json` and its 15 lane logs.
- `artifacts/reports/desktop-build.log` and `desktop-build.json`.
- `artifacts/reports/desktop-validation.json` and `desktop-ui-observations.json`.
- Original analysis receipts under `artifacts/reports/capacity` and `artifacts/reports/transport`.

The controller owns architecture, native transport/backend integration, validation, and publication judgment. Producer tasks `rust_core`, `tauri_frontend`, and `fskit_module` used the native OpenAI `gpt_worker` role. Fresh `module_review` and `native_review` used `gpt_reviewer`; `shared_ui_review` used `gpt_ui_review`. All returned PASS after their findings were resolved. Fixes included explicit unit gating, binding before/after capture and root opening, root-path replacement checks, confined symlinks, lifecycle serialization, nontrapping directory verifiers, overlap-safe C ABI decoding, cross-platform bundle targets, and UI busy states. UI review used controller-supplied rendered observations plus independent source review; the reviewer did not operate the GUI.

The worker and reviewer TOMLs configure OpenAI `gpt-5.6-terra`; `architecture_docs` used `gpt_docs`, configured as `gpt-5.6-luna`. Actual runtime model IDs are not exposed, so configuration pins are not represented as observed runtime identities. No GLM, Spark, Hermes, Gemma, or Mac mini model was used.

## Source checkpoint

The prior application slice introduced the root Cargo workspace and build entry points, `crates/opendrobo-core`, `desktop`, `platform/macos`, and the management-ABI research note. The optional Swift reference build has a separate output and bundle identity. This follow-up adds the two offline core modules, probe crate, DriverKit task-planning component, targeted validators, and recovery/status documentation. Original vendor assets, generated artwork/schema directories, dependencies, toolchains, and build products remain ignored. There was no unrelated working-tree dirt.

This follow-up started from published checkpoint `76be5b2d9ee0382caa9d78bb3a6d16f88c85a5a2`. This record's containing commit identifies the new source checkpoint and exact changed paths. Confirm publication by comparing `git rev-parse HEAD` with `git ls-remote origin refs/heads/main`; the closeout records that result. No tag, app release, deployment, driver installation, extension activation, filesystem mount, disk-pack write, or firmware operation is part of this checkpoint.

## Open gates

Open hardware gates include independent raw byte-unit/model certification, adapter-chain completeness, Finder statistics, disconnect behavior and per-volume/shared-pool presentation. The later home trial confirms macOS 14.7.7/Intel, a service-identified 5D Thunderbolt pool, two mounted HFS+ volumes and SSD telemetry; it does not validate replacement-driver operation. BeyondRAID reconstruction remains research-only. Network management remains proposed authenticated host agent. Source checks do not authorize firmware, restart, shutdown, format, rebuild, array mutation or disk erase; home-Mac access requires its separate accepted trial scope below.

## Active driver-independence research

The user requested continued Thunderbolt and BeyondRAID reverse engineering to remove the original driver dependency and eventually report actual capacity in Finder. This extends the accepted source work. The current bridge remains dependent on the original driver until a replacement transport has been implemented and tested; offline codecs alone do not remove that dependency.

The controller recovered the original PCI/BAR directory, shared request/completion framing, channel routing, and transport-manager layout. Selected directory and queue-framing observations were also checked against the original arm64e slice. A bounded worker resolved the production RMR header/CRC-buffer alias: both point to the same 8192-byte DataWindow, with checksum bytes at `0x10..0x13`. The CRC uses zero initialization, no final XOR, and reflected polynomial `0xedb88320`. Header identity words, revision rules, and shift bounds are checked without inferring disk placement or generation selection. The [transport](../research/THUNDERBOLT_TRANSPORT.md) and [RMR](../research/BEYONDRAID_LAYOUT.md) notes preserve exact evidence and unknowns.

Acceptance for this offline slice requires original-binary hashes, exact query/assembly receipts, independently written bounded code for recovered fields, meaningful truncation/overflow/error tests, and fresh independent review. Hardware-independent DriverKit compilation may establish API/build conformance; it cannot establish activation, hardware behavior, or Finder compatibility. The existing byte-unit gate stays closed until verified evidence supports a model profile. No extension activation, filesystem mount, home-Mac access, firmware execution, or disk-pack mutation is included.

Implementation and validation:

- `make check` passes all 18 current lanes. Rust coverage is 40 core tests (16 capacity, 12 transport, 12 RMR) plus 7 probe tests; existing Python, Swift, frontend, native bridge, and backend checks still pass.
- `scripts/check_driverkit.py` compiles the SDK adapter and policy as ARM64 DriverKit objects, then checks the pure policy with ASAN/UBSAN. It rejects write/vendor/firmware/power commands, unsupported target/LUN scope, invalid transfer sizes/directions, and 32-bit DMA-range overflow. There is no controller registration, DMA submission, or new live backend.
- `cargo build --locked -p opendrobo-probe` produces a native ARM64 CLI. Eight additional process-level cases passed against explicitly synthetic files, including three independently JSON-decoded success responses, malformed/checksum/identity/overflow failures, and unchanged input hashes. No real member image or BAR capture was available.
- The probe reads only copied regular files. It rejects symlinks, directories, and FIFOs; uses nonblocking/nofollow flags on the implemented macOS/Linux x86_64/aarch64 paths; and fails before metadata/open on unimplemented hosts, including Windows. Only the local macOS ARM64 runtime was tested. RMR checks read one explicit 8192-byte image window. Output remains a candidate with unverified or caller-supplied identity, `arrayValidated: false`, and protected capacity unavailable.
- Direct bare Cargo tests inherit a denied temporary directory in this managed sandbox. The repository validator sets `TMPDIR=/private/tmp`; named validation passes with that environment. This is an environment boundary, not a skipped source-test failure.

Fresh `independence_review` (`gpt_reviewer`, OpenAI) returned PASS after an unsupported-host file-opening gap was fixed. It independently reviewed codecs, RMR evidence, the SDK adapter/policy, probe, validators, and the bounded ARM/documentation claims. Producers `thunderbolt_codec`, `rmr_candidate_parser`, and `offline_probe` used `gpt_worker`; `beyondraid_recovery2` performed the bounded firmware trace, and a `gpt_reader` supplied exact SDK evidence. These roles use the configuration pins stated above; runtime model IDs remain unexposed. No additional provider or retired model lane was used.

Latest ignored receipts: `artifacts/reports/desktop-checks/results.json` (18 successful lanes), `artifacts/reports/independence/probe-validation.json`, original REA query receipts under `independence/thunderbolt` and `independence/beyondraid`, ARM corroboration excerpts, and official Apple references under `independence/driverkit`. Input fixtures are explicitly synthetic and remain outside public source.

## DMA, interrupt, and controller slice

The user authorized continued work on DMA, interrupts, and controller lifecycle. This extends published checkpoint `031f95998bebe8cd6e615d86198b26aa853d369e` with a bounded, non-loading component. Acceptance requires actual ARM64 SDK compilation and failure-injected mock-MMIO/DMA checks, while all live activation, hardware, mount, and unit/scope gates remain closed.

Original REA/Ghidra queries recovered the 84-byte interrupt entry, five operator semantics, local-only state transformation, TM request/response/summary selection, DMA limits, and removal ordering. Selected interrupt operations and the 104-byte single-segment TM branch were independently checked against the original arm64e disassembly. The original slice hashes are unchanged. Exact facts, addresses and remaining unknowns are in [THUNDERBOLT_TRANSPORT.md](../research/THUNDERBOLT_TRANSPORT.md); raw queries and selected assembly remain ignored under `artifacts/reports/controller/`.

Implementation:

- `platform/macos/driverkit/DmaOwnership.h` and `.c`: exact-range mapping/permission checks, retained resources on uncertain preparation or failed completion, and explicit quarantine/drain gates.
- `platform/macos/driverkit/DriverKitDma.h` and `.cpp`: actual SDK calls bound to the opened PCI provider, per-command provider retention, bounded SDK segment copying, and request writes through the prepared mapping.
- `platform/macos/driverkit/InterruptProgram.h` and `.c`: bounded definitions, local state evaluation, ordered injected register operations, and sentinel/failure handling.
- `platform/macos/driverkit/ControllerTransport.h` and `.c`: one-shot version-4, target-0/LUN-0 read-only management queue with checked regions/windows, same-provider DMA, independent transaction IDs, completion association, timeout/removal quarantine, partial-enable cleanup and retry. Storage/inline emulation is unsupported here.
- `platform/macos/driverkit/DriverKitSession.h` and `.cpp`: exclusive open, BAR lookup, bounded SDK accessors, bound-controller/interrupt gates before bus mastering, and asynchronous disable/cancel before release. It cannot establish outstanding DMA quiescence.
- `platform/macos/driverkit/DmaChecks.c`, `InterruptChecks.c`, and `ControllerChecks.c`: synthetic ownership/MMIO/clock fixtures and failure injection. `scripts/check_driverkit.py` compiles seven SDK objects and runs four sanitizer suites including the existing policy suite.
- `README.md`, this record, `docs/CAPACITY_REPORTING.md`, and `research/THUNDERBOLT_TRANSPORT.md`: source scope, recovered evidence, validation and remaining hardware limits.

Validation: all 18 `make check` lanes passed. The focused DriverKit lane was rerun after subsequent native changes; its latest result is recorded below. The controller tests include 15 submission failure positions, partial startup, 17 shutdown failure positions with retry, independent wire bytes, foreign-provider rejection, multiple slots/out-of-order replies, stale slot reuse, permissions, timeout/removal, and explicit interrupt/drain gates. Actual SDK PCI/DMA/interrupt calls are compiled only, while host tests use synthetic backends. No full replacement service or live DMA transfer has run. The unchanged app/FSKit package was not rebuilt for this source-only slice; no public binary release was created.

The controller owns integration, architecture and release judgment. Native OpenAI `gpt_worker` tasks `dma_ownership` and `interrupt_program` produced bounded disjoint components; `gpt_reader` task `dma_sdk_reader` supplied SDK and ARM evidence. Fresh `gpt_reviewer` task `controller_review` returned **PASS, no actionable P1/P2/P3**, after final source and receipt review. Findings resolved provider/direction binding, partial-interrupt cleanup, the declared maximum-LUN bound, and documentation precision. Worker/reviewer TOMLs configure `gpt-5.6-terra`; reader config is `gpt-5.6-luna`. Runtime IDs are not exposed. No GLM, Spark or retired-model lane was used. The review is evidence for controller judgment, not independent publication authority.

Final focused result: `python3 scripts/check_driverkit.py` with `TMPDIR=/private/tmp` exited 0 after the last native change, compiling seven ARM64 objects and passing all four sanitizer suites. Direct stdout/exit receipts are `artifacts/reports/controller/final-driverkit-check.log` and `.json`; `controller/validation.json` binds the final native source hashes to that result. The full 18-lane receipt remains `artifacts/reports/desktop-checks/results.json`. The containing source commit is this slice's checkpoint. Publication uses the user's standing source-push authorization; verify `HEAD` against `git ls-remote origin refs/heads/main` for the public result.

No unrelated source dirt was present. Ignored vendor inputs, extracted artwork, REA output, installed toolchains, generated caches and build products remain outside publication. No driver installation, registration, activation, filesystem mount, home-Mac access, disk-pack write, firmware execution, reset, flash, tag, deployment or app release occurred.

## SCSI service and storage-callback slice

The user's request accepted this continuation from `ba319e388f8935c9ae02444fe692c0c09dd7f9c1`: real SDK/IIG controller service, target/ordinary asynchronous task callbacks, bounded storage reads, and inline logical-geometry responses. Acceptance requires SDK generation/compile/link, meaningful host mocks, repository checks, fresh independent review, and source-only publication. No activation, device command, mount, disk-pack write, home-Mac access, firmware operation or public binary release is included.

Implementation and exact changed paths:

- `platform/macos/driverkit/OpenDroboSCSIController.iig` and `OpenDroboSCSIController.cpp`: actual framework subclass, target and normal asynchronous task API, default/auxiliary queue separation, buffer acquisition/remapping, retained actions, timer/interrupt callbacks, and serialized close. Unsupported task-management/reset operations are rejected. Source execution is hard-disabled before PCI open or framework Start.
- `platform/macos/driverkit/ScsiTaskTable.h`, `ScsiTaskTable.c`, and `ScsiTaskChecks.c`: shared task/action ownership, duplicate/transaction checks, live-DMA completion refusal, callback snapshots and destructive/reentrant callback tests.
- `platform/macos/driverkit/ScsiResponses.h`, `ScsiResponses.c`, and `ScsiResponseChecks.c`: bounded inline TUR, READ CAPACITY(10/16), REPORT LUNS and INQUIRY standard/VPD00/80. Geometry remains the original logical ceiling; identity describes the project bridge/registry attachment, not verified physical hardware.
- `platform/macos/driverkit/ControllerTransport.h`, `ControllerTransport.c`, and `ControllerChecks.c`: separate storage/management directory and interrupt selection, checked 512-byte logical geometry, read bounds, 144-byte experimental storage frame, external shared-cause ownership and startup state query. Existing management API remains available.
- `platform/macos/driverkit/DriverInfo.plist`, `scripts/build_driverkit.py`, and `scripts/check_driverkit.py`: local inactive research bundle metadata/build, architecture-neutral IIG generation, eleven ARM64 object compilations, framework link and six host sanitizer suites. No provisioning, installation or activation action exists in these scripts.
- `README.md`, `docs/CAPACITY_REPORTING.md`, `research/THUNDERBOLT_TRANSPORT.md`, and this record: source/build scope, original evidence, hardware limits and continuation.

Original recovery at this checkpoint: REA queried the storage builder at x86 `0x6d9a`, inline handlers and target initializer. Both original driver slice hashes were rechecked and unchanged. A bounded independent ARM read corroborated fixed extension/field offsets in the ARM builder at `0x12a2c`, without certifying all field semantics. Raw receipts and selected evidence remain ignored under `artifacts/reports/scsi`. Storage `+0x38` was then unresolved: the original writes its host virtual SGL address there, while OpenDrobo writes zero. The storage-field/drain slice below resolves its meaning for the examined 5D read paths, correcting the earlier "host cookie" description. Hardware compatibility remains unproven. `od_scsi_service_hardware_enabled()` returns zero and no plist/preference can open that gate.

Validation: `env TMPDIR=/private/tmp make check` exited 0, all 18 lanes passed. The expanded DriverKit lane generated IIG code, compiled eleven ARM64 objects, linked an inactive `.dext`, and passed six ASAN/UBSAN suites. `python3 scripts/build_driverkit.py` also exited 0 against final source, producing `artifacts/driverkit/OpenDroboStorage.dext`. `file` identifies an ARM64 Mach-O executable; its load commands reference Apple DriverKit, PCIDriverKit, SCSIControllerDriverKit and the SDK C++ runtime. No original vendor driver is linked into this research bundle. This is build/link and mock proof only; SDK callbacks and PCI/DMA methods were not executed. The application/FSKit package was not rebuilt or changed for this source-only slice.

Native OpenAI `gpt_worker` tasks `scsi_inline` and `scsi_storage_transport` produced disjoint leaf components. The controller produced and integrated the SDK service, callback coordinator and build/check scripts. `gpt_reader` supplied SDK and ARM evidence. Fresh OpenAI `gpt_reviewer` task `scsi_service_review` returned **PASS, no actionable P1/P2/P3**, after integration fixes and final evidence review. Fixes covered partial-initialization cleanup, failed-Start pin release without assuming Stop, explicit service pins, callback-table destruction safety, and shared-summary startup validation. Worker/reviewer configurations pin `gpt-5.6-terra`; reader pins `gpt-5.6-luna`; runtime model IDs are not exposed. No GLM, Spark or retired-model lane was used.

Receipts: `artifacts/reports/scsi/full-check.log`, `check.log`, `build.log`, `artifacts/reports/desktop-checks/results.json`, and `artifacts/driverkit/build.json`. The containing source commit is this checkpoint; public-main equality is verified at publication. No unrelated source dirt was present. Vendor binaries/artwork, decompilation, toolchains, generated IIG files, caches and the `.dext` remain ignored and excluded from publication. No driver installation, registration, activation, live SCSI command, mount, data-pack write, firmware operation, deployment, tag or binary release occurred.

## Storage-field semantics and completion-based drain

The user's current request accepted storage-field and DMA-drain continuation from `824ecc951aba538d7b1f494a4631372dc81b858e`: trace original host/firmware consumers and implement graceful shutdown through individually validated completions while refusing new work. Acceptance requires exact original/SDK evidence, shared mock-tested drain policy and service integration, full checks, fresh independent review, and source-only publication. Zero response counts, PCI Close, resets and mapping release cannot become fabricated global DMA-drain proof. Hardware, mount, firmware execution and binary-release gates remain closed.

Recovered evidence: the 5D Thunderbolt bridge image has retained DWARF identifying SRB `+0x38` as `request.dataBuffer`, rather than a host cookie. Numeric ARM assembly in single-pass and multi-issue read-response paths selects that field for flag `0x08`, or the inline SGL at SRB `+0x80` for flag `0x10`. OpenDrobo's existing flags `0x400032` select the latter and clear the former, supporting its zero inactive direct-buffer field. No wire bytes change in this slice. This is scoped to examined 5D paths, not all models/firmware. The bridge is `5D.elf`, distinct from the `5D-runesa.elf` BeyondRAID processor. Original inputs, exact addresses, hashes, DWARF types, DMA wait/completion ordering and limits are in [THUNDERBOLT_TRANSPORT.md](../research/THUNDERBOLT_TRANSPORT.md).

Exact changed paths:

- `platform/macos/driverkit/ScsiDrain.h`, `ScsiDrain.c`, and `ScsiDrainChecks.c`: new caller-owned admission/drain policy, bounded action/lease counts, fixed monotonic deadline, terminal quarantine, nonextendable repeated begin, reinitialization refusal, and shared controller fault classification.
- `platform/macos/driverkit/OpenDroboSCSIController.cpp`: Stop/power-down closes admission while timer/interrupt polling continues; zero live actions and leases permit normal source closure. Fault/deadline closes sources with unfinished ownership retained. Lifecycle acknowledgment and free additionally refuse live leases. Poll/summary/submit faults preserve protocol, timeout or removal cause. Source cleanup still passes false for global bus-master quiescence.
- `platform/macos/driverkit/ControllerTransport.c`: correct and scope the direct-buffer/SGL comment; framing is unchanged.
- `platform/macos/driverkit/ControllerChecks.c`: connect the actual core queue, DMA leases, task table and drain policy. Empty responses cannot finish live work; matched completion must finish both leases and the retained action. Stale replies, request timeout and channel/shared-summary removal preserve mappings and actions. Test-only external-quiescence cleanup is explicitly separate from service behavior.
- `scripts/build_driverkit.py` and `scripts/check_driverkit.py`: include the new policy in the SDK build and add its seventh sanitizer suite.
- `README.md`, `docs/CAPACITY_REPORTING.md`, `research/THUNDERBOLT_TRANSPORT.md`, and this record: evidence, current counts, honest capacity boundary and continuation.

Validation: `env TMPDIR=/private/tmp make check` exited 0, all 18 lanes passed. The DriverKit lane generated IIG code, compiled twelve ARM64 objects, linked an inactive `.dext`, and passed seven ASAN/UBSAN suites. The focused native check also passed. `python3 scripts/build_driverkit.py` exited 0 against unchanged final native source, producing the local ARM64 `artifacts/driverkit/OpenDroboStorage.dext`, linked only to Apple DriverKit/PCI/SCSI frameworks and the SDK C++ runtime. SDK callbacks and PCI/DMA calls compile/link only; host tests execute synthetic backends. Initial sandbox run stopped at Swift's compiler-module cache with `Operation not permitted`; the identical offline full check passed with approved compiler-cache access. The failed receipt is retained separately, not reported as a source failure. App/FSKit packaging was not rebuilt and Finder behavior remains unchanged.

Native OpenAI `gpt_worker` task `graceful_drain` produced the three leaf policy/check files. Root produced SDK/core integration, fault-path tests, original analysis, docs and validators. Native `gpt_reader` task `storage_field_reader` supplied bounded DWARF/ARM evidence. Fresh `gpt_reviewer` task `drain_integration_review` returned **PASS, no actionable P1/P2/P3**, after reviewing the final 12-path source/docs slice, original evidence and validation receipts. Initial findings required controller fault-cause propagation, retained-action checks on all fault paths, and narrower firmware wording; root applied those fixes before the passing full check. Worker/reviewer TOMLs configure `gpt-5.6-terra`; reader config pins `gpt-5.6-luna`. Runtime IDs are not exposed. No GLM, Spark or retired-model lane was used.

Receipts: `artifacts/reports/drain/validation.json` binds native source hashes before/after full check and persistent build; `full-check.log`, `check.log`, and `build.log` retain results. `artifacts/reports/desktop-checks/results.json` records all 18 lanes; `artifacts/driverkit/build.json` records exact IIG/compile/link commands. `drain/bridge/llvm-receipts.json` binds original hashes and exact disassembly/DWARF commands; bridge/host REA receipts retain query arguments and evidence IDs. This record's containing commit is the source checkpoint. Verify public-main equality at publication.

No unrelated source dirt was present. Vendor inputs/artwork, raw decompilation, LLVM excerpts, toolchains, caches, generated IIG code and the local `.dext` remain ignored. No driver installation/registration/activation, live enclosure command, filesystem mount, home-Mac access, disk-pack write, firmware execution/reset/flash, deployment, tag or binary release occurred. Hardware enable and capacity-unit confirmation remain false. Per-request completion accounting is improved; global hardware quiescence, original-driver-free operation and Finder capacity remain unproven.

## Home Mac launch and discovery trial

The user's November 5 request authorizes launching OpenDrobo on the home Mac to check connected-device discovery. This supersedes the earlier home-access exclusion only for this bounded application trial: read-only host/registry inspection, copying a compatible preview into an isolated user directory, launching that app, and checking its own discovery output. It does not authorize replacement-driver activation, filesystem mounts, firmware commands, disk-pack writes, or changes to unrelated host services.

Live preflight identified an Intel Mac running macOS 14.7.7, with the original Drobo service running and two registered management endpoints identifying as `5D - TB`. The prior ARM64/macOS-26 preview could not run there. The reviewed shared application now targets macOS 14; FSKit remains native ARM64/macOS 26 and is excluded from the Intel trial. Registry media expose HFS-type partitions beneath a 64 TiB logical disk and a roughly 2 TB logical disk. Two endpoints are not proof of two physical enclosures or verified single-LUN scope. Mounted-filesystem identity, original-driver independence and capacity byte units remain unproven.

Exact changed paths: `scripts/build_desktop.py` adds bounded `--target`, `--without-fskit` and `--output` options, target-correct source/archive paths and metadata, early refusal of unsupported FSKit target combinations, stale copied-extension removal for no-FSKit output, and signing/verification. `desktop/src-tauri/tauri.conf.json` lowers only the shared app minimum to 14.0. `README.md`, `docs/CAPACITY_REPORTING.md`, and this record document build/runtime proof and remaining limits. No native transport, driver, filesystem or UI-interaction code changed.

Validation passed: `env TMPDIR=/private/tmp make check` (all 18 lanes); Intel cross-build with `--target x86_64-apple-darwin --without-fskit --output dist/home-intel`; native ARM64 packaging with FSKit using `--output dist/arm64-validation`; Python compilation, CLI help, early Intel-with-FSKit refusal and whitespace checks. The official Intel Rust standard library was checksum-verified and installed only into the ignored local toolchain prefix. Python's initial download failed certificate validation; macOS curl succeeded with TLS verification enabled. Both app builds use an SDK-26.2 linker and shared app minimum 14.0; FSKit's own minimum remains 26.0. The separate ARM64 validation output preserves the prior default preview.

Intel archive SHA-256: `521efaaabecc9332320801031117c294be6ce57cacab7653e825f9bd2ba6db9b`. The private home copy matched before extraction. Strict deep signature verification passed locally and on the home Mac. The Intel app is x86_64 with minimum macOS 14.0 and contains no `.appex` or `.dext`. Its `--scan-json` exited 0 and returned two `Drobo 5D - TB` endpoints, each Thunderbolt and `driverReady: true`. Launch Services opened that exact isolated app; subsequent process/window metadata confirmed finished launch, active state and one onscreen 1120 × 800 window. Screen contents were not inspected, so rendered device cards are not claimed. The app remains open for the user.

The private trial lives under `~/Applications/OpenDroboTrials/20261105-521efaaabecc/OpenDrobo.app`. No existing app was replaced. No capacity/user-client, firmware or array command was sent; no driver/extension activation, mount, disk-pack write, original-service restart, host-security change, public binary upload, tag or release occurred. Capacity units and Finder behavior remain unverified. The original driver still supplies the discovered management endpoints.

Root controlled remote access, compatibility decisions, toolchain/build integration, validation and private deployment. Native OpenAI `gpt_worker` task `intel_preview_builder` produced the two bounded build/config edits; root preserved ARM64 naming and stripped stale no-FSKit extension output. Fresh OpenAI `gpt_reviewer` task `intel_preview_review` returned source PASS before the copy/launch, then **PASS, no actionable P1/P2/P3**, for the final five-path source/docs slice and recorded build/runtime evidence. Both role TOMLs configure `gpt-5.6-terra`; runtime model IDs are not exposed. No GLM, Spark or retired-model lane was used.

Ignored receipts under `artifacts/reports/home-mac/` retain source hashes, both build logs, full-check results, selected preflight observations, archive transfer/checksum, signature/scan output, launch and window metadata. An initial JXA window-observer conversion failed after successful app launch; a corrected observer confirmed process/window state. That observer failure was not an application failure. Raw vendor assets, toolchains, packages and private host receipts remain outside public source. Source publication uses the standing authorization; this record's containing commit identifies the compatibility change. No unrelated source dirt was present.

## Home Mac capacity read and Finder comparison

The user's next request authorizes bounded read-only capacity queries on the home Mac and comparison with current OS volume totals/usage. The existing single-volume and byte-unit reporting gates remain in place. Start with the installed preview's capture path and mounted-volume metadata, then diagnose any refusal using exact evidence. Any new pool diagnostic must remain separate from per-volume/Finder reporting and accept only fixed data-in requests; no arbitrary CDB, mapping change, reset, firmware command, disk write, driver activation or filesystem mount is included. No recursive file scan or unrelated host-service access is required. Record raw pool values separately from OS filesystem usage and unverified units before making a capacity claim.

## Next safe slice

The user's expanded request adds local read-only MCP access, an application console, sharing inspection and broader storage/filesystem capability reporting. Implement a bounded stdio MCP service with explicit tools, a sanitized bounded console shared by application/MCP processes, OS logical-volume metadata and SMB/NFS inspection. Existing protected-capacity and driver-enable gates remain unchanged. Other RAID devices may be visible as host-mounted storage; vendor-specific management and Linux Thunderbolt/EXT access are not represented as implemented. The user chose native OS accounts/permissions. The Sharing page therefore inspects current exports and opens native File Sharing settings; OpenDrobo does not enable shares or edit ACLs. No network MCP listener, arbitrary shell/file/CDB tool, new mount, filesystem installation or array mutation is part of this implementation.

Add stronger SDK-service backend mocks for drain/source cancellation and partial lifecycle; trace remaining completion/error paths and hardware-wide quiescence without treating PCI Close/reset as proof; complete SCSI sense/mode-page and reset/power policies. Define entitlement/provisioning and a named spare-unit activation plan only after those gates are met. Connect an independent management service to the application with verified device/LUN/unit binding. Capture true protected capacity and compare FSKit `statfs`/Foundation/Finder on the same read-only view. A smaller READ CAPACITY is not a safe way to change existing HFS+ statistics. BeyondRAID member locations/generations and real-image reconstruction remain separate research.

## Continuation

```text
Repo: OpenDrobo; start with pwd, git branch --show-current, git rev-parse HEAD, git status --short --branch.
Read: README.md -> docs/WORK_RECORD.md latest packet and final evidence -> docs/DEVELOPER_GUIDE.md -> docs/MANAGEMENT_FEATURES.md -> docs/FIRMWARE_CAPABILITIES.md -> task-specific research notes.
Truth: The shared app targets macOS 14+. The final 96bb private Intel GUI was launched and visually verified on November 6: Status automatically opens the sole 5D in System Information, and five bay indicators render green. One 5D Thunderbolt pool has two HFS+ LUNs, five HDDs and an mSATA cache; protected capacity remains separate from the logical ceiling. The prior 22 checks and both Mac packages passed. The original driver/service remains required; FSKit is inactive and Finder is unchanged.
Scope: Minimum-window and accessibility verification, plus diagnosis of remote pointer activation, remain open. Subsequent work covers bounded telemetry schema recovery, local-service identity attestation and named spare-unit validation. Host RAID across two enclosures remains an assessment; it is not supported or provisioned.
Do not: expand home access beyond read-only OpenDrobo trial; alter unrelated services/ACLs/exports, activate drivers/extensions, mount, flash, format, erase, mutate arrays, execute light/power commands on the home data pack, or publish vendor assets/private receipts.
Validate: make check after relevant source changes; exact target package/signature and read-only home smoke; fresh source/UI review. Hardware activation and host RAID require separately identified spare hardware and an accepted write plan.
Ledger: docs/WORK_RECORD.md; ignored artifacts/reports/home-capacity/home-reference/; https://github.com/hypler-dev/OpenDrobo.
```

## Management, events and product documentation scope

The user's follow-up requests authorize device-event history, unit and drive activity states, brand/model identification, and a complete management-feature layout alongside MCP, Console, volumes, SSD/cache and native sharing settings. Implemented controls must use exact recovered command identifiers and payloads, fresh opaque target binding and an explicit allowlist. Power operations require typed confirmation, no detected mounted Drobo filesystems, and the original service's non-forced safety/unmount path. This source work does not authorize testing power operations against the home data pack. Reset, format, firmware, rebuild and arbitrary command execution remain outside executable scope.

The README becomes a product overview; developer and protocol details belong in linked technical documentation. Storage support must distinguish existing host mounts, device/pool management, and member reconstruction. Linux/Windows adapters, native EXT access, other RAID managers and BeyondRAID reconstruction are not delivered by a shared UI or metadata-only visibility. macOS accounts, folder permissions and ACLs remain authoritative; the Sharing page inspects configuration and opens native settings without enabling exports or editing permissions.

## Drobo-only visual capacity and recorded history

The user's latest steering removes non-Drobo local volumes from the app, requests clearer unit/drive graphics, a usable-capacity pie chart, closed-by-default advanced logical geometry, and capacity history charts from real observations or usable application logs. Device and UI runtime testing now targets the home Mac. Local synthetic UI testing has stopped; the temporary subscription fixture was closed and never accepted a command-channel request. Offline source/unit tests remain distinct from hardware/UI evidence.

Implement Drobo-only mounted-volume and share scope using verified native provider bindings. Capacity history may store only opaque pool references, bounded model labels, timestamps and aggregate byte counters in private local state. It must not invent earlier readings, treat missing log data as zero, reinterpret an OS logical ceiling as usable capacity, or feed historical samples into Finder's live-statistics path. Checked original-log locations on the home Mac contained no usable capacity time series; this bounded inspection does not prove that no other vendor logs exist.

## Management and statistics checkpoint b50f377

This packet starts from `8b3fc8a964b6783320bbf6b6d7b63d388bd91204` on `main`. It combines the original-service observer, unit/LUN deduplication, Drobo-only volumes/sharing metadata, bounded history/events/Console, optional read-only MCP, fixed GUI control availability, original-style enclosure/drive graphics, model-focused navigation/settings and product/developer documentation. The final navigation amendment removes the stretched Status dropdown and unrelated NAS configuration forms. Current component choices are System Information and Drive Information. All 14 pages are visible in Device, Activity, and Host & application groups. Only the exact selected 5D profile presents Dim Lights; unknown models receive an unavailable-profile state. Original name/redundancy/spindown/NAS setting contracts remain unimplemented.

Validation and packages:

- `make check`: **20/20 lanes PASS**, including **41 desktop backend**, **9 statistics**, **40 portable core**, **7 probe** and **29 Python** tests. Frontend typecheck and production build, Rust format checks, SDK/native policy/sanitizer suites, FSKit typecheck/direct checks and historical Swift reference checks also pass.
- Intel: `python3 scripts/build_desktop.py --target x86_64-apple-darwin --without-fskit --output dist/home-reference-intel` — PASS. The macOS-14 preview contains no `.appex` or `.dext`; its strict signature and x86_64 architecture were verified on the home host.
- Apple silicon: `python3 scripts/build_desktop.py --output dist/home-reference-arm64` — PASS. Its macOS-26 FSKit module remains packaged but inactive. No Apple-silicon Drobo hardware trial occurred.
- Runtime source hashes stayed unchanged throughout final checks and both package builds. Subsequent edits are documentation/ignored receipts only.
- Intel archive SHA-256: `53db3d58f8ac26100e42311c495adb6cbd2c28469e45074e3af5cc73d2110f7b`.
- ARM64 archive SHA-256: `c846a90817d7da0ba59232775da597bbce92bde151165569645a82e66867c4b9`.

Home-Mac evidence is a private Intel macOS 14.7.7 trial, not a public binary release. Final preview: `/Users/jeff/Applications/OpenDroboTrials/20261105-53db3d58f8ac/OpenDrobo.app`. Read-only checks report one physical enclosure, one protected 5D Thunderbolt pool, two mounted Journaled HFS+ LUNs and six slots: five HDDs plus mSATA SSD. Protected capacity is approximately 11.75 TB, used 3.75 TB and free 7.99 TB; OS filesystem-used accounting is approximately 3.76 TB and remains separate. The final CLI receipt contains 30 real retained observations. No fixed sample count or date range is promised; new readings are coalesced by the retention policy.

The final bundle passes all **10 SDK MCP tool calls**, with matched JSON text/structured output, Drobo-only mounted volumes, one shared pool, device/drive display metadata, retained capacity history and mounted-volume power guards. Restart, shutdown and standby remain disabled while the Drobo volumes are mounted. The original DDService process/path remained unchanged across checks. No agent light, brightness, power, firmware, disk-pack or array command was sent. The agent did not change host sharing services, exports or permissions.

The prior `90d1eb55905d` private preview provided direct home-screen observations of the protected-capacity pie, working Drive 3/cache selection, firmware/model data, SSD 96% life, six statistics cards, signed-change/utilization charts, real observation gaps and installed-drive capacities. Its Statistics view showed 27 real readings over 4 hr 30 min; those values describe that observation, not a permanent fixture. The final `53db3d58f8ac` preview was launched from its verified Finder directory and visibly shows all 14 navigation pages at 1120 × 800. At that checkpoint, Status/Settings interaction verification remained pending because Screen Sharing's Control Screen mode was switched off; the subsequent follow-up below records the resumed test. No complete VoiceOver, performance, minimum-window or runtime accessibility-preference audit is claimed.

Fresh independent source reviews `statistics_ui_review` and `native_review` returned PASS for the final navigation/settings/statistics and documentation deltas. Statistics parser and robustness reviews also passed after invalid-type/oversized-value guards were added. Earlier source reviews covered native service/control binding, private-file boundaries, MCP framing and optional-artwork packaging; final UI edits did not change those backends. Root is the integration/build/host-test producer; `statistics_engine` used `gpt_worker`, statistics math reviewers used `gpt_reviewer`, UI review used `gpt_ui_review`, and documentation review used `gpt_reviewer`. These native roles are OpenAI, with configured `gpt-5.6-terra` pins. Earlier bounded reader/documentation roles use configured `gpt-5.6-luna`. Runtime model IDs are not exposed, so no observed-model match or token/cost saving is claimed. No GLM, Spark or retired local-model lane was used.

The unresolved low-severity, high-confidence local-service authenticity finding and failed managed-report publication lane remain as recorded at the top. The unauthenticated loopback source is explicitly marked; it has not been fixed. Original firmware/BeyondRAID are unchanged. The original driver remains required; DriverKit hardware gates and byte-unit confirmation remain closed. FSKit is inactive, no new mount was made, and Finder's logical capacity is unchanged. Two-enclosure host RAID and capacity-aware block presentation remain untested research options, not support claims.

Ignored receipts: `artifacts/reports/home-capacity/home-reference/validation.json`, `home-runtime.json`, `mcp-home-smoke.json`, `home-ui-observations-before-navigation.json`, associated logs and `artifacts/reports/desktop-checks/results.json`. Vendor binaries, firmware, extracted artwork, private host captures, packages and toolchains remain excluded from Git. No unrelated working-tree dirt was present.

The containing source commit identifies this checkpoint; exact public-main equality is verified during the authorized push closeout. No tag, public app archive, notarization, installer release, driver installation or extension activation is authorized by this source publication. Next work is bounded telemetry-schema recovery and authenticated service identity; hardware/host-RAID experiments require a named spare-unit plan.

Exact source paths in this packet (35):

```text
README.md
desktop/src-tauri/Cargo.lock
desktop/src-tauri/Cargo.toml
desktop/src-tauri/src/capacity_history.rs
desktop/src-tauri/src/controls.rs
desktop/src-tauri/src/device_events.rs
desktop/src-tauri/src/diagnostics.rs
desktop/src-tauri/src/main.rs
desktop/src-tauri/src/mcp.rs
desktop/src-tauri/src/service.rs
desktop/src-tauri/src/system.rs
desktop/src/main.ts
desktop/src/statistics.ts
desktop/src/styles.css
desktop/tests/statistics.test.mjs
docs/CAPABILITIES_AND_BACKLOG.md
docs/CAPACITY_REPORTING.md
docs/DEVELOPER_GUIDE.md
docs/FIRMWARE_CAPABILITIES.md
docs/MANAGEMENT_FEATURES.md
docs/MCP.md
docs/STORAGE_SUPPORT.md
docs/WORK_RECORD.md
platform/macos/include/OpenDroboTransport.h
platform/macos/transport/CapacityWire.c
platform/macos/transport/CapacityWire.h
platform/macos/transport/Checks.c
platform/macos/transport/OpenDroboTransport.c
platform/macos/transport/PoolDiagnostic.h
platform/macos/transport/PoolDiagnosticProbe.c
research/MACOS_MANAGEMENT_ABI.md
scripts/build_desktop.py
scripts/build_pool_probe.py
scripts/check_desktop.py
tests/test_desktop_artwork.py
```

## Status selection and drive-indicator follow-up

Accepted source base: `b50f377536cfd31cbe14756337f742e9852d6de5`, `main`. Status automatically displays the sole observed pool. Multiple pools remain an explicit choice through enclosure icons; selecting one opens System Information. Previously selected surviving identities remain stable. Unit labels expand opaque suffixes to resolve collisions, including identical models. Automatic display changes never grant a control target; changing/disappearing units clear control authority and pending confirmation. Settings explains selecting a matching target in Tools when needed.

The 5D front-bay artwork now renders reported green, amber, solid red, alternating green/amber and flashing red indicators. The decoder follows the original named SlotStatus constants and low-nibble rendering mask, not physical-capacity presence or `mDiskState`. Unknown selectors, malformed/overflow values and stale observations stay gray. Titles, ARIA labels, detail text and a legend carry color-independent descriptions. Flashing is low frequency and suppressed under Reduce Motion. This is a recovered indicator selector, not a complete health decoder or a command to the physical LEDs. [Management ABI](../research/MACOS_MANAGEMENT_ABI.md#dashboard-per-slot-led-rendering) records the pinned binary hash, enum addresses and complete XML/model/presenter/render path.

Validation:

- `make check`: **22/22 lanes PASS**, including six Status-selection/identity-label tests, four LED-decoder tests and the unchanged nine statistics/41 backend/40 core/seven probe/29 Python test suites.
- Intel and ARM64 preview package builds PASS; all 112 runtime source hashes match the validated package inputs.
- Final Intel archive SHA-256: `96bb203411c68c6980695b3ba7b8ca997e84860863565936e52c3108a61bdbb5`.
- Final ARM64 archive SHA-256: `dfce2e57bf6454d5c113a5935e294f49f0141934ba56b5dda8421ec0ac3c91c9`.
- Final private home bundle: `/Users/jeff/Applications/OpenDroboTrials/20261105-96bb203411c6/OpenDrobo.app`. Strict signature/Intel architecture/no-extension checks PASS. All eight bounded read-only CLI modes and all ten SDK MCP tools PASS against this bundle. One physical enclosure, one pool, two HFS+ volumes and six slots remain distinct; the final CLI receipt retained 44 genuine observations.
- The original DDService PID/path remained unchanged. No agent light/brightness/power, firmware, array, driver activation, filesystem mount or disk-pack write occurred.

On November 6, restored computer-use launched the final `96bb203411c6` preview through Finder in its verified trial directory. Startup displayed All Drobos, one 5D card and an unselected Dashboard header. Keyboard navigation to the sidebar Status button, followed by Space, automatically opened the sole unit's System Information and changed the header to Drobo 5D without selecting the card or a control target. All five front-bay indicators visibly rendered green. System Information showed the Drobo brand, 5D model, firmware `4.2.3-7.199.116068` and active presence in a fresh original-service snapshot; the cache label showed 240.06 GB. All 14 navigation entries remained visible at the default 1120 × 800 window, with technical details closed. Read-only process metadata confirmed final-preview PID `58148` in the exact trial path and original DDService PID `375` unchanged. No alternative GUI automation was used. These observations validate rendered reported indicators, not independent inspection of physical LEDs.

Remote pointer activation was unreliable: clicks hovered without consistently activating content; Finder File → Open and keyboard Tab/Space worked. The cause is not established. Screen Sharing control was on at the start and end, with one off/on refresh attempt. Minimum-window sizing, full VoiceOver and runtime accessibility-preference checks remain open. The preceding `8a4ff11431e7` preview supplied the 5D-only Dim Lights settings observation, with current brightness not read and Apply disabled until a choice; those settings were not re-exercised in the final preview. Multi-enclosure rendering and real fault/protection/stale hardware transitions were not tested; prior offline selection/decoder cases and independent source review cover those contracts. This verification changed no application source and did not repeat the prior 22-lane suite or package builds.

Fresh `statistics_ui_review` (`gpt_ui_review`, OpenAI, configured `gpt-5.6-terra`) returned PASS after collision-safe labels were added. Fresh `native_review` (`gpt_reviewer`, same configured provider/model) passed the LED decoder, UI labels, stale/unknown behavior, animation and accessibility rules. `led_semantics` (`gpt_reader`, OpenAI, configured `gpt-5.6-luna`) independently verified the named constants and the complete XML `mStatus` to rendered slot-buffer binding. Root owns integration, package validation and host receipts. Runtime model IDs are not exposed; these are configuration pins. No GLM/Spark or retired model lane was used.

Receipts remain ignored under `artifacts/reports/home-capacity/home-status-selection/`: `validation.json`, `home-runtime.json`, `mcp-home-smoke.json`, `home-ui-observations.json`, check/build logs and bounded original-binary disassembly in `led-recovery/`. The earlier pre-LED runtime/SDK receipts have separate filenames. The loopback authenticity finding and managed-report storage failure remain unresolved; this UI follow-up changes neither boundary. Original firmware, DriverKit hardware gates, FSKit activation and Finder geometry remain unchanged. No public binary release or tag was created.

This containing commit identifies the follow-up source checkpoint; public-main equality is checked during authorized publication. No unrelated dirt was present. Exact source paths (11):

```text
desktop/src/drive_indicator.ts
desktop/src/main.ts
desktop/src/status_selection.ts
desktop/src/styles.css
desktop/tests/drive_indicator.test.mjs
desktop/tests/status_selection.test.mjs
docs/DEVELOPER_GUIDE.md
docs/MANAGEMENT_FEATURES.md
docs/WORK_RECORD.md
research/MACOS_MANAGEMENT_ABI.md
scripts/check_desktop.py
```
