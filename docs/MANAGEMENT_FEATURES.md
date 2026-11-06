# Management features

Interface keeps product state, host state, and command availability visible at same time. Every unit and drive can be shown as present/active, absent/inactive, stale, or unknown. Raw numeric status fields remain available where original service supplies them; OpenDrobo decodes the recovered drive-LED selector separately from presence; remaining undocumented bit masks do not become reassuring health labels.

## Product surface

The main views are:

- **All Drobos:** primary physical-unit inventory, brand/model, connection, active/inactive state, and selection.
- **Status:** automatic single-unit display or selectable enclosure icons for multiple units, visible System Information and Drive Information buttons, enclosure state, connection freshness, disk-slot presence/front-drive-bay state, raw unit/slot state, SSD capacity/life, and last observation time. The 5D front-bay indicators support reported green, amber, red, alternating green/amber and flashing red codes, with text labels and gray stale/unknown states. This is not a complete health decoder. Temperature is not currently exposed by the app.
- **Capacity:** protected pool total/used/free, separate OS-used comparison, an explicit Usage breakdown with raw installed capacity kept secondary, pool freshness, and a usable-capacity pie. The 64 TiB logical ceiling stays Advanced metadata, never protected capacity.
- **Statistics:** selectable time ranges, protected/free/OS-used trends, allocation changes, utilization, recording gaps, current drive capacities, SSD life and retained session-event counts.
- **Volumes:** compact rows with native macOS volume icons, name, filesystem, mount path, access and filesystem-used bytes. Volumes group by native physical-enclosure ancestry, never by matching names. Logical totals and available bytes remain in labeled details. BeyondRAID does not expose a volume-to-individual-bay assignment. Non-Drobo volumes are not shown.
- **Events:** bounded observed-since-process-start history for connection changes, slot changes, capacity changes, SSD-life changes, service errors, and numeric state changes. This is not recovered historical onboard event-log data.
- **Console:** bounded application records with level, source, operation, message, pause, filtering, refresh, and copy controls.
- **Sharing:** native SMB/NFS service inspection, SMB exports whose configured paths fall under verified Drobo mount roots, and a handoff to macOS File Sharing settings. This metadata classification does not resolve symlink targets or grant file access. User accounts, folder permissions, and ACLs stay in the operating system.
- **Tools / Reference:** original operation ordering and a separate command catalogue with explicit availability and safety state.
- **Drobo Settings:** the selected device's verified settings profile. The exact 5D profile presents light settings using the matching control target. NAS network/account/share forms are not shown for this Thunderbolt enclosure. Unknown models receive an explicit unavailable-profile state.

Navigation uses three always-visible groups: Device; Activity; and Host & application. Events, Statistics, Console, native host Sharing, MCP and Reference require no Advanced disclosure. Status component choices are compact visible buttons rather than a full-width dropdown. Status may automatically choose a sole reported pool for display. Model-specific settings still require a separately selected matching control target; display selection does not grant control authority. Original NAS settings remain research references, not fake forms on an unrelated DAS model. This is an adaptation of the original Dashboard hierarchy, not a claim of complete functional parity.

Status and capacity cards use device/drive graphics with allocated, used, and free SVG donut segments. A 64 TiB logical ceiling remains technical metadata, not protected-pool capacity. Raw status and other implementation fields stay in closed technical-detail disclosures. Those disclosures do not hide application navigation or component choices.

Capacity history is a new OpenDrobo-only series. It records fresh pool observations as decimal-string `total`, `used`, `free`, and `osUsed` values with timestamp, opaque unit ID, and model. The private per-user `capacity-history.jsonl` file is coalesced to the newest sample in each five-minute interval per pool, 2,048 records globally, and 1 MiB, using secure openat/flock handling. Statistics offers 1-hour, 6-hour, 24-hour, 7-day and all-recorded ranges; protected used/free, OS-used, allocation-change and utilization charts; observation quality; current drive-capacity and SSD-life summaries; and bounded session-event counts. Byte arithmetic uses BigInt. Observed average allocation change requires three readings over one hour with stable total capacity and is not a forecast. Charts require two real observations; an empty chart says “collecting” and never backfills fabricated onboard history.

## Original management functions

Original application contains commands for identify, dimming, restart, standby, shutdown, reset, firmware/status reporting, volume operations, notifications, and device preferences. OpenDrobo lists this full catalogue with availability, research, or disabled state; it does not claim full original parity. Name, redundancy, spindown and NAS configuration readback/write contracts remain unavailable; the app does not present those as operational settings.

Current light controls use fixed presets 0..9 mapped to recovered raw levels `1, 2, 5, 7, 9, 11, 15, 25, 40, 59`, plus a read-only brightness query. Identify/light actions require the original service binary hash to match the known build and fresh SHA-256 internal target binding. Power actions are restart, shutdown, and standby only. They require uppercase typed confirmation, all identified Drobo volumes (including custom paths) to be unmounted, and original force flags to remain false; the service performs its own unmount checks. No live light or power write test has been performed.

Reset, format, erase, firmware update, rebuild, arbitrary XML, and disk-pack mutation remain disabled. Visible menu entry is not proof that command is executable.

## Events and state

Events are generated from bounded service observations and host lifecycle records. Repeated identical snapshots are coalesced. A single failed read does not become false disconnect; UI reports stale or read-error state until fresh observation resolves it. Event records exclude serial numbers, credentials, raw XML, filesystem contents, and arbitrary command arguments.

## MCP

MCP is local stdio interface, not network management daemon. It exposes ten fixed read-only tools for inventory, guarded capacity, pool status, host storage, sharing status, capabilities, console records, observed events, control availability, and capacity history. It cannot execute controls. Control mutation remains GUI-only until command identity, authorization, and audit behavior are complete. MCP may write bounded private local logs/history; it has no device or arbitrary-filesystem write tool. Service readings carry `sourceTrust: "unverified-loopback"`.

## Original command catalogue (recovered)

The original service build identified by SHA-256 `6348d0374c0193c628380426bb7c9a814c41f67bc02c4304c0eaf4116efd5732` exposes command IDs 0..103. This table records names and dispatch-table pointers recovered from ignored analysis artifacts. It is a compatibility inventory, not an implementation promise. OpenDrobo currently routes only six GUI operations: identify, LED presets, brightness read, restart, shutdown, and standby. Those six remain experimental, gated, mock-tested, and hardware-untested. The other 98 entries are not implemented by OpenDrobo; many are internal, model-specific, NAS-specific, destructive, or research-only. Names such as `eCmdGetVersion` do not mean OpenDrobo implements a matching route.

| ID | Original name | Dispatch pointer |
| ---: | --- | --- |
| 0 | `eCmdSetAdmin` | 0x1003f95fc |
| 1 | `eCmdGetAdmin` | 0x1003f9609 |
| 2 | `eCmdCreateLUN` | 0x1003f9616 |
| 3 | `eCmdDeleteLUN` | 0x1003f9624 |
| 4 | `eCmdRenameLUN` | 0x1003f9632 |
| 5 | `eCmdGetNextUniqueLUNID` | 0x1003f9640 |
| 6 | `eCmdSetCHAP` | 0x1003f9657 |
| 7 | `eCmdGetVersion` | 0x1003f9663 |
| 8 | `eCmdSetDimming` | 0x1003f9672 |
| 9 | `eCmdGetDimming` | 0x1003f9681 |
| 10 | `eCmdDeviceLogin` | 0x1003f9690 |
| 11 | `eCmdDeviceLogout` | 0x1003f96a0 |
| 12 | `eCmdMount` | 0x1003f96b1 |
| 13 | `eCmdUnmount` | 0x1003f96bb |
| 14 | `eCmdForceUpdate` | 0x1003f96c7 |
| 15 | `eCmdGetDiskPackId` | 0x1003f96d7 |
| 16 | `eCmdRepair` | 0x1003f96e9 |
| 17 | `eCmdSetFlashConfig` | 0x1003f96f4 |
| 18 | `eCmdSendTestEmail` | 0x1003f9707 |
| 19 | `eCmdSendDashboardTestEmail` | 0x1003f9719 |
| 20 | `eCmdGetKernelInterfaceInfo` | 0x1003f9734 |
| 21 | `eCmdRestart` | 0x1003f974f |
| 22 | `eCmdStandby` | 0x1003f975b |
| 23 | `eCmdShutdown` | 0x1003f9767 |
| 24 | `eCmdReset` | 0x1003f9774 |
| 25 | `eCmdRename` | 0x1003f977e |
| 26 | `eCmdIdentify` | 0x1003f9789 |
| 27 | `eCmdInstallFirmware` | 0x1003f9796 |
| 28 | `eCmdRevertFirmware` | 0x1003f97aa |
| 29 | `eCmdUploadDiags` | 0x1003f97bd |
| 30 | `eCmdGetConfig` | 0x1003f97cd |
| 31 | `eCmdSetConfig` | 0x1003f97db |
| 32 | `eCmdGetFirmwareInfo` | 0x1003f97e9 |
| 33 | `eCmdFormatDevice` | 0x1003f97fd |
| 34 | `eCmdFormatLUN` | 0x1003f980e |
| 35 | `eCmdGetDroboSyncSettings` | 0x1003f981c |
| 36 | `eCmdSetDroboSyncSettings` | 0x1003f9835 |
| 37 | `eCmdGetDroboSyncSummaryLog` | 0x1003f984e |
| 38 | `eCmdGetDroboSyncDetailedLog` | 0x1003f9869 |
| 39 | `eCmdGetDroboSyncLog` | 0x1003f9885 |
| 40 | `eCmdSyncNow` | 0x1003f9899 |
| 41 | `eCmdStopSync` | 0x1003f98a5 |
| 42 | `eCmdGetReservedDriveLetters` | 0x1003f98b2 |
| 43 | `eCmdGetDemoModeInfo` | 0x1003f98ce |
| 44 | `eCmdSetDemoModeInfo` | 0x1003f98e2 |
| 45 | `eCmdUpdateUserEmailSetting` | 0x1003f98f6 |
| 46 | `eCmdGetUserEmailSettings` | 0x1003f9911 |
| 47 | `eCmdGetDevices` | 0x1003f992a |
| 48 | `eCmdGetUpdate` | 0x1003f9939 |
| 49 | `eCmdInsertDrive` | 0x1003f9947 |
| 50 | `eCmdRemoveDrive` | 0x1003f9957 |
| 51 | `eCmdRunESACommand` | 0x1003f9967 |
| 52 | `eCmdSendTMCommand` | 0x1003f9979 |
| 53 | `eCmdEnableAutoDiscovery` | 0x1003f998b |
| 54 | `eCmdIsAutoDiscoveryEnabled` | 0x1003f99a3 |
| 55 | `eCmdAddManualDiscoveryIP` | 0x1003f99be |
| 56 | `eCmdRemoveManualDiscoveryIP` | 0x1003f99d7 |
| 57 | `eCmdGetManualDiscoveryIPList` | 0x1003f99f3 |
| 58 | `eCmdModeSense` | 0x1003f9a10 |
| 59 | `eCmdIsSCSIConnAvailable` | 0x1003f9a1e |
| 60 | `eCmdAddVolume` | 0x1003f9a36 |
| 61 | `eCmdGetSysInfo` | 0x1003f9a44 |
| 62 | `eCmdGetPerformance` | 0x1003f9a53 |
| 63 | `eCmdGetFanInfo` | 0x1003f9a66 |
| 64 | `eCmdGetPowerInfo` | 0x1003f9a75 |
| 65 | `eCmdGetControllerCard` | 0x1003f9a86 |
| 66 | `eCmdGetExpanderCard` | 0x1003f9a9c |
| 67 | `eCmdCacheBattery` | 0x1003f9ab0 |
| 68 | `eCmdForceSingleInitiator` | 0x1003f9ac1 |
| 69 | `eCmdGetOSInfo` | 0x1003f9ada |
| 70 | `eCmdReadHostBuffer` | 0x1003f9ae8 |
| 71 | `eCmdWriteHostBuffer` | 0x1003f9afb |
| 72 | `eCmdGetTMDiagFilePath` | 0x1003f9b0f |
| 73 | `eCmdForceRepair` | 0x1003f9b25 |
| 74 | `eCmdGetLunInitatorExtraInfo` | 0x1003f9b35 |
| 75 | `eCmdDoPoll` | 0x1003f9b51 |
| 76 | `eCmdToggleNightMode` | 0x1003f9b5c |
| 77 | `eCmdGetDroboAppsStatus` | 0x1003f9b70 |
| 78 | `eCmdDroboAppsAction` | 0x1003f9b87 |
| 79 | `eCmdAuthenticateUser` | 0x1003f9b9b |
| 80 | `eCmdGetAdminConfig` | 0x1003f9bb0 |
| 81 | `eCmdGetSharesConfig` | 0x1003f9bc3 |
| 82 | `eCmdSetTime` | 0x1003f9bd7 |
| 83 | `eCmdGetEventLogs` | 0x1003f9be3 |
| 84 | `eCmdGetSessionIPAddress` | 0x1003f9bf4 |
| 85 | `eCmdGetAllDevicesStatusXML` | 0x1003f9c0c |
| 86 | `eCmdUploadFileToSupport` | 0x1003f9c27 |
| 87 | `eCmdCancelUploadToSupport` | 0x1003f9c3f |
| 88 | `eCmdGetRegisteredEmail` | 0x1003f9c59 |
| 89 | `eCmdRunCommand` | 0x1003f9c70 |
| 90 | `eCmdGetUserGroupList` | 0x1003f9c7f |
| 91 | `eCmdGetUserEventLog` | 0x1003f9c94 |
| 92 | `eCmdUpdateUserGroupList` | 0x1003f9ca8 |
| 93 | `eCmdGetDeviceNamesToShutdown` | 0x1003f9cc0 |
| 94 | `eCmdGetSavedFileContent` | 0x1003f9cdd |
| 95 | `eCmdDownloadConfigFile` | 0x1003f9cf5 |
| 96 | `eCmdDownloadAllConfigFiles` | 0x1003f9d0c |
| 97 | `eCmdGetDASDeviceNamesWithMountedVolumes` | 0x1003f9d27 |
| 98 | `eCmdGetDeviceMountedVolumesCount` | 0x1003f9d4f |
| 99 | `eCmdPostUserEvent` | 0x1003f9d70 |
| 100 | `eCmdDownloadFTPFileToSharedPath` | 0x1003f9d82 |
| 101 | `eCmdGetAppDataFile` | 0x1003f9da2 |
| 102 | `eCmdGetNASUsernamePassword` | 0x1003f9db5 |
| 103 | `eCmdChkFullDiskAccessOnMac` | 0x1003f9dd0 |
