# Release preparation

The canonical release decision and hardware scope are recorded in [WORK_RECORD.md](WORK_RECORD.md); open gates appear in [V1_READINESS.md](V1_READINESS.md). The following tools prepare and inspect artifacts. They do not authorize publication or establish hardware support.

## Source and toolchain

Use tracked Rust 1.98.0 and Node 24.22.1 pins. The validated build host uses Xcode 26.3, build 17C529, and Python 3.11. The workspace declares Rust 1.85 as its minimum; that minimum has not been independently certified by this audit.

Before packaging, verify a clean, reviewed source checkpoint, version parity and the named checks:

```sh
git status --short --branch
make check
python3 scripts/dependency_inventory.py --outdir artifacts/release-evidence/dependencies
```

The dependency tool emits a deterministic CycloneDX 1.6 pre-build inventory of locked dependencies, not an assertion about exact shipping contents. Declared, conflicting and unknown licenses remain visible. Its audit flag stays false; complete the license and shipping-content review separately. Full schema validation is not claimed by the generator. Existing inventory receipts are not overwritten.

Desktop package, Tauri, Cargo and both npm lockfile version entries must match a canonical numeric `X.Y.Z`. Update them together for a release. Build receipt, archive name and updater User-Agent derive from checked version metadata. No v1.0 version is assigned merely because packaging checks pass.

## Local candidates

Run each architecture build on its matching native Mac, or provision the exact Rust standard-library target and compatible Apple toolchain before cross-compiling. The minimal Rust toolchain pin does not install cross targets automatically.

```sh
python3 scripts/build_desktop.py --target x86_64-apple-darwin --output dist/candidate-intel
python3 scripts/build_desktop.py --target aarch64-apple-darwin --output dist/candidate-arm64
python3 scripts/release_preflight.py dist/candidate-intel/OpenDrobo.app --local-structural --output artifacts/release-evidence/intel-local.json
```

Default packages use the release profile, generated artwork and no filesystem extension. Ad-hoc candidates carry `local` in their archive name. `--debug` produces a separately labeled debug artifact. `--local-artwork` opts into ignored vendor artwork only for a private build, also labeled local. Public artifacts must exclude it.

The inactive filesystem research extension requires explicit `--with-fskit-research`, a native Apple-silicon host and macOS 26. Its archive is separately labeled, its app launch floor becomes macOS 26, and it fails normal macOS-14 distribution preflight. Neither that build nor the DriverKit checks install, register or activate a driver or mount a filesystem.

Local structural mode always reports `distributionReady: false`. Read every gate; its process exit status alone does not establish structural or public-release readiness.

## Distribution gates

A public macOS package needs an explicit Developer ID Application identity, nested-code signing, notarization, stapling and Gatekeeper validation. The builder never selects a signing identity automatically. An explicitly supplied Developer ID enables hardened-runtime/timestamp signing, but the build receipt still starts with `distributionReady: false`.

Use Apple's current [notarization workflow](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) to submit the candidate and staple the accepted result. Configure credentials locally through the keychain; do not put them in repository files, logs or command examples. No notarization submission or credential operation is performed by OpenDrobo's build/preflight tools.

After notarization and stapling, run both explicit artifact checks:

```sh
python3 scripts/release_preflight.py /path/to/OpenDrobo.app \
  --check-stapler --check-spctl \
  --output artifacts/release-evidence/distribution-preflight.json
```

Public preflight requires version parity, release profile, exact declared macOS-14 floor, declared target/architecture matching the inspected Mach-O, valid Developer ID signing, no research module, no local vendor artwork, a valid stapled ticket and Gatekeeper acceptance. Missing or failed checks stay false. A passing artifact receipt does not establish hardware certification, license clearance, application QA or publication authority.

Re-create the ZIP after stapling and regenerate its SHA-256 manifest, because stapling can change package bytes. Retain the notarization log and final artifact/preflight receipts. The initial builder manifest describes its initial archive only. Match final version, architecture, tag and checksums before any authorized upload. These tools do not publish a tag, GitHub release, installer or application update.

## Hardware and QA acceptance

Release claims must match observed model, firmware, macOS, adapter chain and architecture. Current evidence covers one Intel 5D Thunderbolt path. Apple-silicon connectivity, additional enclosures and live control operations require their own receipts. Driver replacement, FSKit/Finder statistics and BeyondRAID assembly remain closed research gates.

Complete default/minimum-window and keyboard/VoiceOver checks, real disconnect/stale behavior and all critical user flows. Test destructive scenarios only against the named disposable lab unit and disks under their separate accepted execution plan.
