# OpenDrobo MCP integration

OpenDrobo exposes an optional local Model Context Protocol (MCP) server for trusted, local diagnostics. It lets an MCP client ask for current Drobo inventory, protected-pool capacity, host volume metadata, sharing status, bounded events and console records, capability information, control availability, and local capacity history. This supports an AI assistant or script that helps explain capacity trends and support observations; it does not configure MCP for the app and does not make the app a network service.

## Start and transport

Start the desktop binary with the explicit `--mcp-stdio` argument. The process inherits the caller's standard input and output and speaks newline-delimited JSON-RPC. `get_app_info` returns the absolute executable path as `mcpCommandPath`; clients can use that path in their own configuration:

```json
{"mcpServers":{"opendrobo":{"command":"/absolute/path/to/OpenDrobo.app/Contents/MacOS/opendrobo-desktop","args":["--mcp-stdio"]}}}
```

MCP is opt-in per client. OpenDrobo does not install a client, listen on a network port, relay requests over the network, or expose a shell/CDB/arbitrary-file endpoint. It works without MCP configuration through the normal desktop UI.

## Fixed tool boundary

The server publishes exactly ten fixed tools:

1. `opendrobo_inventory`
2. `opendrobo_read_capacity`
3. `opendrobo_pool_status`
4. `opendrobo_storage_status`
5. `opendrobo_sharing_status`
6. `opendrobo_capabilities`
7. `opendrobo_console`
8. `opendrobo_events`
9. `opendrobo_controls`
10. `opendrobo_capacity_history`

All are read-only from the client's perspective. No tool accepts an execute request or dynamically selected command. No tool changes device state, mounts a volume, changes native ACLs, writes a share, or reads a caller-selected path.

Responses may include mount paths, share names, device/model labels, firmware display fields, and aggregate byte counts. They do not expose serial numbers, credentials, raw service XML, filesystem contents, or unrestricted command arguments. Byte-sized `u64` values use decimal strings. Pool responses separate protected total/used/free from OS logical usage and identify service-derived readings with `sourceTrust: "unverified-loopback"`.

Storage metadata includes per-volume driver and physical-enclosure identifiers, the driver product name, and optional bounded native macOS volume icons generated at runtime. Icons are read only for verified mounted Drobo roots; callers cannot choose paths.

Sharing reports native SMB/NFS service status. SMB exports are included only when their configured path falls beneath a verified Drobo mount root using component boundaries; symlink targets are not resolved, so this is metadata classification rather than a file-access boundary; OpenDrobo does not modify native accounts, permissions, or ACLs. Linux and Windows Drobo adapters are not implemented. Hardware-specific telemetry is bounded: firmware version/release date and drive manufacturer/model/firmware revision are shown only when supplied, as bounded ASCII fields; temperature, RPM, and serial identity are not inferred.

## Security and trust assessment

MCP has no per-tool authorization layer. Security boundary is local process ownership plus the caller's ability to start the binary and read its output. Client software can forward returned data elsewhere; OpenDrobo does not control that downstream handling. Keep MCP client configuration and output within a trusted local account.

The original macOS observer connects to plain `127.0.0.1:4502` and validates bounded XML, but does not authenticate the peer. If the original DDService is absent, a lower-privilege local process could bind that port and provide false pool/readings, which could poison bounded local history. The original running listener cannot be displaced. This is one supported low-severity, high-confidence boundary gap in static review. It does not demonstrate MCP control execution, privilege escalation, arbitrary file/disk writes, or remote attack.

`sourceTrust` makes this limitation visible; it is not an authentication fix. Before any autonomous action or broader trust claim, add authenticated local IPC with peer identity and an attestation/authorization decision for the service. Do not treat a binary hash, response bounds, or a client-side label as peer authentication. No dependency-CVE or runtime-exploit claim follows from this assessment.

## Operational limits

- One active backend job and at most 16 queued jobs. Each input frame is limited to 64 KiB and each serialized response to 1 MiB; compatibility text is capped at 128 KiB. Oversized input and output are rejected.
- Cancellation suppresses queued results but cannot interrupt native I/O already in progress.
- Original-service status subscription has a fixed deadline of about 35 seconds. Guarded native capacity I/O cannot be interrupted; an already-running kernel call may outlive that deadline. Clients should poll conservatively and surface stale/read-error state.
- Console and capacity history may write bounded private local app files. “Read-only MCP” means no device/control/filesystem mutation tool; it does not mean zero local writes.
- History is a local observation series, not recovered onboard history or a backfill source.

## Client guidance

Use MCP for inspection, explanations, capacity trend collection, and support evidence. Treat missing, stale, or `unverified-loopback` values as unknown rather than healthy. Do not automate restart, shutdown, or standby through MCP; those remain experimental GUI-only light/power controls, with hardware execution untested. Firmware, reset, format, rebuild, and disk-pack operations are unavailable. Review and redact client-side forwarding before sending results outside the local host.
