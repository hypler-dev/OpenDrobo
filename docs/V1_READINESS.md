# v1.0 readiness audit

The application quality audit and source remediation are complete for the scope below. **Public v1.0 release remains blocked.** This document records evidence and remaining gates; it is not a release or security certificate. [WORK_RECORD.md](WORK_RECORD.md) remains the canonical accepted scope and publication record.

## Scope and supported evidence

The audit began at `ee9f9fd68ab68f20372d4e91973fc41e7c0f1cfc` and covered the shared interface, Rust runtime, native storage paths, portable decoders, tests, packaging, dependencies, documentation and product claims. Independent read-only reviews covered interface, backend, native/core and release tooling. Resulting source fixes passed fresh review and validation.

Hardware evidence covers one Drobo 5D through the original Thunderbolt driver on Intel macOS 14.7.7. Apple-silicon packages compile; device transport has not been verified on an Apple-silicon Mac. Linux and Windows device adapters remain future work. Replacement-driver operation, Finder capacity changes and BeyondRAID reconstruction remain unavailable. The offline spare-unit kit prepares test plans and never dispatches hardware operations.

## Resolved findings

| Area | Correction and evidence |
| --- | --- |
| Selection and control authority | Viewing Status, Capacity or an enclosure never creates a control target. Tools requires explicit selection; identity changes/disappearance clear prior authority. Duplicate models receive distinct stable labels. Regression tests cover these transitions. |
| Capacity and missing data | Capacity and Usage share exact accounting guards. Missing or inconsistent data cannot become zero or plausible capacity. OS filesystem usage, protected allocation, raw installed capacity and cache occupancy remain distinct. History failures retain their backend explanation. |
| Interface and product copy | Removed the development banner and dummy discovery, email, language and tray preferences. Added useful Help content, real MCP executable-path handling, visible keyboard focus, named main regions, skip navigation, route-focus handling and local overflow containers. |
| Service bounds | The bounded reader now accepts complete snapshots for the existing 32-unit limit: 64 frames, 2 MiB total and 512 KiB per frame. Tests exercise 15, 16 and 32 units and refusal above the unit cap. |
| Native and command failures | Tests exercise scope/capture/rebind failures, oversize native replies and unconfirmed units. Partial, malformed and stalled control replies remain unknown and never cause automatic command retries. |
| Inactive filesystem research | Nonintegral 4096-byte capacity values now leave all filesystem statistics unset. Finder reporting remains inactive. |
| Inactive driver research | Synthetic SCSI inquiry no longer invents a device serial from a registry identifier. Timer/queue cancellation errors retain ownership, report failure and support explicit retry before one completion. SDK compilation and sanitizer tests pass; real deferred callbacks and DMA quiescence still require hardware proof. |
| Packaging | Default builds are optimized, use generated artwork and exclude inactive filesystem research. Debug, private artwork and research builds receive explicit local labels. Native target normalization preserves cross-build output lookup. |
| Distribution checks | Version parity, Mach-O architecture and macOS floor are checked. Developer ID, stapling and Gatekeeper checks are explicit; missing checks cannot produce a ready result. Local structural mode always reports `distributionReady: false`. |
| Dependency evidence | Deterministic CycloneDX 1.6 pre-build inventory reports declared, conflicting and unknown licenses without claiming clearance. File reads are bounded, symlinks and unsafe license strings are refused, and output receipts are exclusive. |

## Validation and home-Mac QA

- `make check`: **24/24 lanes passed**, including 58 Python tests, 49 desktop backend tests, 40 portable core tests, seven probe tests and 28 frontend behavior tests. Frontend type/build, formatting, native icon binding/rendering, wire/native checks and inactive FSKit checks passed.
- DriverKit SDK/IIG/link checks and eight ASAN/UBSan suites passed. No module was installed or activated.
- Three optimized local packages passed build and structural inspection: generated-artwork Intel, generated-artwork ARM64 and a private-artwork Intel home preview. All 105 recorded application/build inputs stayed unchanged during validation. All packages remain version `0.2.0`, ad-hoc signed and not distribution-ready.
- Both Intel candidates passed all eight bounded read-only CLI modes on the home Mac. The final private candidate passed all ten read-only SDK MCP tools. Power controls remained disabled while Drobo volumes were mounted. The original DDService remained running and unchanged.
- Computer Use inspected all 14 navigation pages in the generated-artwork Intel candidate. Status automatically selected the sole `Drobo 5D - TB`; five bay indicators rendered green. Capacity and Usage showed approximately 3.75 TB protected used, 7.99 TB free and 11.75 TB usable, with 3.76 TB OS usage and 20.00 TB raw capacity separately labeled. Volumes showed two native-icon HFS+ rows in one enclosure. Charts rendered recorded observations.
- The generated-artwork candidate was visibly resized to the 860 × 640 minimum. Capacity/Usage and Volumes stacked content and scrolled vertically without a page-width overflow in the observed states. Visible keyboard focus and the skip link were observed. Full keyboard, VoiceOver, runtime accessibility preferences and measured performance are not certified; remote pointer/key forwarding was inconsistent.
- The final private-artwork candidate was then launched through Finder. Its All Drobos and automatically selected Status views showed the same exact device label and five reported green indicators. This private package is not a publication artifact.

No device light/brightness/power, firmware, array, format, repair or disk-pack write was sent. No driver installation, FSKit mount, geometry change, sharing export or ACL change occurred. Rendered indicators report original-service observations; they are not independent inspection of physical LEDs.

## Dependency and security assessment

The pre-build inventory contains 518 components: 347 with a declared license and 171 unknown. License clearance and exact shipping-content review are incomplete. Full CycloneDX schema validation was not run; the inventory's audit flags remain false.

A complete OSV query covered 515 third-party public package coordinates. Two locked components matched advisories: `glib@0.18.5` ([RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html), also GHSA-wrw7-89jp-8q8g) and `proc-macro-error@1.0.4` ([RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html), unmaintained). Offline target-specific dependency trees selected neither component for either macOS target in the inspected desktop dependency graph. This is not a claim that the lockfiles are advisory-free. Resolve the matches before enabling affected future platforms; the all-target tree could not complete with the offline cache. No dependency versions were silently changed.

The known low-severity, high-confidence CWE-345 remains open: a local process that acquires the original service's loopback status port while DDService is absent can forge observations and bounded history. `sourceTrust: unverified-loopback` and UI explanations disclose this boundary; they do not authenticate the peer. Optional MCP exposes local read-only metadata to its chosen client and has no MCP network listener, hardware-control tool, arbitrary file reader or shell.

The managed security scan previously failed to save canonical artifacts because an output ancestor was group-writable without the sticky bit. There is no sealed successful report. This audit and the separate dependency receipts do not replace that report or establish that the application is vulnerability-free.

## Remaining v1.0 gates

| Gate | Required next evidence | Status |
| --- | --- | --- |
| Signing and notarization | Developer ID Application identity, notarization, stapled ticket and explicit Gatekeeper acceptance for final artifacts | Blocked: no available Developer ID identity; no submission performed |
| Dependency clearance | Resolve unknown/conflicting licenses, shipping-content review and applicable advisory disposition; validate BOM schema | Open |
| Apple-silicon device transport | Read-only device receipts on an Apple-silicon Mac with the supported Thunderbolt chain | Unverified |
| Spare-unit management | Explicitly scoped disposable hardware tests for advertised light/power controls and recovery from disconnect/stale observations | Unverified |
| Accessibility and QA | Full keyboard/VoiceOver and runtime accessibility-preference checks, remaining critical flows and measured responsiveness | Open |
| Service-source authenticity | Authenticate the legacy status source or make an explicit documented release-risk decision | Unresolved |
| Final release and publication | Accepted version, approved hardware scope, reviewed final artifacts/checksums and authorized public release | Not performed |

Replacement-driver activation, Finder capacity modification, BeyondRAID member assembly/repair and Linux/Windows adapters are outside the presently working product scope. They require their own implementation and hardware acceptance; removing development text does not turn them into supported features.

## Evidence and continuation

Private receipts are ignored under `artifacts/reports/v1-audit/`: `validation.json`, build/preflight receipts, `home-runtime.json`, `mcp-home-smoke.json`, `home-ui-observations.json` and `dependencies/`. Native SDK details are in `artifacts/reports/v1-driverkit-check.log`. Private vendor artwork, host captures, identifiers and package bytes are not published with source.

Use [RELEASING.md](RELEASING.md) for packaging and distribution checks and [SPARE_UNIT_LAB.md](SPARE_UNIT_LAB.md) for the disposable-hardware plan. Complete the open gates before assigning or publishing v1.0.
