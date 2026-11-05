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
- `platform/macos/driverkit/`: an ARM64 SDK task projection and pure read-only command policy; no driver bundle or live transport.

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

## Next safe slice

Implement the remaining DriverKit controller lifecycle, mapped-buffer ownership, request publication, interrupts, synchronization, timeouts, and disconnect cleanup using the recovered structures. Prove them against mocked MMIO before a named spare-unit activation test. For BeyondRAID, establish member-image locations and generation selection, then validate CRC and known reconstructed bytes against real images. Collect read-only capacity records to establish byte units and pool scope before testing FSKit mount/statistics/Finder agreement. Keep hardware compatibility and public releases separate from software-only validation.

## Continuation

```text
Repo: OpenDrobo; start with pwd, git branch --show-current, git rev-parse HEAD, git status --short --branch.
Read: README.md -> docs/WORK_RECORD.md -> research/THUNDERBOLT_TRANSPORT.md -> research/BEYONDRAID_LAYOUT.md -> docs/CAPACITY_REPORTING.md.
Truth: Rust/Tauri app still needs original live driver; independent offline probe/codecs and non-loading DriverKit adapter compile; no hardware/Finder proof.
Scope: actual DriverKit transport lifecycle, capacity-unit lab evidence, RMR image locations/generations, FSKit read-only view.
Do not: access home Mac, install/activate drivers or extensions, mount, flash, format, erase, mutate arrays, or publish vendor artifacts.
Validate: make check; make build; fresh native/module review; hardware tests only on named spare plan.
Ledger: docs/WORK_RECORD.md; artifacts/reports; https://github.com/hypler-dev/OpenDrobo.
```
