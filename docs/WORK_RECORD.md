# OpenDrobo work record

Last updated: November 4, 2026. This file is the canonical status and acceptance record.

## Authority and intended result

The user's current request authorizes local analysis of the original Drobo software, use of REA, a native macOS/Apple silicon application, an independent project repository, the original Dashboard layout with glass/transparency/RGB selection effects, original artwork in the local build, and an update source pointing to this project's latest GitHub release. The longer-term objective is an independent, open BeyondRAID implementation.

The user explicitly requested a new GitHub repository. `hypler-dev/OpenDrobo` was created public. No app release, firmware release, or hardware compatibility certification is authorized by passing tests alone.

## Current scope

- Build and launch a native macOS 26 universal app with original-style navigation and truthful empty/unavailable states.
- Provide local registry discovery and a latest-stable-release lookup for `hypler-dev/OpenDrobo`.
- Preserve original software/firmware locally, record hashes, and inspect them offline with REA/Ghidra.
- Produce reproducible TDIH/ELF inventory tooling and a bounded next research slice.

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
| Protocol regression checks | 176 upstream-derived assertions pass with a corrected complete-status fixture |
| App/policy checks | 30 assertions pass: PCI IDs, byte order, missing/truncated status and labels, versions, and release links |
| Python checks | 25 tests pass: archive boundaries, TDIH CRC/length validation, ELF tables/programs/symbols, and byte-exact extraction |
| App signature/package | `codesign --verify --strict` passes; ad-hoc local build only, not notarized |
| Final command-line discovery | `--scan-json` returned an empty list on the local Mac; no device commands sent |
| Firmware containers | 5D/5D3 length, header CRC, and body CRC match |
| Embedded storage application | Both images extracted/inventoried; 10,598 / 10,628 named symbols |
| REA native analysis | Ghidra 12.1.4 provider healthy; original Thunderbolt initializer, on-disk-index signature validator, and Configure function decompiled |

The upstream protection assertion originally expected protection without a status record. The local validation runner supplies a complete zero status for its “bit 6 clear” assertion and separately checks that missing/truncated status remains unknown. The pinned upstream source is not edited.

## Ownership and review

The controller owns architecture, scope, external state, research, source acquisition, updates, integration, validation, and release judgment. A bounded `gpt_worker` produced only `macos/ContentView.swift` and `macos/PreviewApp.swift`; the controller applied final integration fixes. Fresh `gpt_reviewer` review found and resolved missing-status, truncation, status-label, and PCI-byte-order gaps; archive, ELF program/symbol, and extraction coverage was added. Final non-UI review returned PASS with no remaining findings. Fresh `gpt_ui_review` found a hardcoded reduced-transparency flag on segment glow; it was corrected, and review returned PASS with no remaining findings.

Rendered UI evidence was collected by the controller and supplied to the independent UI reviewer. That reviewer did not independently operate the application. OS Reduce Transparency and Reduce Motion toggles were not manually changed; their source guards were reviewed. No attached-device, Intel-execution, or home-Mac compatibility proof is claimed.

The local role TOMLs pin worker/reviewer roles to `gpt-5.6-terra` on OpenAI. Actual provider runtime IDs are not exposed by the agents, so configuration pins are not represented as independently observed runtime identities. No GLM, Spark, Hermes, Gemma, or Mac mini model was invoked.

## Source and artifact boundaries

New source lives in `macos`, `scripts`, `tests`, and `research`; licenses, build instructions, and this record are included. Original installers, firmware, driver binaries, extracted artwork, REA/Ghidra/JDK/Node runtimes, decompilation outputs, and build products are ignored under `artifacts`, `third_party`, `tools`, and `dist`.

Artwork provenance is recorded, but redistribution rights have not been verified. Source-only public builds use native fallback artwork. Local app bundles may include the explicitly requested extracted originals.

There was no pre-existing Git repository or unrelated work in this folder. Local `main` and remote `origin` now exist. This record belongs to the initial reviewed source checkpoint; the containing commit identifies that source. Check `git log -1`, remote `main`, and the task closeout for publication receipts. No tag or app/firmware release has been published.

## Limits and next action

The app currently discovers metadata and checks releases. Thunderbolt management/storage transport and the BeyondRAID data engine remain unimplemented. No hardware behavior is inferred from an empty local scan. Exact enclosure model and home macOS version still need identification before hardware testing.

Next: trace the embedded storage application's metadata layout and prepare a separate spare-unit plan identifying the exact enclosure, disks, imaging destination, and permitted changes before destructive experiments. The native glass/artwork app is a launch/discovery preview; it is not hardware management or an array implementation.

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
