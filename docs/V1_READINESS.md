# v1.0 readiness audit

The application quality audit and actionable release-preparation follow-up are complete for the scope below. **Public v1.0 release remains blocked.** This document records evidence and remaining gates; it is not a release or security certificate. [WORK_RECORD.md](WORK_RECORD.md) remains the canonical accepted scope and publication record.

## Scope and supported evidence

The audit began at `ee9f9fd68ab68f20372d4e91973fc41e7c0f1cfc` and covered the shared interface, Rust runtime, native storage paths, portable decoders, tests, packaging, dependencies, documentation and product claims. Independent read-only reviews covered interface, backend, native/core and release tooling. A follow-up from `170d8ad1273ec6c4f1b8fb26165ba9501d087d5e` adds checksum-bound dependency notices, package notice gates, newly rendered artwork and signing/hardware/rights handoffs. Resulting source fixes passed fresh review and validation.

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
| Packaging | Default builds are optimized, use newly rendered OpenDrobo imagery and exclude inactive filesystem research. Live enclosure graphics remain data-driven. The icon master stays outside the frontend bundle, and application/third-party licenses are packaged before signing. Debug, private artwork and research builds receive explicit local labels. |
| Distribution checks | Version parity, Mach-O architecture and macOS floor are checked. Developer ID, stapling and Gatekeeper checks are explicit; missing checks cannot produce a ready result. Local structural mode always reports `distributionReady: false`. |
| Dependency evidence | The target-selected collector verifies Cargo SHA-256 and npm SHA-512 archive bytes, reproduces both default macOS graphs, preserves terms/NOTICE distinctions and records current lock/package hashes. Packaging checks notice hashes and architecture coverage; absent or incomplete notices block distribution. The refreshed CycloneDX 1.6 inventory passes official schema validation. None of these checks grants legal approval. |

## Validation and home-Mac QA

- `make check`: **24/24 lanes passed**, including 77 Python tests, 49 desktop backend tests, 40 portable core tests, seven probe tests and 28 frontend behavior tests. The full 77-test Python suite was repeated after the final test-only review correction and passed. Frontend type/build, formatting, native icon binding/rendering, wire/native, DriverKit and inactive FSKit checks passed.
- DriverKit SDK/IIG/link checks and eight ASAN/UBSan suites passed. The current `OpenDroboStorage.dext` executable is ARM64. No module was installed or activated; its hardware gate remains disabled.
- Two optimized local packages passed build and structural inspection, one Intel and one ARM64. All 112 recorded application/package inputs stayed unchanged during validation. Both include the OpenDrobo license and a checksum-bound partial third-party notice bundle. The packages remain version `0.2.0`, ad-hoc signed and not distribution-ready. Signature identity, stapling/Gatekeeper and incomplete-notice gates remain false.
- The new Intel candidate passed all eight bounded read-only CLI modes and all ten SDK MCP tools on the home Mac. One enclosure, one pool, two HFS+ volumes and six slots remained distinct. Power controls remained disabled with mounted volumes. Original DDService remained running and unchanged.
- Computer Use launched the new candidate through Finder and observed its new native icon, faded three-quarter artwork, automatic single-unit Status selection and five green decoded bay indicators. Clicking Drive 3 displayed its metadata. Capacity and Usage retained approximately 3.75 TB protected used, 7.99 TB free and 11.75 TB usable, with 3.76 TB OS usage and 20.00 TB raw capacity separately labeled. Volumes retained two native-icon rows grouped to one enclosure. Tools remained without a control target after read-only display selection.
- The new candidate was visibly resized to 860 × 640; the observed Volumes rows stacked without page-width overflow. The default 1120 × 800 window was restored and left on System Information. The earlier audit inspected all 14 pages and additional compact scrolling states; this follow-up did not repeat every page. Full keyboard/VoiceOver, runtime accessibility preferences, measured performance, multiple enclosures and live fault/disconnect transitions remain uncertified.

Decorative artwork contains green bay lights and blue capacity lights as requested; it is aria-hidden and does not report hardware state. Active enclosure indicators use the decoded current/stale observations. No device light/brightness/power, firmware, array, format, repair or disk-pack write was sent. No driver installation, FSKit mount, geometry change, sharing export or ACL change occurred.

## Dependency and security assessment

The refreshed all-lock pre-build inventory contains 518 components: 492 with a recognized declared license and 26 unresolved by that collector. Many remaining entries involve legacy expression syntax, the collector's bounded metadata policy or project packages; these are not 26 findings that permission is absent. The earlier audit's 171 unknown entries largely reflected incomplete local cache metadata. The refreshed inventory passes offline validation against the saved official CycloneDX 1.6, SPDX and JSF schemas; its legal-audit flags remain false.

The narrower default macOS/UI graph has 290 components, all with matching declared license metadata. There are 492 collected local/upstream notice documents, with documentary terms coverage for 277 components. Thirteen entries need manual terms review: `selectors@0.38.0` and twelve objc2-family crates. This is a collection/compliance gap, not a finding that those packages have no license grant. `plist@1.10.3` is covered by its verified `LICENCE` file. The generator correctly reports `noticeReady: false` and `legalReviewRequired: true`. See [THIRD_PARTY.md](THIRD_PARTY.md) for exact-byte binding and [RIGHTS_AND_DISTRIBUTION.md](RIGHTS_AND_DISTRIBUTION.md) for obligations, MPL source access and vendor-rights research. No dependency versions were changed.

A complete OSV query covered 515 third-party public package coordinates. Two locked components matched advisories: `glib@0.18.5` ([RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html), also GHSA-wrw7-89jp-8q8g) and `proc-macro-error@1.0.4` ([RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html), unmaintained). Offline target-specific dependency trees selected neither component for either macOS target in the inspected desktop dependency graph. This is not a claim that the lockfiles are advisory-free. Resolve the matches before enabling affected future platforms; the all-target tree could not complete with the offline cache. No dependency versions were silently changed.

The known low-severity, high-confidence CWE-345 remains open: a local process that acquires the original service's loopback status port while DDService is absent can forge observations and bounded history. `sourceTrust: unverified-loopback` and UI explanations disclose this boundary; they do not authenticate the peer. Optional MCP exposes local read-only metadata to its chosen client and has no MCP network listener, hardware-control tool, arbitrary file reader or shell.

Read-only process/socket research found useful kernel PID/path visibility but normal application permissions could not inspect the root-owned service descriptors or BSD process information. No dependable production peer-verification design was established, no privileged helper was installed and no authentication claim was added.

The managed security scan previously failed to save canonical artifacts because an output ancestor was group-writable without the sticky bit. There is no sealed successful report. This audit and the separate dependency receipts do not replace that report or establish that the application is vulnerability-free.

## Remaining v1.0 gates

| Gate | Required next evidence | Status |
| --- | --- | --- |
| Signing and notarization | Apple Developer membership, publisher identity, Developer ID Application certificate, notarization and final stapler/Gatekeeper proof | Blocked: owner has no membership yet; no purchase, credentials or submission performed |
| Dependency compliance | Resolve thirteen documentary terms gaps, applicable license/source-access duties and actual shipped-content review | Open: 277/290 documentary coverage; schema validation now passed |
| Apple-silicon device transport | Read-only device receipts on a named Apple-silicon Mac/macOS and Thunderbolt chain | Unverified; package compilation only |
| Independent Thunderbolt driver | Approved provisioning, implemented/reviewed activation and actual initialization/DMA/interrupt/read-only I/O proof | Inactive compiled prototype; not a working replacement |
| Spare-unit management | Identify the separate disposable 5D/pack and execute a separately accepted control/disconnect plan | Spare model confirmed; hardware execution pending |
| Accessibility and QA | Full keyboard/VoiceOver, runtime accessibility-preference checks, remaining critical flows and measured responsiveness | Open; default/minimum visual smoke passed |
| Service-source authenticity | A dependable authenticated peer design or explicit release-risk disposition | Unresolved; warning and optional read-only MCP retained |
| Public identity/assets | Review final compatibility naming, imagery and affiliation statements; obtain permission when relying on material requiring it | Newly rendered assets replace extracted images; no blanket rights clearance claimed |
| Final release and publication | Accepted version/support scope, reviewed final artifacts/checksums and authorized binary release | Not performed |

The working dashboard still requires the original driver. Advertising independent Thunderbolt support on newer macOS/Apple silicon requires the replacement-driver gate above; an application-signing certificate cannot satisfy it. Finder capacity modification, BeyondRAID member assembly/repair and Linux/Windows adapters also require their own implementation and hardware acceptance.

## Evidence and continuation

Private follow-up receipts are ignored under `artifacts/reports/v1-followup/`: `validation.json`, Intel/ARM64 build/preflight receipts, final Python output, `home-runtime.json`, `mcp-home-smoke.json`, `home-ui-observations.json` and final process proof. Dependency/official-schema receipts are under `artifacts/reports/v1-license-clearance/`, with the actual package notice bundle in `notices-v7/`. Earlier audit evidence remains under `artifacts/reports/v1-audit/`. Private captures, identities, vendor files and package bytes are not published with source.

Use [RELEASING.md](RELEASING.md) for enrollment/signing and distribution checks, [HARDWARE_VALIDATION.md](HARDWARE_VALIDATION.md) for what the owner must provide and the ordered test sequence, and [SPARE_UNIT_LAB.md](SPARE_UNIT_LAB.md) for the offline manifest. Complete the relevant open gates before assigning or publishing v1.0.
