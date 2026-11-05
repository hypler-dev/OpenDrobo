# Platform capabilities and backlog

This document separates recovered historical behavior from OpenDrobo behavior. It does not claim hardware compatibility.

## Current architecture

| Surface | Current state |
| --- | --- |
| Shared UI | Tauri 2 interface in `desktop/`, dark glass styling, RGB selection glow, optional local artwork |
| Portable core | Rust `crates/opendrobo-core`, pure capacity decoding and validation with C ABI |
| macOS transport | C bridge to original loaded Drobo management driver; bounded read-only MODE SENSE path only |
| macOS filesystem | Swift FSKit extension source and direct checks; separate read-only view, not installed or mounted |
| Linux / Windows | Shared UI/core direction only; device transport, filesystem adapter, packaging, and hardware tests pending |
| BeyondRAID | Static recovery and research only; no reader, writer, or mount implementation |
| Network management | Proposed authenticated host agent; no service, port, or home-Mac connection exists |

Original driver remains prerequisite for current macOS transport. Replacement Thunderbolt PCI/DMA or DriverKit storage driver is separate research project.

## Management backlog

1. Validate byte units and model identity on named spare enclosure with read-only captures.
2. Add reviewed per-model capability profile only after repeated captures agree with known geometry.
3. Complete FSKit mount, unmount, disconnect, and Finder comparison tests on spare unit; keep HFS+ geometry unchanged.
4. Add authenticated home-Mac read-only status agent only after pairing, credential, retention, and exposure design is accepted.
5. Implement Linux SCSI-generic and Windows SCSI pass-through adapters behind same Rust contracts, separating software tests from hardware claims.
6. Recover BeyondRAID metadata generations, CRC rules, and block mapping from known member images before recovery UI.
7. Keep firmware, restart, shutdown, format, rebuild, and array writes closed until operation-specific safety and rollback tests exist.

Legacy NAS features such as DHCP/static addressing, users, shares, permissions, and alerts belong to model-specific network adapters. Do not infer them for Thunderbolt DAS.

## Platform interface references

The [Linux kernel SCSI-generic documentation](https://www.kernel.org/doc/html/latest/scsi/scsi-generic.html) and [Microsoft SCSI pass-through documentation](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddscsi/ni-ntddscsi-ioctl_scsi_pass_through) establish available OS interfaces, not Drobo compatibility. Modern USB variants may differ. The original [drobo-utils README](https://github.com/petersilva/drobo-utils/blob/main/README.rst) documents older Linux/model paths, later protocol gaps, broken diagnostics/firmware features, and lack of current hardware testing. It is useful reference evidence, not a compatibility certificate. Its GPL code has not been copied into this MIT project.

The historical [5D3 vendor user guide](https://content.etilize.com/User-Manual/1040150862.pdf) includes Windows Dashboard installation and USB/Thunderbolt interfaces; it does not prove that OpenDrobo or every connection works on a current Windows release. The user's exact enclosure remains unidentified.

## Legacy capabilities depend on model

The vendor-authored [5N Getting Started Guide, preserved PDF](https://images-na.ssl-images-amazon.com/images/I/A1fn47V9u8L.pdf) describes Ethernet LAN use, DHCP with optional static addressing, and Dashboard discovery. The vendor-authored [5N2 User Guide, preserved mirror](https://www.manualslib.com/manual/1300917/Drobo-5n2.html) documents network settings, dual-interface configuration, users/shares, permissions, alerts, and diagnostics. These are historical manuals, not current support statements.

| Capability | Legacy support shown by manuals | OpenDrobo implication |
| --- | --- | --- |
| LAN discovery and manual IP entries | Network Drobos such as 5N/5N2 | A separate NAS discovery adapter; not proof that Thunderbolt units speak the same network protocol |
| DHCP/static IP and network configuration | 5N/5N2; dual-NIC independent/bonded operation and MTU are 5N2-specific | Expose only model-confirmed settings, with read-only inspection before mutation |
| Users, shares, guest access, per-share none/read-only/read-write permissions | NAS firmware and Dashboard | Preserve distinction between NAS permissions and macOS sharing of a mounted DAS volume |
| Health, capacity, protection-state alerts | Dashboard status and on-screen/email notification features | Recover reliable records first; expose freshness and unknown states |
| Diagnostic export | Dashboard support workflow | Add a local, previewable export with explicit redaction rules |
| DroboApps | Preserved applications for 5N/5N2/B810n in the [community archive](https://github.com/dhomas1/DroboApps/blob/main/README.md) | Separate NAS application support from DAS management and firmware work |

The original GUI alone does not make a Thunderbolt Drobo into a NAS. For the user's enclosure, a home-Mac agent would read local device status and offer an authenticated management connection while that Mac is online. File sharing would remain a separate host-OS capability. Neither an agent nor a listening network endpoint exists in OpenDrobo today.

Static `DDService64d` inspection found OpenSSL-related symbols. This does not establish that discovery or management traffic is encrypted or that a remote interface is safe. Recover message framing and authentication separately before implementing a compatible network client.

## Proposed management path

1. **Local visibility:** show enumeration, transport readiness, management availability, record age, and actionable failure messages. Unknown health remains unknown.
2. **Local history and alerts:** record connection changes, verified health/capacity snapshots, protection transitions, and notification delivery. Stale or disconnected records remain visibly distinct.
3. **Authenticated home-Mac agent:** read-only status, history, and user-approved diagnostic export; deliberate pairing and encrypted transport; no implicit public exposure. Agent lifecycle, access control, retention, and credential storage require a separate accepted design.
4. **Multiple hosts and NAS adapters:** one device inventory with per-model capability reporting. NAS settings/users/shares remain separate from Thunderbolt transport.
5. **Management writes:** only after read paths and response semantics are verified on the named spare enclosure. Restart, shutdown, settings, firmware, formatting, and array changes need operation-specific safeguards and validation.

These are proposed slices, not promised delivery dates. The current app provides registry discovery, optional GitHub release checks, and an explicit bounded read-only capacity query through the original management driver. This workspace has no attached enclosure, so no device command was submitted here. No home-Mac access, service installation, network port exposure, or remote telemetry was introduced.

## Community evidence and corresponding work

Reports below are attributed symptoms, not reproduced root causes. Repository feature requests are identified as such. Hardware, adapters, OS builds, firmware, and previous settings can change outcomes.

| Evidence | Reported symptom / request | Bounded improvement and required proof |
| --- | --- | --- |
| [drobo-utils issue 7](https://github.com/petersilva/drobo-utils/issues/7), January 2022 | Drobo 8D firmware 4.2.3 on Linux: Thunderbolt/PCI components visible without a usable storage device; discussion does not settle cause | Separate enumeration, transport, storage, and management states. Verify each on the exact target; Linux symptoms do not prove a macOS fix |
| [5C/Sonoma report](https://www.reddit.com/r/drobo/comments/16zv74k/), October 2023 | Missing volume after reboot, reported recovery after a power cycle | Create cold-boot/reboot/sleep/replug matrix with logs and integrity checks on spare hardware; do not automate power cycles as a cure |
| [Mini/Thunderbolt report](https://www.reddit.com/r/drobo/comments/1il01vj/), February 2025 | Reported kernel crash with an adapter chain; no validated panic attribution here | Collect transport/power-transition diagnostics and exact adapter/OS/driver versions; avoid attributing a crash from a brief symptom report |
| [5D/macOS 15.4 discussion](https://www.reddit.com/r/drobo/comments/1jpcq8t/), April–May 2025 | Dashboard/driver discovery difficulties and user-reported workarounds | Publish evidence-based compatibility matrix; do not present security-setting changes as a verified or universal solution |
| [Recovery integrity report](https://www.reddit.com/r/drobo/comments/1d51f8t/), May–June 2024 | Recoverable directory/file names with unreadable media; later discussion involves all members | A BeyondRAID reader must prove reconstructed file bytes and member/generation selection, not merely list a plausible directory |
| [DroboBridge issue 1](https://github.com/ringo380/DroboBridge/issues/1), January 2026 | Maintainer feature request for direct USB health/bay/capacity/rebuild information; not a reproduced device failure | Reuse confirmed protocol records behind a capability interface; keep USB, Thunderbolt PCI, and NAS paths separate |

The [static recovery notebook](../research/STATIC_RECOVERY.md) now identifies handshake, power-transition, index-version, CRC, and map-validation targets relevant to those slices. It does not establish that any reported issue has been fixed.

## Spare-unit experiment boundary

The user has a separate enclosure and spare drives. Before a destructive experiment, name the enclosure model/firmware, every member drive, the host and adapter chain, imaging destination, test data, and permitted changes. Capture read-only member images and hashes for each known state. Preserve original generations and compare reconstructed files byte-for-byte. Availability of spare drives does not identify which disks are safe to erase.
