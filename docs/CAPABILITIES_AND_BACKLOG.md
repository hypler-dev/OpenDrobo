# Platform support, network management, and improvements

Research date: November 4, 2026. This records legacy capabilities and proposed OpenDrobo work. It does not claim those capabilities are implemented. Current app behavior is in [the work record](WORK_RECORD.md).

## Linux and Windows direction

Linux and Windows are intended targets alongside macOS. They are not supported builds today. The existing app imports AppKit, SwiftUI, and IOKit and builds only a macOS 26 universal binary. [Swift supports Linux and Windows](https://www.swift.org/platform-support/); [Apple's SwiftUI](https://developer.apple.com/swiftui/) targets Apple platforms. Language portability does not make this complete application portable.

The proposed first step is to implement byte parsing, status interpretation, metadata validation, and eventual read-only BeyondRAID block mapping in a portable core. That core must accept byte buffers or read-only image providers and return records/errors without importing UI, registry, driver, or filesystem-mount APIs. Swift is a candidate because the current app uses it, subject to actual Linux/Windows builds and shared fixture tests. A language change is not required by the current evidence. No device-protocol parser ships in the app today.

| Surface | macOS today | Linux / Windows path |
| --- | --- | --- |
| Desktop interface | Native dark/glass SwiftUI preview | Separate frontend or portable shell; reproduce the visual design with platform-appropriate effects. Apple glass APIs are not a portable dependency |
| Offline firmware inventory | Python tools tested locally | Candidate portable tooling; run archive/ELF tests on each target before claiming support |
| Device protocol records | Registry discovery only; no protocol parser or management transport | Implement shared pure record parsing; keep platform discovery and command adapters separate |
| USB management | Unimplemented in this app | Linux SCSI-generic and Windows SCSI pass-through are candidate OS facilities; require a compatible model, exposed device path, permissions, and verified commands |
| NAS management | Proposed only | Shared network-protocol implementation after framing/authentication are recovered; test each NAS model/firmware |
| Remote view of home-Mac Drobo | Proposed authenticated agent | A Windows/Linux client could consume verified agent records without a local Thunderbolt driver; home Mac must remain online |
| Direct Thunderbolt storage | Original driver under static study; no replacement | Separate PCI/DMA/SCSI transport work on each OS; enumerating a Thunderbolt device is not storage or management support |
| BeyondRAID recovery from member images | Static findings; no reader yet | Portable read-only block mapper and identical known-image tests on all targets; filesystem interpretation/mounting remains a separate layer |

The [Linux kernel SCSI-generic documentation](https://www.kernel.org/doc/html/latest/scsi/scsi-generic.html) and [Microsoft SCSI pass-through documentation](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddscsi/ni-ntddscsi-ioctl_scsi_pass_through) establish available OS interfaces, not Drobo compatibility. Modern USB variants may differ. The original [drobo-utils README](https://github.com/petersilva/drobo-utils/blob/main/README.rst) documents older Linux/model paths, later protocol gaps, broken diagnostics/firmware features, and lack of current hardware testing. It is useful reference evidence, not a compatibility certificate. Its GPL code has not been copied into this MIT project.

The historical [5D3 vendor user guide](https://content.etilize.com/User-Manual/1040150862.pdf) includes Windows Dashboard installation and USB/Thunderbolt interfaces; it does not prove that OpenDrobo or every connection works on a current Windows release. The user's exact enclosure remains unidentified.

Delivery order: portable parser/core plus fixture CLI, target-OS builds/tests, USB/NAS or authenticated host-agent status, platform frontends, then separately validated direct Thunderbolt transports. Any new CI matrix must label software tests separately from hardware compatibility. No Linux or Windows runner, binary, driver, installer, or hardware test was added in this slice.

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

These are proposed slices, not promised delivery dates. The current app only performs local registry discovery and optional GitHub release checks. No home-Mac access, service installation, network port exposure, hardware command, or remote telemetry was introduced by this work.

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
