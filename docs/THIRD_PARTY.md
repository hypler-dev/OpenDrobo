# Third-party notices and license review

OpenDrobo source is MIT licensed. The notice generator creates a bounded review receipt for selected macOS dependencies. It does not establish exact shipping contents, license compatibility, patent status, trademark permission or legal clearance.

## Generate review material

Capture each macOS graph with default features, the pinned desktop tool environment and the locked desktop manifest. Use a fresh private evidence directory; output paths are exclusive.

```sh
mkdir -p artifacts/release-evidence/licenses
CARGO_HOME="$PWD/tools/cargo-home" cargo metadata --offline --locked --format-version 1 \
  --filter-platform aarch64-apple-darwin --manifest-path desktop/src-tauri/Cargo.toml \
  > artifacts/release-evidence/licenses/cargo-arm64.json
CARGO_HOME="$PWD/tools/cargo-home" cargo metadata --offline --locked --format-version 1 \
  --filter-platform x86_64-apple-darwin --manifest-path desktop/src-tauri/Cargo.toml \
  > artifacts/release-evidence/licenses/cargo-intel.json
python3 scripts/dependency_licenses.py \
  --metadata aarch64-apple-darwin=artifacts/release-evidence/licenses/cargo-arm64.json \
  --metadata x86_64-apple-darwin=artifacts/release-evidence/licenses/cargo-intel.json \
  --npm-runtime @tauri-apps/api \
  --outdir artifacts/release-evidence/licenses/notices
```

The CLI reruns both exact default-feature `cargo metadata --offline --locked` commands through `desktop_tools.environment`, compares canonical package IDs, dependency edges, kinds, enabled features, and default workspace members, then records public command and tool-version text. Supplied metadata is otherwise operator input, rather than build attestation. A production receipt requires `defaultGraphVerified: true`.

For each selected Cargo registry package, the generator binds the name, version, source and checksum to `desktop/src-tauri/Cargo.lock`; reads matching `.crate` bytes from `tools/cargo-home/registry/cache`; verifies SHA-256; then reads only bounded top-level regular `Cargo.toml`, `.cargo_vcs_info.json`, `LICENSE*`, `LICENCE*`, `COPYING*`, and `NOTICE*` members from the archive. It validates the embedded package identity and compares declared license with metadata. It never trusts extracted Cargo source trees.

For each explicit UI runtime package, the generator requires the exact version in both production `dependencies` sections of `desktop/package.json` and `desktop/package-lock.json`, verifies the locked SHA-512 integrity against the npm content-addressed cache, and reads terms only from the verified tarball. It never trusts `node_modules`.

All regular files and archive members are size- and count-bounded. Archive traversal, symlink, hardlink, non-regular member, invalid identity, archive checksum/integrity mismatch, declaration mismatch, duplicate target metadata, or missing target fails closed. The manifest records SHA-256 values for Cargo.lock, package-lock.json, and package.json.

`NOTICE*` files are retained as attribution material but never satisfy license-terms coverage alone. `noticeReady` means every selected package has matching declared license metadata and documentary license-terms coverage. `legalReviewRequired` is always true.

When a verified crate archive has no local terms file, reviewed supplemental JSON may provide an original file after its `originCommit` matches the archive's `.cargo_vcs_info.json`. Each record binds the ecosystem, package version, source URL containing an immutable 40-character commit, text and SHA-256. Supplements cannot replace verified local terms. Supplemental `NOTICE*` material remains attribution-only and cannot satisfy terms coverage. The generator does not fetch URLs, accept branch URLs or substitute generic SPDX text.

## Review gates

Retain the generated JSON, notices, graph receipt, input hashes and final artifact receipt. A human reviewer must check each declaration, terms file, copyright/attribution notice, distribution condition, mismatch, missing terms, and whether build-only packages ship. Build-path presence is provenance only; it does not claim distribution.

The all-lock CycloneDX inventory remains discovery material. It includes packages outside the reviewed macOS graphs and does not establish the final shipped-notice list.

## Vendor material

Drobo being defunct does not grant a license to software, firmware, drivers, extracted artwork, documentation, trademarks, or name. Copyright can persist and transfer. OpenDrobo public builds exclude vendor binaries, firmware, and extracted artwork. Use independently created or separately licensed replacement artwork; a new rendering alone does not establish every applicable right. Do not represent OpenDrobo as affiliated with Drobo.

The [U.S. Copyright Office explains that copyright ownership can transfer](https://copyright.gov/help/faq/faq-assignment.html), and ownership of a copy does not itself transfer copyright. See [rights and distribution research](RIGHTS_AND_DISTRIBUTION.md) for the ownership lead and practical release approach. Review the finished public material and obtain a written grant when relying on vendor material that requires permission.
