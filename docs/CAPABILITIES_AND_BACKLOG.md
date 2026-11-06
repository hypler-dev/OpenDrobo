# Platform capabilities and backlog

This document separates recovered historical behavior from OpenDrobo behavior. It does not claim hardware compatibility.

## Current architecture

| Surface | Current state |
| --- | --- |
| Shared UI | Tauri 2 interface in `desktop/`, dark glass styling, RGB selection glow, selectable unit/drive graphics, capacity pies, ranged observation charts, optional local artwork |
| Portable core | Rust `crates/opendrobo-core`, pure capacity decoding and validation with C ABI |
| macOS transport | C bridge to original loaded Drobo management driver; bounded capacity path plus original-service pool observer |
| macOS filesystem | Swift FSKit extension source and direct checks; separate read-only view, not installed or mounted |
| Linux / Windows | Shared Rust/UI direction; Drobo transport adapters, host-volume adapters, packaging, and hardware tests pending |
| BeyondRAID | Static recovery and research only; no reader, writer, or mount implementation |
| Network management | No host agent, network MCP listener, or remote telemetry; native macOS sharing inspection limited to verified Drobo-mounted paths and settings handoff only |

Original driver remains prerequisite for current macOS transport. Replacement Thunderbolt PCI/DMA or DriverKit storage driver is separate research project.

## Management backlog

1. Validate byte units and model identity on named spare enclosure with read-only captures. Current home evidence is one Drobo 5D-TB on Intel macOS 14 with HFS+ volumes, one protected pool, two LUNs, and an SSD; it is not a universal model profile.
2. Add reviewed per-model capability profile only after repeated captures agree with known geometry.
3. Complete FSKit mount, unmount, disconnect, and Finder comparison tests on spare unit; keep HFS+ geometry unchanged.
4. Implement Linux SCSI-generic and Windows SCSI pass-through adapters behind same Rust contracts, separating software tests from hardware claims.
5. Recover BeyondRAID metadata generations, CRC rules, and block mapping from known member images before recovery UI.
6. Keep reset, format, rebuild, firmware, and array writes closed. Current GUI-only controls are identify, LED presets, brightness read, restart, shutdown, and standby; hardware execution remains untested.
7. Establish authenticated or independently attested local-service identity. Current bounded loopback status parsing does not authenticate the process serving port 4502; a local process that acquires that port while DDService is absent can forge observations and recorded capacity. Keep MCP read-only, expose the unverified source, and do not use these readings to authorize autonomous device operations. See [MCP integration](MCP.md).

Legacy NAS features such as DHCP/static addressing, users, shares, permissions, and alerts belong to model-specific network adapters. Do not infer them for Thunderbolt DAS.

## Platform interface references

The [Linux kernel SCSI-generic documentation](https://www.kernel.org/doc/html/latest/scsi/scsi-generic.html) and [Microsoft SCSI pass-through documentation](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddscsi/ni-ntddscsi-ioctl_scsi_pass_through) establish available OS interfaces, not Drobo compatibility. Modern USB variants may differ. The original [drobo-utils README](https://github.com/petersilva/drobo-utils/blob/main/README.rst) documents older Linux/model paths, later protocol gaps, broken diagnostics/firmware features, and lack of current hardware testing. It is useful reference evidence, not a compatibility certificate. Its GPL code has not been copied into this MIT project.

The historical [5D3 vendor user guide](https://content.etilize.com/User-Manual/1040150862.pdf) includes Windows Dashboard installation and USB/Thunderbolt interfaces; it does not prove that OpenDrobo or every connection works on a current Windows release. Current home evidence identifies one 5D-TB unit, but does not establish broader 5D-class compatibility.

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

These are proposed slices, not promised delivery dates. Current app provides registry discovery, protected-pool observation, Drobo-bound host-volume statistics, bounded console/events, five-minute local capacity history with range, utilization, allocation-change and observation-gap charts, native sharing inspection limited to Drobo-mounted exports, read-only MCP, and GUI control availability. GUI control writes are source-implemented and mock-tested but hardware-untested. Other array providers and generic host-volume inventory remain future adapter work. No service installation, network port exposure, or remote telemetry was introduced. Original DDService remains owner of the management connection; OpenDrobo does not stop it.

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

## Two-enclosure host RAID assessment

Thunderbolt chaining does not merge two enclosures into one BeyondRAID pool. A host-managed mirror or stripe across separate Drobos is an untested architecture option, not current OpenDrobo support. Thin-provisioned logical sizes, actual protected capacities, stable member identity, virtual-member ancestry, mount guards, disconnect recovery and power safety need a separate spare-unit test plan. No raw-member passthrough mode has been verified. See [firmware capabilities](FIRMWARE_CAPABILITIES.md#beyondraid-filesystems-and-appleraid).

## Spare-unit experiment boundary

The user has a separate enclosure and spare drives. Before a destructive experiment, name the enclosure model/firmware, every member drive, the host and adapter chain, imaging destination, test data, and permitted changes. Capture read-only member images and hashes for each known state. Preserve original generations and compare reconstructed files byte-for-byte. Availability of spare drives does not identify which disks are safe to erase.
