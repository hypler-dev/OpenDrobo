# OpenDrobo work record

Last updated: November 6, 2026. This is the canonical status and acceptance record.

## Current documentation amendment: multi-section screenshot gallery

The user requested screenshots of different application sections in the GitHub README and an updated product summary. From clean `ad7599d21ce7957ce26fa2a5ffdbd1380e10b532`, this is a documentation and captured-image slice only. The user clarified that captures must be taken on this Mac from the visible Screen Sharing view. Navigate the verified home-Mac application through Computer Use, capture the specific local Screen Sharing window and frame only the app into repository screenshot assets. The initially opened remote Screenshot utility was canceled without taking a capture. Choose representative views without exposed private host identifiers, paths or raw technical IDs. Images must be genuine app captures, with no generated UI or inferred hardware state. Root owns captures, documentation, asset inspection and publication; fresh read-only review checks exact images, captions, links and unchanged compatibility boundaries. No application change, hardware control, service restart, driver activation, disk operation, signing or binary release is included.

## Previous amendment: longer splash and compact light legend

From clean `989bff1b316545f8ca1ff865a59de928b0a59dad`, the user requested a longer splash so its artwork can be seen, then direct home-Mac verification. They also replaced the large indicator icons/cards with a compact legend of slim color bars and meaning labels that match the enclosure lights. Acceptance is a three-second automatic splash interval after paint, immediate explicit skip, unchanged independent error recovery, unchanged status decoding, and visible steady/flashing distinction. The user also requested an RGB-blended capacity arc with a transparent unused track, simple Storage capacity wording and enclosure names without the - TB suffix. These are presentation changes: source transport metadata, stable identity, stale handling and separate Drobo/macOS accounting remain intact. Root owns the bounded frontend/doc edits; fresh read-only UI review, focused frontend checks, an Intel candidate build and Computer Use on the home Mac verify this amendment. Hardware controls, service restart, driver activation, disk operations and public binary release remain excluded. Final results follow below.

## Active follow-up: startup branding, tray navigation and indicator clarity

The user approved the newly rendered artwork and requested a matching startup screen, application/tray icons and logical tray commands. They also requested verification of the drive-light meanings and more distinct presentation of Ready, Low space, Add capacity, Protecting data, Drive fault and Status unavailable. Acceptance preserves the recovered per-bay selector and physical color conventions, distinguishes steady red capacity alerts from flashing red faults, and provides persistent non-color cues, including under Reduce Motion. The startup screen must not depend on hardware discovery or imply successful connectivity. Tray commands provide navigation, read-only refresh, visibility and quit only; entering Tools must not establish device-control authority.

Source work starts from `d70cd37ca82bd5deded27312b912981bbef14581`. Root owns integration, indicator changes, notice refresh, packaging and host verification; bounded workers own startup/frontend routing and native tray files. Fresh independent review is required. Read-only home-Mac verification remains authorized, with GUI interaction through computer use only. No driver activation, DDService restart, physical LED/control operation, pack mutation, signing-account enrollment, binary release or tag is included. Manual Gatekeeper exceptions for trusted local/tester builds are documented separately from the unchanged notarized-public-release policy.

## Latest accepted follow-up: remaining v1.0 gates and OpenDrobo artwork

The user authorized continuing all presently actionable v1.0 blockers, researching license/rights questions, and documenting what they must provide for signing and hardware testing. They confirmed no Apple Developer membership yet and a spare enclosure of the same 5D model. Newly rendered product art must preserve recognizable 5D hardware, the approved three-quarter angle, green bay lights and blue lower-row lights; decorative lights do not become active-device telemetry. Adding “Open” or changing an angle is not evidence of rights clearance.

Current source scope: exact target-selected dependency/notice evidence and schema validation; newly rendered artwork/icons and safe UI integration; bounded legacy-service ownership hardening only where verified host APIs can support it; signing/operator and hardware acceptance documentation. Root retains security design, integration, validators and publication. Bounded workers own disjoint dependency and UI files; fresh independent reviews follow. No developer account, paid enrollment, credentials, driver activation, DDService restart, production-pack control or destructive hardware test is authorized by this follow-up. The working Intel app continues to depend on the original driver; the compiled replacement remains inactive research.

The baseline is `170d8ad1273ec6c4f1b8fb26165ba9501d087d5e` on clean `main`. Verified findings, changed paths, validation and remaining user-dependent gates are recorded in the final follow-up section below. Source commit/public-main equality is checked at publication closeout.

## Previous checkpoint: v1.0 quality audit and release preparation

The user requested a full application quality audit after the Usage/Volumes/indicator and offline spare-unit work, removal of temporary/development-facing text, and preparation for v1.0. Source remediation and independent reviews are complete. The 24-lane suite passes, optimized Intel/ARM64 local packages build, and read-only home-Mac observations cover the final interface. Public v1.0 release remains blocked by signing/notarization, dependency clearance, additional hardware and accessibility QA, and the unresolved legacy loopback-source trust decision. Version remains `0.2.0`; no public binary/tag was created. See [V1_READINESS.md](V1_READINESS.md) for findings, acceptance evidence and open gates, and the final audit record below for exact paths and receipts.

## Accepted Dashboard, statistics and MCP boundaries

The user requested a closer reconstruction of the original Dashboard's screen organization, component selection, enclosure graphics, capacity display, and settings locations, with new functionality kept accessible alongside it. The original local XAML resources, English labels, and artwork inventory provide layout evidence; they do not establish full functional parity. This packet replaces the generic status-card layout with a component inspector and selectable drive bays, restores Capacity/Usage and original Tools/General settings placement, and places additional diagnostics under Advanced. Unknown values and unsupported operations must remain explicitly unavailable. Runtime UI and device checks target the home Mac; local checks remain offline software validation.

The MCP assessment covers the current desktop source and its host/service boundaries. The independent baseline reported no candidate; a separate focused reviewer found an unauthenticated loopback-source boundary. Root validated that local port 4502 can supply structurally valid observations without peer authentication, and accepted one **low-severity, high-confidence CWE-345** finding: a local process that obtains the port while DDService is absent can forge displayed metadata and poison bounded capacity history. This does not establish remote exposure, MCP control execution, arbitrary file access, or a disk-pack write. `sourceTrust: unverified-loopback` records that limitation; it does not remediate peer authentication. The [MCP product guide](MCP.md) documents purpose, intentional data disclosure, and operational limits.

The Codex Security managed scan (`7bef81a8-678a-43f7-a184-a3773956952c`) could not save canonical artifacts. Its tool repeatedly returned `Scan output parent must not be group- or world-writable without the sticky bit.` The authoritative output path has a `/Volumes/Storage` ancestor with mode `0775`; all scan descendants were private. No existing-scan relocation tool was available. Root did not change volume permissions, weaken plugin checks, or write replacement canonical artifacts. The scan was marked failed for this confirmed storage blocker; no sealed report or successful scan completion is claimed. The original candidate, generated model, and source validation were saved only through the tool's temporary artifact storage and may be cleaned by the OS. This is a source-backed assessment with an incomplete managed publication lane, not a clean security certification. No runtime exploit was performed. Token usage was not returned.

The preceding Usage/Volumes checkpoint passed all 24 software-check lanes, including 42 desktop backend tests, nine statistics tests, six Status-selection tests, six drive-indicator tests, five storage-presentation tests, 40 portable core tests, seven probe tests and the Python suites. The native icon lane checks PNG rendering, fixed dimensions, output bounds and synthetic registry/path binding. Intel and ARM64 preview packages pass packaging checks. Ten SDK MCP tool calls pass against the final private Intel preview on the home Mac. The current runtime observations and remaining UI-input verification are recorded below. The original driver/service, Finder geometry, driver activation, filesystem mounts, live light/power tests and disk-pack mutation remain outside this packet.

The user subsequently accepted optional MCP and requested more elaborate statistics plus a firmware-grounded feature assessment and an explanation of macOS software RAID. Extend Statistics with actual time-range selection, protected/free/OS-used trends, utilization and signed changes, observation gaps, reported drive capacities and SSD life, and session-scoped activity. Byte arithmetic must remain exact, missing observations must not become zero or an outage, and growth rates must be described as observed averages rather than forecasts. Additional telemetry command names are research leads until their payloads, units and model capability are recovered. Neither original firmware nor the BeyondRAID on-disk format has been modified. No AppleRAID conversion, member-disk passthrough, formatting or array migration is authorized by this assessment. The follow-up question about connecting two matching Drobos is an architecture assessment: Thunderbolt chaining is connectivity, while a potential host-managed mirror or stripe would retain two separate onboard BeyondRAID pools. OpenDrobo has not implemented or validated this arrangement; virtual-member ancestry, mount guards, thin-provisioned capacity and failure recovery remain open. See [firmware capabilities](FIRMWARE_CAPABILITIES.md).

The user then identified the stretched Status selector in the current home-Mac view and requested visible options, better organization and device-specific settings. Replace that dropdown with visible System/Drive choices, remove the collapsed Advanced navigation group, and show only verified settings for the explicitly selected device. Remove irrelevant NAS forms from the 5D flow and retain native host Sharing as a separate page. Raw logical geometry remains closed technical detail, as previously requested.

The next accepted Status amendment automatically opens the sole observed Drobo; with multiple units it presents enclosure icons and opens System Information for the selected identity. Selection is read-only: automatic display changes must not grant a control target, and device disappearance clears any prior control authority. Multiple identical models need distinct opaque unit labels. The user authorized briefly resuming Screen Sharing control to verify the interface. Device actions remain outside the home test.

The user also requested original-style green/amber/red enclosure indicators. Recover the original per-slot LED selector before applying colors; keep unknown or stale observations gray, expose color-independent text, and leave physical lights unchanged. This amendment does not decode every health bit or authorize device-control execution.

## Capacity Usage and mounted-volume correction

The user reported incorrect information under Capacity → Usage, specified the device label `Drobo 5D - TB` without an invented unit number, and requested a tidy Volumes list using native macOS icons and clear drive relationships. This is the accepted follow-up scope. Read-only home observations on November 6 show approximately 20.00 TB installed raw data capacity, 11.75 TB protected usable capacity, 3.75 TB protected allocation, 7.99 TB protected free space and 3.76 TB OS filesystem usage. Usage previously emphasized raw installed capacity and incorrectly described the entire used/free/protection breakdown as unavailable, despite available protected used/free values. Physical protection overhead and cache occupancy are still not decoded.

The correction must distinguish these accounting domains, preserve exact byte arithmetic, display the reviewed 5D Thunderbolt label, group mounted Drobo volumes only by native enclosure ancestry, and obtain bounded native icons at runtime without distributing Apple assets. Equal model or volume names do not establish a shared enclosure; unknown grouping remains explicit. BeyondRAID volumes cannot be assigned to individual bays from current evidence. Native host permissions, original service ownership, Finder geometry, driver/FSKit activation and all disk-pack/control gates remain unchanged. The user also requested matching bay lights on All Drobos and explanations of indicator meaning and action rather than color names. Inventory and Status must share the recovered per-slot selector, preserve bay positions and unknown/stale states, distinguish capacity warnings from drive faults, and honor Reduce Motion.

Validation requires focused accounting/grouping tests, the repository check suite, Intel/ARM64 packages, fresh independent review and read-only home-Mac UI checks.

## Spare-unit lab preparation

The user will provide a working spare enclosure and disposable disks and requested scaffolding before hardware arrives. The accepted scope is an offline lab kit: manifest templates, no-hardware preparation plans, protected/home unit and disk exclusions, bounded copied-baseline receipts, an ordered manual test sequence and developer/operator documentation. Missing identities must remain pending, and plan generation must never establish destructive authority or report hardware ready. This scope adds no formatter, reset dispatcher, filesystem mount, replacement-driver activation, member-disk reconstruction or write-back path. The home data pack remains excluded. See [lab preparation](SPARE_UNIT_LAB.md).

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
Truth: The shared app targets macOS 14+. Final private Intel preview 9f4a is running on the home Mac with Drobo 5D - TB, five aligned inventory bay indicators and unchanged original service. Usage now separates protected allocation/free/usable total from OS usage and raw disks; Volumes uses native icons and hardware ancestry. All 24 check lanes and both Mac packages passed; original driver/service remains required, FSKit inactive, Finder unchanged.
Scope: The offline spare-unit lab scaffold is prepared. Next is the requested full application quality and v1.0 readiness audit, including user-facing development-text cleanup; future hardware tests require the promised working spare enclosure and disposable disks. Minimum-window/accessibility, multiple enclosures and live fault transitions remain unverified. Broader provider adapters and BeyondRAID reconstruction remain future work.
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

## Capacity, volume presentation and inventory indicator validation

Source base: `d5ae42d96e516a27c02ddd758cb70e2147a08a62`, `main`. The Usage defect was presentation of raw installed capacity as its primary number while labeling available protected used/free accounting unavailable. Usage now displays protected used/free/usable totals together, with separate OS filesystem usage, raw installed data-drive capacity and SSD life. Invalid or missing slot measurements remain unavailable rather than becoming zero. The reviewed 5D Thunderbolt display label is `Drobo 5D - TB`; the model remains separately reported in System Information, and no invented single-unit number is appended.

Volumes now displays compact rows with runtime-native macOS icons, filesystem, mount path, access and decimal filesystem-used bytes. It groups only by the native physical enclosure key; distinct per-LUN driver IDs remain separate fields. Matching names never establish hardware identity, and unknown ancestry remains explicit. Logical totals/available space remain closed technical details with thin-provisioning guidance. No per-volume assignment to a BeyondRAID member bay is inferred.

All Drobos and Status share the recovered per-bay LED selector. Inventory shows five fixed bay positions with text for affected bays; missing/stale observations cannot become ready. Labels distinguish low capacity, critical capacity, drive fault and protection activity and provide guidance rather than repeating colors. The original guide and recovered resource evidence are recorded in the management ABI note. Real failures and physical lights were not manipulated.

Validation and home evidence:

- `make check`: 24/24 lanes passed after the final thumbnail alignment change; the new lab tests receive their final result in the lab record below. The desktop backend has 42 tests. Frontend type/build checks and Intel/ARM64 packaging passed after alignment, with all 116 desktop/package input hashes unchanged during verification.
- Intel archive SHA-256: `9f4a129360fae94db7621b00c886df1111ff27834beccfcb05972f001c817e06`.
- ARM64 archive SHA-256: `0e8d76ea615f3abc27e24332edefadbedd643def906ef180d3802ecf23470f1e`.
- Final private home preview: `/Users/jeff/Applications/OpenDroboTrials/20261106-9f4a129360fa/OpenDrobo.app`, strict signature/Intel/no-extension checks passed. All eight bounded read-only CLI modes and ten SDK MCP tools passed. Both mounted volumes report the same physical enclosure key, distinct driver endpoint IDs, the expected product name and 5,740 base64 characters per native icon.
- Computer-use visibly confirmed five aligned green inventory bay lights, the exact device name and ready-bay text in final `9f4a`. The preceding `b094` GUI confirmed Usage at 3.75 TB protected used, 7.99 TB free and 11.75 TB usable, with OS-used 3.76 TB and raw installed 20.00 TB kept separate. It also showed two compact HFS+ rows with native Mac icons and shared enclosure grouping. Only thumbnail alignment CSS changed between these builds; Usage and volume code did not. Final process readback confirmed preview PID `75038` at the exact final path and original DDService PID `375` unchanged.
- Remote pointer activation remained unreliable, and navigation also changed outside automation. Claims cover visibly observed states rather than attributing every interaction to automation. Minimum-window layout, full VoiceOver, multiple enclosures and live fault/stale hardware transitions remain open. Offline tests cover bay fault position, missing/stale suppression and accounting/grouping edge cases.

Fresh `usage_volumes_review` (`gpt_ui_review`, OpenAI) passed after fixes for control-page naming consistency, unknown slot accounting and local table overflow. It reviewed source plus controller-supplied rendered observations; it did not operate the home GUI. Fresh `volume_backend_review` (`gpt_reviewer`, OpenAI) passed after exception-safe graphics-state restoration, standalone-probe linkage isolation, premultiplied bitmap rendering and native binding/size tests. The bounded backend implementation used `native_volume_identity` (`gpt_worker`); root produced and integrated the UI and test changes. These roles are configured as `gpt-5.6-terra`; runtime model IDs are not exposed. No GLM, Spark or retired model lane was used.

Receipts remain ignored under `artifacts/reports/home-capacity/usage-volumes/`. Vendor assets, native runtime icon payloads and private host captures are not published. No physical light/power command, driver installation/activation, FSKit mount, firmware operation, sharing/ACL change or disk-pack write occurred. Finder geometry and the original driver dependency remain unchanged. No public binary release or tag was created.

## Spare-unit scaffold validation and next v1.0 audit

The offline lab kit is implemented in `scripts/spare_unit_lab.py`, with operator documentation in `docs/SPARE_UNIT_LAB.md`. `init` creates a pending manifest, and `plan --template` generates a six-case preparation sequence without hardware or invented identities. Real-manifest validation requires strict SHA-256 fingerprint syntax, confirmed target fields and separate protected/home-unit and protected-member exclusions. Supplied evidence envelopes require exact unit/member/target binding, integer timestamps within the freshness window and nonempty copied payloads. Whole-envelope hashes and byte counts preserve bounded operator evidence. This is operator-asserted metadata, not hardware attestation or a provider-protocol certificate.

No plan reports hardware ready or dispatches an operation. Format/reset, capacity profiling, migration, fault replacement and reconstruction remain future manual cases, with destructive authority required separately at execution time. Template artifacts were prepared privately under `artifacts/spare-unit-lab/20261106-preparation/`; they report `PENDING_HARDWARE`, `ready: false` and blocked execution. No spare hardware has been observed.

Fresh `spare_lab_review` (`gpt_reviewer`, OpenAI, configured `gpt-5.6-terra`) returned PASS after fixes for stale/unbound evidence, target metadata binding, protected disk exclusion, strict fingerprint/schema types and empty payload rejection. Root integrated the fixes; `native_volume_identity` (`gpt_worker`, same configured provider/model) produced the bounded scaffold. Runtime model IDs remain unexposed. `make check` passed all 24 lanes, and after the final lab-only review fixes the complete Python suite passed 39 tests, including ten lab cases. No frontend/native package input changed during those final lab fixes. `git diff --check` passed.

The user then requested a full application quality audit, removal of temporary/development-facing product text and preparation for v1.0. That is the next accepted packet. Audit scope covers user-facing flows, state and accounting correctness, error handling, native/service/MCP boundaries, package/release configuration, tests, documentation and claimed platform support. Remove development clutter while retaining useful capability and safety explanations. Audit findings and hardware/release gates must remain explicit; this request does not establish new Linux/Windows transport support, independent BeyondRAID recovery or a successful public release. No public binary/tag publication is part of the completed scaffold checkpoint.

This containing commit identifies the Usage/Volumes/indicator/scaffold checkpoint. Exact changed source and documentation paths:

```text
desktop/src-tauri/build.rs
desktop/src-tauri/src/system.rs
desktop/src/drive_indicator.ts
desktop/src/main.ts
desktop/src/storage_presentation.ts
desktop/src/styles.css
desktop/tests/drive_indicator.test.mjs
desktop/tests/storage_presentation.test.mjs
docs/CAPACITY_REPORTING.md
docs/DEVELOPER_GUIDE.md
docs/MANAGEMENT_FEATURES.md
docs/MCP.md
docs/SPARE_UNIT_LAB.md
docs/STORAGE_SUPPORT.md
docs/WORK_RECORD.md
platform/macos/include/OpenDroboTransport.h
platform/macos/transport/OpenDroboTransport.c
platform/macos/transport/VolumeIcon.m
research/MACOS_MANAGEMENT_ABI.md
scripts/check_desktop.py
scripts/check_volume_icons.py
scripts/spare_unit_lab.py
tests/test_spare_unit_lab.py
```

## v1.0 application quality audit and release preparation

Source baseline: `ee9f9fd68ab68f20372d4e91973fc41e7c0f1cfc`, `main`. The user authorized the full application quality audit, removal of temporary/development-facing product text, release preparation and continued source publication. This section records the reviewed source changes; commit and public-main equality are verified at publication closeout. Public v1.0 remains blocked; version stays `0.2.0` and no binary release or tag was published. [V1_READINESS.md](V1_READINESS.md) records resolved findings and remaining gates; [RELEASING.md](RELEASING.md) supplies concrete packaging/distribution steps.

The interface removes the development banner and dummy preferences, preserves missing/error states, adds named route focus/skip navigation and bounded overflow, and keeps display selection separate from explicit Tools authority. Duplicate-model labels remain distinct across views. Rust service framing now supports the existing 32-unit bound without prematurely truncating snapshots. Native scope/capture/rebind tests and control reply-failure tests exercise refusal paths without hardware retries. Inactive FSKit rejects nonintegral block geometry; inactive SCSI inquiry stops inventing serial numbers, and timer/queue cancellation failure retains ownership for explicit retry. Hardware activation gates remain closed.

Default packages now use optimized release code, generated artwork and no inactive filesystem extension. Debug, local vendor-artwork and research variants are explicitly labeled local. Release tooling checks version parity, native/cross target output paths, architecture and launch floors, with explicit Developer ID/stapler/Gatekeeper gates. Local structural mode cannot establish distribution readiness. Deterministic dependency inventory preserves unknown/conflicting licenses and safe bounded metadata; it does not claim human clearance or exact shipping contents. Rust 1.98.0 and Node 24.22.1 are pinned; the declared Rust 1.85 minimum was not separately validated.

Validation and artifacts:

- `make check`: **24/24 lanes PASS**, including **58 Python, 49 desktop backend, 40 core, seven probe and 28 frontend behavior tests**. Frontend type/build, formatting, native volume icons, wire/native and inactive FSKit checks passed. DriverKit SDK/IIG/link and eight ASAN/UBSan suites passed separately after native fixes. `git diff --check` passed before publication.
- Optimized Intel and ARM64 generated-artwork packages and the private-artwork Intel home package all built successfully. The 105 recorded app/build input hashes were unchanged across final validation. Local structural inspections passed every applicable structural gate; Developer ID, notarization/stapling and Gatekeeper distribution gates remain false. There were zero available Developer ID Application identities in the local keychain. No signing credentials or notarization account were configured or used.
- Generated-artwork Intel archive SHA-256: `42ae4701d8b1aa2ed15d2f9c2e49e962de8aa9bc925fb5edb8c75683bf782c21`.
- Generated-artwork ARM64 archive SHA-256: `48b5bb839b332fba56ab36e4859651b975330e5d102b523403a15fbe7f0bbbe7`.
- Private-artwork Intel archive SHA-256: `8aeb2dc64398bccf9278fa4cb6482c1148d753b780d82cefafb2e622c85695e2`.
- Both Intel candidates passed all eight bounded read-only CLI modes on the home Mac. Final private candidate passed all ten SDK MCP tools; one pool, two HFS+ volumes, six slots and native icon/enclosure binding remained correct. Power operations stayed disabled while volumes were mounted. The final receipt retained 76 actual history observations.
- Computer Use launched the generated-artwork Intel candidate and inspected all 14 pages. Single-unit Status selection, exact `Drobo 5D - TB` label, five reported green bay indicators, separate protected/OS/raw accounting, native volume rows and historical charts were visibly correct. The window was resized to 860 × 640; Capacity/Usage and Volumes stacked and scrolled vertically in the observed states. Visible keyboard focus and skip link were observed. Full keyboard/VoiceOver, accessibility preferences, measured performance, live disconnect/fault transitions and multiple physical enclosures remain unverified. Remote input was occasionally unreliable; observations are not claims of flawless automation or exhaustive flow coverage.
- Final private preview was launched through Finder and left on automatically selected Status with five green bay indicators: `/Users/jeff/Applications/OpenDroboTrials/20261106-8aeb2dc64398/OpenDrobo.app`, PID `99264`. Read-only process metadata confirmed exactly one OpenDrobo GUI process and original DDService PID `375` unchanged. No SSH/AppleScript GUI fallback was used.

Independent reviews returned PASS after source fixes: `v1_interface_audit` (`gpt_ui_review`), `v1_backend_audit` and `v1_native_audit` (`gpt_reviewer`), and `v1_release_audit` (`gpt_reviewer`). Bounded producers were `native_volume_identity`, `v1_backend_fixes` and `v1_release_builder` (`gpt_worker`), with root producing/integrating residual UI, native, release and documentation fixes. Roles use OpenAI and are configured as `gpt-5.6-terra`; runtime model IDs are not exposed. Root is the configured Codex controller; its runtime model identifier is likewise unavailable. No GLM, Spark, retired role or Mac-mini model was used. Reviewers' standalone frontend check encountered the wrong-architecture TypeScript dependency; root's pinned frontend validator passed. Final read-only documentation/evidence review by `v1_release_audit` identified a premature source-publication description; it was corrected to separate reviewed source from commit/public-main verification. The reviewer also requested explicit cross-target standard-library/toolchain provisioning guidance; the release guide now supplies it. Final documentation/evidence review returned PASS with no remaining actionable P1/P2/P3 findings.

Dependency inventory contains 518 locked components, 347 declared licenses and 171 unknown. A complete OSV query of 515 third-party public coordinates matched `glib@0.18.5` (RUSTSEC-2024-0429/GHSA-wrw7-89jp-8q8g) and `proc-macro-error@1.0.4` (RUSTSEC-2024-0370, unmaintained). Offline target-specific trees selected neither for either macOS target in the inspected desktop dependency graph; an all-target tree could not complete from cache. Future affected platform enablement must resolve these matches. License/shipping clearance and full BOM schema validation remain open. The existing low-severity/high-confidence CWE-345 loopback observation-source authenticity finding remains unresolved; disclosure is not remediation. The failed managed security report remains failed; no clean security certificate is claimed.

Receipts are ignored under `artifacts/reports/v1-audit/`, with native details in `artifacts/reports/v1-driverkit-check.log` and `v1-fskit-check.log`. Only source/docs are published. Vendor artwork, firmware/binaries, host identifiers/captures and package bytes remain local. No light/brightness/power command, driver installation/activation, FSKit mount, geometry change, firmware/array operation, sharing/ACL change or disk-pack write occurred. Original driver dependency and Finder reporting remain unchanged. No unrelated dirt was present at startup.

Next accepted action is completion of the explicit release gates: Developer ID/notarization, dependency clearance, read-only Apple-silicon device proof, full accessibility/QA and the separately scoped spare-unit management plan. Replacement-driver activation, Finder geometry and BeyondRAID reconstruction require independent future packets, not a release-label change.

Exact changed source/documentation paths:

```text
.node-version
README.md
desktop/src-tauri/src/controls.rs
desktop/src-tauri/src/main.rs
desktop/src-tauri/src/native.rs
desktop/src-tauri/src/service.rs
desktop/src-tauri/src/updates.rs
desktop/src/main.ts
desktop/src/status_selection.ts
desktop/src/styles.css
desktop/tests/status_selection.test.mjs
desktop/tests/storage_presentation.test.mjs
docs/CAPACITY_REPORTING.md
docs/DEVELOPER_GUIDE.md
docs/MANAGEMENT_FEATURES.md
docs/RELEASING.md
docs/V1_READINESS.md
docs/WORK_RECORD.md
platform/macos/driverkit/AsyncSourceClose.h
platform/macos/driverkit/OpenDroboSCSIController.cpp
platform/macos/driverkit/ScsiResponseChecks.c
platform/macos/driverkit/ScsiResponses.c
platform/macos/driverkit/ScsiResponses.h
platform/macos/driverkit/SourceCloseChecks.cpp
platform/macos/fskit/Checks/main.swift
platform/macos/fskit/OpenDroboFS.swift
rust-toolchain.toml
scripts/build_desktop.py
scripts/check_driverkit.py
scripts/dependency_inventory.py
scripts/release_metadata.py
scripts/release_preflight.py
tests/test_dependency_inventory.py
tests/test_release_metadata.py
tests/test_release_preflight.py
```

Continuation: start in this repository with `git status --short --branch`, `git rev-parse HEAD` and `git log -1 --oneline`; read `docs/WORK_RECORD.md`, `docs/V1_READINESS.md`, `docs/RELEASING.md`, then `docs/SPARE_UNIT_LAB.md`. Identify the current reviewed checkpoint from Git and this audit record; local candidates are ad-hoc, not public releases. Active scope is the remaining recorded v1.0 gates. Keep home production pack, DDService ownership, driver/FSKit activation, Finder geometry and all destructive operations unchanged. Run only validators affected by a new accepted change; use final audit receipts and this ledger for existing evidence.

## v1.0 blocker follow-up: notices, artwork and owner handoffs

Source baseline: `170d8ad1273ec6c4f1b8fb26165ba9501d087d5e`, clean `main`. The user authorized continuing actionable v1.0 gates and research, then requested newly rendered artwork recognizable as real Drobo hardware, retaining the approved three-quarter angle with green bay lights and blue lower lights. They confirmed a separate spare 5D and no Apple Developer membership. This section records reviewed source changes; commit and public-main equality are verified at closeout. Version remains `0.2.0`, and no public binary/tag is published.

The new decorative lineup and application icon are AI-rendered images using recognizable 5D hardware references and OpenDrobo styling. The original redesign draft was replaced after the user's correction. Green/blue lights in those images are decorative only; the background is aria-hidden. The active Status and All Drobos graphics are independently authored code with five bay positions, left handles, decoded right indicators and neutral lower windows. Stale/unknown observations do not become healthy. Normal UI no longer loads extracted vendor artwork. The high-resolution icon master is outside the frontend public directory to avoid embedding an unused duplicate, and unused generated mobile icons were removed. New Mac/Windows icon sizes remain source assets; this does not add Windows device support.

The notice collector now reads exact locked package archive bytes: Cargo SHA-256 against Cargo.lock and npm SHA-512 against package-lock. It checks identities and declaration agreement, bounded regular archive members, path safety, input hashes and fixed CLI inputs. It reproduces both default-feature macOS Cargo graphs through the pinned environment. Attribution-only NOTICE material cannot satisfy license-term coverage, including supplements. Supplements are immutable-commit/SHA-bound to verified archive VCS metadata and cannot replace local terms. No mutable extracted Cargo tree or node_modules content establishes notice provenance.

Actual selected scope is 290 macOS/UI components with zero declaration mismatches, 492 collected notice documents and documentary terms coverage for 277 components. Thirteen entries need manual documentary review: selectors and twelve objc2-family packages. These are not findings of absent license grants. Plist's British-spelled LICENCE is collected correctly. The report retains `noticeReady: false` and `legalReviewRequired: true`. Five selected MPL-2.0 packages require a source-access/notice disposition; see the rights guide. The refreshed all-lock inventory has 518 components, 492 recognized license declarations and 26 collector-unresolved entries. It passes offline validation against saved official CycloneDX 1.6/SPDX/JSF schemas. All-lock discovery and selected target evidence remain separate; no dependency versions changed.

Packaging includes the application's MIT license and supports an explicit checksum-bound third-party bundle. Developer ID packaging stops before build/signing when notices are absent or incomplete. Preflight verifies notice resource hashes, current lock/package bindings, exact target coverage and the application license; missing, partial, changed or wrong-target resources cannot pass distribution. These are documentary gates, not legal approval or proof of exact compiled contents. Current local packages deliberately include the partial report and remain distribution-blocked.

Research and owner requirements are documented in `docs/RIGHTS_AND_DISTRIBUTION.md`, `docs/RELEASING.md` and `docs/HARDWARE_VALIDATION.md`. U.S. copyright duration and transfer rules do not make closure a public-domain grant. The cited USPTO-hosted bankruptcy filing/order supplies an IP-sale ownership lead without establishing the present owner of every Drobo right. Newly rendering hardware, adding “Open” and changing an angle do not establish blanket rights clearance. No rights-holder outreach was sent. Apple enrollment, the publishing identity, Developer ID/keychain setup and notarization remain owner-dependent; DriverKit entitlement/provisioning and activation are separate. The reported spare 5D is an appropriate initial target. The operator still must identify its disposable pack, the Apple-silicon test Mac/macOS and the cable/adapter chain; none of those test bindings has been verified. Other models need their own transport/firmware receipts; the B800fs Ethernet role and B800i administration-only USB distinction are documented from original vendor sources.

Read-only home research confirmed that executable paths and kernel socket-table PID text were visible, while root-owned service descriptor/BSD-info queries returned EPERM. That did not establish a dependable production authentication route. No helper, service restart or weakened OS protection was introduced. The known low-severity/high-confidence CWE-345 status-source finding remains unresolved, and the failed managed-security publication lane remains failed. Optional MCP stays read-only with no MCP network listener. No security certification is claimed.

Validation and exact candidate evidence:

- `make check`: **24/24 lanes PASS**; **77 Python, 49 desktop backend, 40 portable core, seven probe and 28 frontend behavior tests**. The final 77-test Python suite passed again after the last test-only correction. Focused notice tests passed 10/10; focused release/notices tests passed 23/23. Native/DriverKit and inactive FSKit checks passed through the integrated suite. The ARM64 DriverKit executable exists, with hardware disabled and no activation.
- Optimized Intel and ARM64 local packages passed build, signature-integrity and structural inspection. **112 app/package input hashes stayed unchanged** during validation. Expected distribution failures are ad-hoc identity, missing Developer ID/stapling/Gatekeeper proof and the thirteen incomplete documentary entries. No actual Developer ID signing or notarization submission occurred.
- Intel archive SHA-256: `cdc6715ab2323dd0543437064ab85b3704ea6cec2bf47825e8d61f79d6d39fe6`.
- ARM64 archive SHA-256: `1073fe7d454e505acd2d6ac7f50ef23875141e6ee4dde8e296584017e31667c5`.
- The final home candidate is identified by the Intel archive hash above; its exact trial path and process identifiers remain in the ignored `home-final-processes.json` receipt. Eight read-only CLI modes and ten SDK MCP tools passed, retaining one enclosure/pool, two HFS+ volumes, six slots, native icons and actual history observations. Mounted power operations stayed disabled. Final process readback confirmed one GUI and the original DDService unchanged.
- Computer Use launched the exact candidate through Finder. It showed the new icon/backdrop, single-unit auto-selection, five green reported bay lights, Drive 3 metadata after selection, unchanged protected/OS/raw accounting, tidy native volume rows and an unselected Tools control target. At 860 × 640, Volumes stacked without page-width overflow. The window was restored to 1120 × 800 and left on System Information. This is a scoped visual smoke, not a repetition of every page or full keyboard/VoiceOver/performance certification. No alternative GUI automation was used.

Fresh independent `v1_brand_review` (`gpt_ui_review`) passed the source/assets after inspection and 28 frontend tests. Fresh `v1_notice_review` (`gpt_reviewer`) passed after archive/lock and npm-integrity binding, default-feature reproduction, NOTICE-versus-terms, CLI-override and refusal-test fixes. Fresh `v1_release_audit` (`gpt_reviewer`) passed the package resource/target gates and 23 focused tests. Producers were `v1_brand_integration` and `v1_dependency_clearance` (`gpt_worker`), with root owning architecture, legal/security judgment, package integration, docs, validation and host actions. The installed TOMLs were rechecked: all three GPT roles are configured as OpenAI `gpt-5.6-terra`; runtime model IDs are not exposed. Root's runtime model identifier and the built-in image generator's model identifier are not exposed. No GLM, Spark, retired role or Mac-mini model was used. Final documentation/evidence review by `v1_release_audit` returned PASS after clarifying pending hardware bindings and keeping the new private home path/process identifiers in ignored receipts.

Private receipts are under `artifacts/reports/v1-followup/` and `artifacts/reports/v1-license-clearance/`. The latter includes actual `notices-v7/`, immutable upstream evidence, both original/refreshed schema receipts and a bounded service-identity feasibility summary. Public network reads were limited to locked dependency downloads, upstream licensing material, official documentation/schemas and isolated validation-tool installation; no application dependency was added or upgraded. Source/docs and newly rendered assets are the publication slice. Vendor files, private host captures and built packages remain ignored. No unrelated source dirt existed at startup.

No production-pack command/write, brightness query/change, physical light/power action, firmware/array operation, driver install/activation, filesystem mount/geometry change, sharing/ACL change, Developer account enrollment, purchase or credential operation occurred. The original driver remains required; Finder reporting is unchanged. The independently compiled driver is still an inactive prototype, and new-macOS/Apple-silicon hardware support cannot be claimed from it. No public binary release/tag was created.

Next owner actions: choose and enroll the publisher with Apple, then configure signing/notarization locally without sharing secrets; identify the spare 5D/pack and Apple-silicon Mac/OS/cable chain for the staged plan; complete the remaining terms/source-access review. Full accessibility/QA, legacy-source trust disposition and actual replacement-driver bring-up remain release gates.

Exact changed source/documentation/art paths:

```text
desktop/public/brand/README.md
desktop/public/brand/opendrobo-lineup-v2.png
desktop/src-tauri/icons/128x128.png
desktop/src-tauri/icons/128x128@2x.png
desktop/src-tauri/icons/32x32.png
desktop/src-tauri/icons/64x64.png
desktop/src-tauri/icons/Square107x107Logo.png
desktop/src-tauri/icons/Square142x142Logo.png
desktop/src-tauri/icons/Square150x150Logo.png
desktop/src-tauri/icons/Square284x284Logo.png
desktop/src-tauri/icons/Square30x30Logo.png
desktop/src-tauri/icons/Square310x310Logo.png
desktop/src-tauri/icons/Square44x44Logo.png
desktop/src-tauri/icons/Square71x71Logo.png
desktop/src-tauri/icons/Square89x89Logo.png
desktop/src-tauri/icons/StoreLogo.png
desktop/src-tauri/icons/icon.icns
desktop/src-tauri/icons/icon.ico
desktop/src-tauri/icons/icon.png
desktop/src-tauri/icons/source-v2.png
desktop/src-tauri/icons/source.svg
desktop/src/main.ts
desktop/src/styles.css
docs/DEVELOPER_GUIDE.md
docs/HARDWARE_VALIDATION.md
docs/MCP.md
docs/RELEASING.md
docs/RIGHTS_AND_DISTRIBUTION.md
docs/THIRD_PARTY.md
docs/V1_READINESS.md
docs/WORK_RECORD.md
scripts/build_desktop.py
scripts/dependency_licenses.py
scripts/release_notices.py
scripts/release_preflight.py
tests/test_dependency_licenses.py
tests/test_release_notices.py
tests/test_release_preflight.py
```

Continuation: start here with `git status --short --branch`, `git rev-parse HEAD` and `git log -1 --oneline`; read `docs/WORK_RECORD.md`, `docs/V1_READINESS.md`, `docs/RELEASING.md`, `docs/THIRD_PARTY.md`, then `docs/HARDWARE_VALIDATION.md` and the offline lab guide. Source is the reviewed follow-up identified by Git; home preview and both local packages remain ad-hoc/distribution-blocked. Active scope is the explicit remaining v1.0 gates. Preserve the home pack, DDService ownership, activation/geometry fences and separate destructive authority. Run focused checks only for new accepted changes, reuse unchanged evidence, and retain this ledger plus the named private receipts as the current status pointer.

## Startup, tray and indicator follow-up acceptance

The approved newly rendered enclosure art now appears in the independent startup screen; the prior checkpoint's matching application icon is retained in both packages. A new code-drawn macOS template tray depicts five horizontal bays. Its menu opens dashboard pages, requests read-only refresh, hides/reveals the window and quits. Closing the window hides it only after tray installation succeeds. CLI and MCP modes initialize no tray or webview. The bounded main-window queue preserves hidden-window confirmation cancellation, latest navigation and refresh independently; Show cannot discard a queued destination. Tray navigation does not grant a control target, query brightness or execute a device action.

The six bay meanings remain bound to the original selector. Ready is steady green, low space steady yellow, urgent capacity steady red, protection alternating green/yellow and a drive fault flashing red. Gray is OpenDrobo's unavailable/stale state, not a physical error code. The clearer legend adds persistent symbols, patterns, steady/flashing descriptions and action guidance; fault and protection retain nonanimated cues under Reduce Motion. Green is explicitly not a complete array-health assessment. Real fault transitions were not induced.

Validation and private evidence:

- Final `make check`: 26/26 lanes passed, including 77 Python tests, 54 backend tests, 40 portable core tests, seven probe tests and 36 frontend tests. Five startup tests cover paint/dismissal timing, manual skip followed by failure, persistent recovery, post-ready errors and early-error recovery. Five native tray tests cover action authority, main-window gating and queue order. Source reviews caught and corrected a hidden-error overlay, late error handling, pre-paint dismissal and lost rapid navigation/refresh clicks.
- Optimized Intel and ARM64 packages passed build and structural checks with all 118 recorded application/package inputs unchanged. They remain version 0.2.0, ad-hoc signed, without bundled research extensions and not distribution-ready. The notice/signature/notarization/Gatekeeper gates remain open. Build logs retain the existing unused test-only import and local Rust objcopy/LLVM warning; packaging and strict signature verification passed. Initial sandbox loopback fixture binding failed with `Operation not permitted`; the approved outside-sandbox run passed. A superseded in-flight package was rejected by the input-drift guard during review; only the final unchanged-input candidates were used.
- Intel archive SHA-256: `08ff002ab64169daa476b4d773f6eb8cc464679a0afb66e86d50a900e1a9b6a9`. ARM64 archive SHA-256: `f22bf27a1c3a915bd75aac8d3e0cfcd1270349058177e168b06ff65a31397ab5`.
- Both locked default macOS license graphs were regenerated and replayed after adding the existing `tray-icon` feature. No packages or versions changed: 290 components, 492 documents, 277 documentary terms-coverage entries and the same thirteen manual-review gaps. `defaultGraphVerified` is true and legal review remains required. Cargo.lock is unchanged.
- The exact final Intel candidate passed all eight read-only CLI modes and ten SDK MCP tools on the home Mac. Computer Use observed the matching Finder/Dock icon and monochrome tray; Status, single-unit selection and five green bays; close-to-tray and Capacity reopening; read-only Refresh; Tools without an established target; explicit tray Quit and Finder relaunch. The six-card legend was readable at 1120 × 800 and 860 × 640 without horizontal page overflow. The final view restores 1120 × 800 with the legend visible.
- Remote screenshot sampling missed the brief startup overlay. Its timing and recovery are verified by source review and automated tests, not claimed as direct visual acceptance on the home Mac. Full keyboard/VoiceOver, runtime accessibility preferences, performance, fault/disconnect transitions and Apple-silicon device transport remain open.
- Final read-only process proof found exactly one GUI at the verified trial path and the original service's PID and SHA-256 unchanged. Private process paths, serial/correlation data and observation receipts remain ignored under `artifacts/reports/splash-tray/`; no raw capture or host identity was added to public source.

Apple's current per-app approval guidance is linked in RELEASING.md. Developer membership is not required for a trusted ordinary local/tester dashboard build when macOS offers that exception; managed hosts can restrict it. The project's public v1.0 policy still requires Developer ID and notarization. No Gatekeeper/SIP/quarantine weakening, Apple account enrollment, certificate or credential operation was performed. Dashboard approval does not approve the separate inactive DriverKit research extension.

Producers were root plus native OpenAI `gpt_worker` tasks `splash_frontend` and `native_tray`; the reused dependency worker produced ignored receipt material only. Fresh read-only `splash_ui_review` (`gpt_ui_review`) and `tray_release_review` (`gpt_reviewer`) returned PASS after corrections. Role TOMLs configure `gpt-5.6-terra`; actual runtime model IDs are not exposed. No GLM, Spark or retired model lane was used. Root retains integration and release judgment. No sealed new security scan, full accessibility lane or hardware certification is claimed.

No device LED/brightness query or write, power command, original-service restart, driver/FSKit activation, Finder geometry change, sharing/ACL change, disk-pack operation, public binary release or tag occurred. The original driver dependency and v1.0 gates remain unchanged. There was no unrelated working-tree dirt. Standing source publication authority covers this reviewed main-branch slice; exact containing commit and public-main equality are checked at closeout. The next user-dependent work remains publisher enrollment/identity for a notarized release and the identified disposable 5D/pack plus named Apple-silicon test host.

Changed source/document paths:

- `README.md`
- `desktop/index.html`
- `desktop/public/startup.css`
- `desktop/public/startup.js`
- `desktop/src-tauri/Cargo.toml`
- `desktop/src-tauri/src/desktop_ui.rs`
- `desktop/src-tauri/src/main.rs`
- `desktop/src/desktop_actions.ts`
- `desktop/src/drive_indicator.ts`
- `desktop/src/main.ts`
- `desktop/src/styles.css`
- `desktop/tests/desktop_actions.test.mjs`
- `desktop/tests/startup.test.mjs`
- `docs/DEVELOPER_GUIDE.md`
- `docs/RELEASING.md`
- `docs/V1_READINESS.md`
- `docs/WORK_RECORD.md`
- `research/MACOS_MANAGEMENT_ABI.md`
- `scripts/check_desktop.py`

## Longer splash and compact capacity presentation acceptance

This amendment completes the current request from `989bff1b316545f8ca1ff865a59de928b0a59dad`. Automatic startup dismissal now allows three seconds after two animation frames; explicit skip and independent startup failure recovery remain intact. The legend uses narrow bar swatches matching the enclosure palette rather than icon cards. Steady capacity pressure and flashing drive faults retain visible text distinctions, and protection has a persistent green/yellow bar. Existing motion-reduction handling remains available; runtime OS preferences were not changed.

The capacity ring now blends green/cyan/violet/pink on its used arc over a translucent unused track. Unique SVG gradient IDs prevent cross-chart collisions; stale values still use neutral gray, and a zero-used value cannot draw a rounded-cap dot. Plain Storage capacity, Used space and Available space labels replace ordinary protected-pool wording. Drobo and macOS used-space values remain separately labeled and calculated. Display names omit the - TB suffix in headers, selectors, cards and grouped volume headings; original transport fields, backend/MCP metadata and ancestry-based grouping remain unchanged.

Validation: all 36 focused frontend tests passed, including five startup, six indicator, six storage-presentation, nine statistics, seven selection and three tray-action tests. Existing name/group tests now verify the cleaned label and unchanged source deviceName. Production TypeScript/Vite checks and Intel packaging passed. Local structural checks found only the existing incomplete-notice and ad-hoc/notarization/Gatekeeper distribution gates. All 118 application/package inputs stayed unchanged during build and final inspection. Intel archive SHA-256: `985db62a8ed5c7d97cea301d4357304ad35829af64e9803659edd72661a3f745`. The existing exact notice bundle remains applicable; Cargo/npm locks and dependency versions are unchanged.

The final Intel app passed eight read-only CLI modes on the home Mac. Computer Use directly captured the startup artwork/card across multiple successive frames and then the dashboard opening without a skip click. This closes the earlier missing direct splash visual observation. The configured interval is tested; precise visible duration was not timed on the host. Capacity visibly retained 11.75 TB usable, 3.75 TB used and 7.99 TB free with 3.76 TB used by macOS separately shown. The RGB used arc, translucent unused track, plain heading, shortened name and compact bar legend were observed. One final GUI at the expected private trial path and the unchanged original DDService PID/hash were verified; no hardware-control request was sent.

Root produced the changes. Fresh native OpenAI `gpt_ui_review` task `splash_capacity_review` returned PASS after exact source/state review; its role config pins `gpt-5.6-terra`, while actual runtime model IDs are not exposed. The reviewer did not operate the home GUI. Targeted UI/build checks replace an unnecessary repeat of the unaffected backend/native suite for this slice; no new ARM64 build, MCP suite, measured performance, full accessibility or live fault/disconnect proof is claimed. The preceding full-suite/ARM64/MCP evidence remains historical. No GLM or retired-model lane was used.

Changed paths are `desktop/public/startup.js`, `desktop/src/main.ts`, `desktop/src/storage_presentation.ts`, `desktop/src/styles.css`, `desktop/tests/startup.test.mjs`, `desktop/tests/storage_presentation.test.mjs`, `docs/DEVELOPER_GUIDE.md`, `docs/V1_READINESS.md`, `docs/WORK_RECORD.md` and `research/MACOS_MANAGEMENT_ABI.md`. Ignored receipts are under `artifacts/reports/splash-duration/`. There was no unrelated dirt. No LED/brightness/power command, driver or filesystem activation, disk operation, DDService restart, sharing/ACL change, public binary release or tag occurred. Original driver, signing/compliance and hardware gates remain unchanged. Standing source publication authority covers the reviewed main-branch update; closeout verifies containing commit/public-main equality. Next verification remains the named spare-unit and Apple-silicon trial, under their existing separate hardware plan.

## README screenshot gallery acceptance

The user requested screenshots of different sections and current product copy in the GitHub repository, then explicitly clarified that capture should happen on this Mac. From `ad7599d21ce7957ce26fa2a5ffdbd1380e10b532`, the README now contains six linked app images: All Drobos, Status, Capacity, Usage, Statistics and Tools. Feature text describes the current compact light bars, RGB used-space ring/translucent unused track, three-second skippable artwork splash and menu-bar navigation. Compatibility, original-driver dependency, separate macOS accounting and unavailable operation boundaries remain intact.

The home-Mac app was navigated through Computer Use. Captures came from the specific local Screen Sharing window and were framed to the observed 1120 × 800 app window, as the user requested. The initial remote Screenshot utility was canceled without taking a picture. A first local region capture included an obscuring window and was rejected into ignored local evidence; only the later window-specific captures were selected. Standard sRGB conversion replaces the local display profile. No app text, values, status or graph points were fabricated or replaced. Selected images omit surrounding desktop, private paths, host/device identifiers, raw IDs and diagnostic exports. Public assets include only the standard sRGB color profile, with no EXIF or text metadata.

Validation decodes all six PNGs, checks exact dimensions, verifies each image against its original frame with the documented color conversion, checks all README/gallery/work-record local links, and confirms that all six README image references exist. The six assets total 4,654,717 bytes. `git diff --check` passes and no application source, dependency or build input changed. Every image was visually inspected; genuine flat history and observation gaps remain visible. The capture guide identifies the application source checkpoint, observed configuration and current preview limitations. No app/native test suite or package rebuild was repeated for this documentation-only change.

GUI navigation ended on Capacity with Screen Sharing's Control Screen switched off. No control target, hardware command, brightness read/write, power operation, original-service restart, driver/FSKit activation, disk operation, sharing/ACL change or binary release/tag was involved. No screenshot or GUI automation was performed through SSH. Private/full-window/rejected images and receipts remain ignored under `artifacts/reports/readme-gallery/`; only the six selected app images are included in source publication.

Changed paths are `README.md`, `docs/WORK_RECORD.md`, `docs/screenshots/README.md`, `docs/screenshots/all-drobos.png`, `docs/screenshots/status.png`, `docs/screenshots/capacity.png`, `docs/screenshots/usage.png`, `docs/screenshots/statistics.png` and `docs/screenshots/tools.png`. Root produced the captures and documentation. Fresh read-only `readme_gallery_review` (`gpt_ui_review`, native OpenAI) inspected all six images, captions, source claims and privacy boundaries and returned PASS with no actionable P1/P2/P3 findings. The configured reviewer role pins `gpt-5.6-terra`; runtime model IDs are not exposed. Standing source publication authority covers this gallery; closeout verifies containing commit/public-main equality and a clean worktree. Existing v1.0, hardware, signing, source-trust and rights gates remain unchanged.
