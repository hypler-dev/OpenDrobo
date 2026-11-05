# OpenDrobo work record

Last updated: November 5, 2026. This is the canonical status and acceptance record.

## Accepted intent

The user authorized an independent OpenDrobo application with the original Dashboard interaction model, modern macOS/Apple silicon support, a Rust core, a shared Tauri UI, and offline reverse engineering of the original service, driver, and firmware. The user also authorized publication of the latest source updates to the public `hypler-dev/OpenDrobo` repository. Long-term scope includes BeyondRAID recovery and Linux/Windows adapters.

The former third-party application dependency has been removed. No external parser checkout or copied parser is part of the build. Original binaries, firmware, extracted artwork, and REA/Ghidra output remain ignored local research artifacts with recorded provenance. New parser code comes from direct analysis of the original vendor software.

## Current implementation

- `crates/opendrobo-core`: portable Rust capacity decoder/C ABI, copied Thunderbolt port/frame codecs, and an offline RMR header-candidate inspector.
- `crates/opendrobo-probe`: a bounded, file-only inspection CLI independent of the original Drobo driver.
- `desktop/`: shared Tauri 2 TypeScript/CSS interface with original-style navigation, dark glass surfaces, animated selection glow, empty states, and release lookup.
- `desktop/src-tauri/`: Rust commands for registry discovery, bounded read-only capacity calls, raw hexadecimal lab capture, and scoped GitHub release checks.
- `platform/macos/transport/`: C bridge for original user-client ABI. Query is open → LUN page 4 → capacity page 2 → close; no arbitrary or write commands.
- `platform/macos/fskit/`: Swift FSKit read-only separate-view module with path/inode binding, held descriptors, directory enumeration, and conservative symlink confinement.
- `platform/macos/driverkit/`: an ARM64 SDK/IIG SCSI service, read-only policy, DMA leases, interrupt interpreter, bounded version-4 storage/management engines, inline logical-geometry replies, completion-based graceful drain, and SDK resource adapters. The local `.dext` is disabled before hardware access and has not been installed or activated.

The native source intentionally passes `byteUnitsConfirmed: false`. Injected FSKit tests exercise a positive confirmed record, but live provider statistics remain unavailable until hardware evidence exists. The older `macos/` SwiftUI app and `experiments/capacity/` Swift prototype remain reference code; `make build` uses the Rust/Tauri application.

The macOS bridge depends on the original Drobo management driver already being loaded. It does not replace the Thunderbolt PCI/DMA or DriverKit storage driver. Linux and Windows currently have shared source direction only; neither target has been built or tested here.

## Capacity truth

Recovered protected-capacity fields are documented in [CAPACITY_REPORTING.md](CAPACITY_REPORTING.md), with the recovered call structure in [MACOS_MANAGEMENT_ABI.md](../research/MACOS_MANAGEMENT_ABI.md). Device byte units are **not hardware-verified**. Live decoding therefore reports unavailable; raw capture is evidence collection only. No model profile currently enables byte reporting.

The FSKit view does not override an existing Apple HFS+ mount or change disk geometry. The extension is compiled and directly checked, but has not been activated, mounted, or validated in Finder. Its nonthrowing statistics callback returns zeroed fields when information is unavailable; those zeros are not a measured empty pack. The user believes the home volume uses HFS+, but this has not been confirmed. No home-Mac access or attached-device command occurred in this workspace.

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

Hardware proof remains open: exact enclosure model, home macOS version, filesystem, adapter chain, LUN count, byte units, Finder statistics, disconnect behavior, and multiple-LUN policy. BeyondRAID reconstruction remains research-only. Network management remains proposed authenticated host agent. No firmware, restart, shutdown, format, rebuild, array mutation, disk erase, or home-Mac operation is authorized by source checks.

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

## Next safe slice

Add stronger SDK-service backend mocks for drain/source cancellation and partial lifecycle; trace remaining completion/error paths and hardware-wide quiescence without treating PCI Close/reset as proof; complete SCSI sense/mode-page and reset/power policies. Define entitlement/provisioning and a named spare-unit activation plan only after those gates are met. Connect an independent management service to the application with verified device/LUN/unit binding. Capture true protected capacity and compare FSKit `statfs`/Foundation/Finder on the same read-only view. A smaller READ CAPACITY is not a safe way to change existing HFS+ statistics. BeyondRAID member locations/generations and real-image reconstruction remain separate research.

## Continuation

```text
Repo: OpenDrobo; start with pwd, git branch --show-current, git rev-parse HEAD, git status --short --branch.
Read: README.md -> docs/WORK_RECORD.md -> research/THUNDERBOLT_TRANSPORT.md -> research/BEYONDRAID_LAYOUT.md -> docs/CAPACITY_REPORTING.md.
Truth: Rust/Tauri app still needs original live driver; 5D selected SGL field resolved; graceful owned-request drain integrated; twelve ARM64 objects, seven native suites and all18 lanes pass; hardware gate closed, no Finder proof.
Scope: stronger service/cancellation mocks; remaining firmware/error paths; proven hardware quiescence and SCSI/power behavior; independent app management binding; capacity-unit/FSKit evidence; RMR image research.
Do not: access home Mac, install/activate drivers or extensions, mount, flash, format, erase, mutate arrays, or publish vendor artifacts.
Validate: make check; scripts/check_driverkit.py after native changes; make build after app/package changes; fresh review; hardware only on named spare plan.
Ledger: docs/WORK_RECORD.md; artifacts/reports; https://github.com/hypler-dev/OpenDrobo.
```
