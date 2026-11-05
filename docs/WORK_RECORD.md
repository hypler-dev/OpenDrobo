# OpenDrobo work record

Last updated: November 5, 2026. This is the canonical status and acceptance record.

## Accepted intent

The user authorized an independent OpenDrobo application with the original Dashboard interaction model, modern macOS/Apple silicon support, a Rust core, a shared Tauri UI, and offline reverse engineering of the original service, driver, and firmware. The user also authorized publication of the latest source updates to the public `hypler-dev/OpenDrobo` repository. Long-term scope includes BeyondRAID recovery and Linux/Windows adapters.

The former third-party application dependency has been removed. No external parser checkout or copied parser is part of the build. Original binaries, firmware, extracted artwork, and REA/Ghidra output remain ignored local research artifacts with recorded provenance. New parser code comes from direct analysis of the original vendor software.

## Current implementation

- `crates/opendrobo-core`: portable Rust capacity decoder and validation C ABI.
- `desktop/`: shared Tauri 2 TypeScript/CSS interface with original-style navigation, dark glass surfaces, animated selection glow, empty states, and release lookup.
- `desktop/src-tauri/`: Rust commands for registry discovery, bounded read-only capacity calls, raw hexadecimal lab capture, and scoped GitHub release checks.
- `platform/macos/transport/`: C bridge for original user-client ABI. Query is open → LUN page 4 → capacity page 2 → close; no arbitrary or write commands.
- `platform/macos/fskit/`: Swift FSKit read-only separate-view module with path/inode binding, held descriptors, directory enumeration, and conservative symlink confinement.

The native source intentionally passes `byteUnitsConfirmed: false`. Injected FSKit tests exercise a positive confirmed record, but live provider statistics remain unavailable until hardware evidence exists. The older `macos/` SwiftUI app and `experiments/capacity/` Swift prototype remain reference code; `make build` uses the Rust/Tauri application.

The macOS bridge depends on the original Drobo management driver already being loaded. It does not replace the Thunderbolt PCI/DMA or DriverKit storage driver. Linux and Windows currently have shared source direction only; neither target has been built or tested here.

## Capacity truth

Recovered protected-capacity fields are documented in [CAPACITY_REPORTING.md](CAPACITY_REPORTING.md), with the recovered call structure in [MACOS_MANAGEMENT_ABI.md](../research/MACOS_MANAGEMENT_ABI.md). Device byte units are **not hardware-verified**. Live decoding therefore reports unavailable; raw capture is evidence collection only. No model profile currently enables byte reporting.

The FSKit view does not override an existing Apple HFS+ mount or change disk geometry. The extension is compiled and directly checked, but has not been activated, mounted, or validated in Finder. Its nonthrowing statistics callback returns zeroed fields when information is unavailable; those zeros are not a measured empty pack. The user believes the home volume uses HFS+, but this has not been confirmed. No home-Mac access or attached-device command occurred in this workspace.

## Validation and packaging

`make check` passed all 15 named lanes: 25 Python firmware/archive tests, 16 Rust core tests, 1 Tauri backend policy test, 25 legacy Swift policy assertions, 91 historical capacity-prototype assertions, formatting/build checks, native transport ASAN/UBSAN mocks, warning-clean C syntax, and FSKit typecheck/direct checks. After the final icon-rendering adjustment, frontend typechecking and production asset compilation passed again. These are software checks, not hardware or Finder proof.

`make build` passed and produced the local ARM64 debug preview at `dist/OpenDrobo.app`. It embeds the inactive FSKit extension under `Contents/Extensions`, includes optional ignored local artwork, and ad-hoc signs the nested extension and host. Strict signature verification passed for both, and for the app extracted from its ZIP with `ditto`. The initial misplaced entitlements-file packaging failure was corrected before this final package. No installer, activation, notarization, public binary upload, or release occurred.

Final archive: `dist/OpenDrobo-0.2.0-preview-macos-arm64.zip`. SHA-256: `180547f9311a62fcf70d2d122e4e37b46e20a82cba3f5cea4eb6c8a3d72d65b1`.

The controller observed a clean quit and final-bundle relaunch on macOS 26/Apple silicon. The 1120 × 800 render showed readable original navigation glyphs, dark glass panels, the faded product lineup, and RGB borders confined to their controls. List selection, registry refresh, unavailable management sections, and the Preferences panel were checked. Automatic release checks remained off; manual Check Now progressed through its disabled busy state to “No app releases have been published yet.” The empty local scan returned `{platform, devices, message}` with no devices. Invalid raw-capture arguments returned the expected errors before device access. Runtime OS Reduce Motion/Transparency settings were not changed, and no performance benchmark was run.

Local receipts:

- `artifacts/reports/desktop-checks/results.json` and its 15 lane logs.
- `artifacts/reports/desktop-build.log` and `desktop-build.json`.
- `artifacts/reports/desktop-validation.json` and `desktop-ui-observations.json`.
- Original analysis receipts under `artifacts/reports/capacity` and `artifacts/reports/transport`.

The controller owns architecture, native transport/backend integration, validation, and publication judgment. Producer tasks `rust_core`, `tauri_frontend`, and `fskit_module` used the native OpenAI `gpt_worker` role. Fresh `module_review` and `native_review` used `gpt_reviewer`; `shared_ui_review` used `gpt_ui_review`. All returned PASS after their findings were resolved. Fixes included explicit unit gating, binding before/after capture and root opening, root-path replacement checks, confined symlinks, lifecycle serialization, nontrapping directory verifiers, overlap-safe C ABI decoding, cross-platform bundle targets, and UI busy states. UI review used controller-supplied rendered observations plus independent source review; the reviewer did not operate the GUI.

The worker and reviewer TOMLs configure OpenAI `gpt-5.6-terra`; `architecture_docs` used `gpt_docs`, configured as `gpt-5.6-luna`. Actual runtime model IDs are not exposed, so configuration pins are not represented as observed runtime identities. No GLM, Spark, Hermes, Gemma, or Mac mini model was used.

## Source checkpoint

This slice changes the root Cargo workspace and build entry points, `crates/opendrobo-core`, `desktop`, `platform/macos`, the management-ABI research note, and the four architecture/status documents. The optional Swift reference build now has a separate output and bundle identity. Original vendor assets, generated artwork/schema directories, dependencies, toolchains, and build products remain ignored. There was no unrelated working-tree dirt.

The previous published checkpoint was `2f07c14ab72d1e4783aa6d9a0c3e310371086bdb`. This record's containing commit identifies the new source checkpoint and exact changed paths. Confirm publication by comparing `git rev-parse HEAD` with `git ls-remote origin refs/heads/main`; the closeout records that result. No tag, app release, deployment, driver installation, extension activation, filesystem mount, disk write, or firmware operation is part of this checkpoint.

## Open gates

Hardware proof remains open: exact enclosure model, home macOS version, filesystem, adapter chain, LUN count, byte units, Finder statistics, disconnect behavior, and multiple-LUN policy. BeyondRAID reconstruction remains research-only. Network management remains proposed authenticated host agent. No firmware, restart, shutdown, format, rebuild, array mutation, disk erase, or home-Mac operation is authorized by source checks.

## Next safe slice

Use spare hardware for read-only identity and capacity captures, image and hash member disks, validate units and pool scope, then test FSKit mount/statistics/Finder agreement. Record each model and OS separately from software-only validation. Keep release publication closed until hardware evidence and independent review pass.

## Continuation

```text
Repo: OpenDrobo; start with pwd, git branch --show-current, git rev-parse HEAD, git status --short --branch.
Read: README.md -> docs/WORK_RECORD.md -> docs/CAPACITY_REPORTING.md -> research/MACOS_MANAGEMENT_ABI.md -> research/STATIC_RECOVERY.md.
Truth: Rust/Tauri source plus macOS read-only bridge; original driver required; no hardware/Finder proof.
Scope: transport framing, capacity-unit lab evidence, FSKit read-only view, BeyondRAID static recovery.
Do not: access home Mac, install/activate drivers or extensions, mount, flash, format, erase, mutate arrays, or publish vendor artifacts.
Validate: make check; make build; fresh native/module review; hardware tests only on named spare plan.
Ledger: docs/WORK_RECORD.md; artifacts/reports; https://github.com/hypler-dev/OpenDrobo.
```
