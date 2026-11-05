# OpenDrobo work record

Last updated: November 5, 2026. This file is the canonical status and acceptance record.

## Authority and intended result

The user's current request authorizes local analysis of the original Drobo software, use of REA, a native macOS/Apple silicon application, an independent project repository, the original Dashboard layout with glass/transparency/RGB selection effects, original artwork in the local build, and an update source pointing to this project's latest GitHub release. The longer-term objective is an independent, open BeyondRAID implementation.

The user explicitly requested a new GitHub repository. `hypler-dev/OpenDrobo` was created public. No app release, firmware release, or hardware compatibility certification is authorized by passing tests alone.

The user also requested Linux/Windows support as a direction. The capability record now describes a portable core, separate OS transport adapters, and platform frontends. This extends the roadmap; it does not establish Linux/Windows builds or hardware compatibility.

The user subsequently directed removal of the previously included third-party application dependency and its README references. This means removing its derived parser, fixtures, acquisition/build integration, and bundled notice together. The current app must build from this repository's own application source without an external checkout; it does not ship a replacement protocol parser.

## Current scope

- Build and launch a native macOS 26 universal app with original-style navigation and truthful empty/unavailable states.
- Provide local registry discovery and a latest-stable-release lookup for `hypler-dev/OpenDrobo`.
- Preserve original software/firmware locally, record hashes, and inspect them offline with REA/Ghidra.
- Produce reproducible TDIH/ELF inventory tooling and a bounded next research slice.
- Extend static Thunderbolt/production storage analysis and distinguish legacy networking from proposed host-agent management.
- Apply the user's darker transparent UI correction, faded original product lineup, blended section/card surfaces, and animated perimeter selection effects.

Current work does not install drivers, mount or format a Drobo volume, flash firmware, manipulate a disk pack, replace BeyondRAID, or access the home Mac. Spare hardware availability is not an instruction to erase unspecified drives.

## Acceptance evidence

| Item | Evidence / current result |
| --- | --- |
| Original software acquired | Vendor DMG SHA-256 recorded in research findings |
| Original UI inspected | Original app reached All Drobos and Dashboard Preferences locally |
| Native app build | Universal arm64/x86_64 binary built; Apple silicon launch observed |
| Original-style app UI | OpenDrobo All Drobos and Dashboard Preferences observed through accessibility and screenshot |
| GitHub release lookup | Check Now returned “No app releases have been published yet.” |
| Glass/RGB/artwork update | Native build passes; default All Drobos/preferences render and AX states verified; original glyph padding and sidebar shift corrected |
| Protocol parsing | No protocol parser is shipped; the previous derived parser and its fixture/suite have been removed |
| App/policy checks | 25 independent Swift assertions pass: PCI IDs/byte order, SCSI registry identity, versions, and release links |
| Python checks | 25 tests pass: archive boundaries, TDIH CRC/length validation, ELF tables/programs/symbols, and byte-exact extraction |
| App signature/package | `codesign --verify --strict` passes; ad-hoc local build only, not notarized |
| Final command-line discovery | `--scan-json` returned an empty list on the local Mac; no device commands sent |
| Firmware containers | 5D/5D3 length, header CRC, and body CRC match |
| Embedded storage application | Both images extracted/inventoried; 10,598 / 10,628 named symbols |
| REA native analysis | Ghidra 12.1.4 provider healthy; original Thunderbolt initializer, on-disk-index signature validator, and Configure function decompiled |
| Follow-up static recovery | Driver handshake/channels/power paths, index getters/load/version paths, production RMR validation/CRC, and address maps inspected; selected constants/paths assembly corroborated; see `research/STATIC_RECOVERY.md` |
| Network/community scope | Legacy NAS capability matrix and attributed community backlog recorded in `docs/CAPABILITIES_AND_BACKLOG.md`; host agent proposed only |
| Dark/blended UI follow-up | Universal `make build` and signature verification pass; full-panel Preferences and Help screenshots observed; Status/Capacity/Volumes/Tools/Settings navigation checked; source-reviewed shared surfaces cover all sections |

Protocol and synthetic-status assertions from the initial checkpoint belong to the removed dependency. They are not current application coverage or evidence of live management support.

## Ownership and review

The controller owns architecture, scope, external state, research, source acquisition, updates, integration, validation, and release judgment. A bounded `gpt_worker` produced the native UI; the controller applied final integration fixes. Initial review identified protocol-status and PCI-byte-order gaps, while archive/ELF/extraction coverage was added. The current source no longer includes that protocol parser or its checks. Independent UI review corrected a hardcoded reduced-transparency flag on segment glow.

Rendered UI evidence was collected by the controller and supplied to the independent UI reviewer. That reviewer did not independently operate the application. OS Reduce Transparency and Reduce Motion toggles were not manually changed; their source guards were reviewed. No attached-device, Intel-execution, or home-Mac compatibility proof is claimed.

The darker follow-up was produced by the same bounded UI worker in `macos/ContentView.swift`, with controller integration. Agent task `/root/glass_ui_review`, using the `gpt_ui_review` role, found an opaque fallback covering its own color blend; the controller corrected the background composition. Rendered Preferences exposed a content-width seam; the controller moved full-panel sizing before the shared background. Both corrections received fresh source review with no remaining findings. The controller observed the final Preferences/Help render and navigated all remaining management sections through CUA. This is representative rendered verification plus shared-surface source coverage, not a claim that every OS accessibility setting or hardware state was exercised.

The pre-removal UI build ZIP SHA-256 was `920d5c4366e7eabfb2bd5238d5c5f9ab267ad4c2ca05e2fa83e4e428e2c5047b`. Its receipts are `artifacts/reports/round2/full-panel-blends-build.log`, `final-validation.json`, and `ui-render-observations.json`. These prove that earlier styling build, not the subsequent dependency-free bundle. Final removal-slice validation supersedes them. No performance benchmark was run.

The removal slice was produced by agent task `/root/remove_dependency` using the `gpt_worker` role, with controller changes to independent SCSI registry matching, identity checks, metadata, and documentation. `make check` passes 25 Python tests and 25 independent Swift assertions. `make build` succeeds with the removed source checkout absent and compiles only `ContentView.swift`, `LocalDiscovery.swift`, `PreviewApp.swift`, and `ReleaseUpdater.swift`. The resulting binary contains arm64 and x86_64; strict ad-hoc signature verification passes. The bundle and ZIP contain no removed dependency names/notices, and the bundle metadata has no upstream-source field. Final ZIP SHA-256: `b138886d185224d74578371dc53c35fea35d00ea13a20d9daf561f22492b4ff5`.

Receipts: `artifacts/reports/round2/independent-commands.json`, `independent-check.log`, `independent-build.log`, and `independent-validation.json`. Command-line discovery returns an empty list. The controller closed the earlier app, observed it stopped, launched the new bundle, and verified All Drobos accessibility state plus the Preferences dark/blended render. No device was attached. The independent registry predicate uses facts from the original vendor personality file, recorded in `original-scsi-identities.json`; generic legacy TRUSTED mass-storage names are excluded pending stronger identity evidence, so older devices may require additional matching work.

Fresh agent task `/root/dependency_review`, using the `gpt_reviewer` role, reviewed the removal, local-only build/check paths, identity policy and tests, documentation, and final receipts. It returned PASS with no actionable P1/P2/P3 findings. The controller accepts this bounded source checkpoint; hardware and platform compatibility gates remain open.

Agent task `/root/recovery_review`, using the `gpt_reviewer` role, independently reviewed the research/capability notes and status changes. Missing signature-literal, service-architecture, and build/validation receipts were supplied and reviewer role/task attribution was clarified. Final review returned PASS with no actionable P1/P2/P3 findings, including a separate follow-up on the Linux/Windows roadmap. All 24 original artwork manifest entries match their local source hashes and local bundle entries; the images remain ignored. Documentation link checks and `git diff --check` pass.

The local role TOMLs pin worker/reviewer roles to `gpt-5.6-terra` on OpenAI. Actual provider runtime IDs are not exposed by the agents, so configuration pins are not represented as independently observed runtime identities. No GLM, Spark, Hermes, Gemma, or Mac mini model was invoked.

## Source and artifact boundaries

New source lives in `macos`, `scripts`, `tests`, and `research`; licenses, build instructions, and this record are included. Original installers, firmware, driver binaries, extracted artwork, REA/Ghidra/JDK/Node runtimes, decompilation outputs, and build products are ignored under `artifacts`, `third_party`, `tools`, and `dist`.

Artwork provenance is recorded, but redistribution rights have not been verified. Source-only public builds use native fallback artwork. Local app bundles may include the explicitly requested extracted originals.

There was no pre-existing Git repository or unrelated work in this folder. Local `main` and remote `origin` now exist. The initial reviewed source checkpoint is `4419efadecd2c1566e35036cc11c7d47e9604736`. Later source changes are identified by their containing commits. Check `git log -1`, remote `main`, and the task closeout for publication receipts. No tag or app/firmware release has been published.

The follow-up changes the native UI, build/check entry points, registry identity matching, research/capability notes, provenance manifest, and README/work record. It removes the derived protocol parser, unused synthetic fixture, source-acquisition script, bundled third-party notice, and corresponding dependency pin. The containing commit gives the exact changed-path list.

The user reaffirmed GitHub repository creation as “Open Drobo.” The existing public `hypler-dev/OpenDrobo` repository was verified through the configured GitHub CLI and reused. No duplicate repository or update-source rename was introduced.

Removed dependency source/archive and old check executables were moved out of active paths into ignored `artifacts/retired-dependencies` for research provenance. They are not build inputs or public source files. Existing Git history remains intact; no attribution was erased from retained derived code, because the derived code itself was removed.

## Limits and next action

The app currently discovers metadata and checks releases. Thunderbolt management/storage transport and the BeyondRAID data engine remain unimplemented. No hardware behavior is inferred from an empty local scan. Exact enclosure model and home macOS version still need identification before hardware testing.

Next: establish serialized RMR boundaries and header/buffer pointer relationships, then validate metadata generations and CRC against known member images. Prepare a separate spare-unit plan identifying the exact enclosure, disks, imaging destination, and permitted changes before destructive experiments. The native glass/artwork app is a launch/discovery preview; it is not hardware management or an array implementation. Remote home-Mac management remains a proposed authenticated agent, not a running service.

## Continuation

```text
Repo: OpenDrobo; start with pwd, git branch --show-current, git rev-parse HEAD, git status --short --branch.
Read: README.md -> docs/WORK_RECORD.md -> research/REVERSE_ENGINEERING.md.
Truth: native preview only; vendor/firmware/decompiler artifacts local; no Thunderbolt or BeyondRAID implementation.
Scope: original-style glass/RGB UI, latest GitHub release lookup, offline original-system analysis.
Do not: access home Mac, install drivers, flash, format, mutate disk packs, or publish vendor artifacts.
Validate: make check; make build; native launch and update/empty/error states; fresh independent review.
Ledger: docs/WORK_RECORD.md; local receipts artifacts/reports; repo https://github.com/hypler-dev/OpenDrobo.
```
