//! Host metadata and native sharing controls. No filesystem or RAID mutation.
use serde_json::{Value, json};

pub fn capabilities() -> Value {
    json!({
        "mcp": {"transport":"stdio","readOnly":true,"networkListener":false,
                "protocolVersions":["2026-07-28","2025-11-25"]},
        "permissions": {"model":"os-native","message":"Host accounts and folder ACLs remain authoritative. Export restrictions are separate."},
        "storageProviders":[
            {"id":"drobo","management":"Registry discovery and guarded capacity reads through the original macOS driver."},
            {"id":"host","management":"Mounted-volume metadata only; no vendor RAID management."}
        ],
        "filesystems":[
            {"id":"hfs-apfs","driverProvided":false,"message":"Visible when mounted by the host operating system."},
            {"id":"ntfs-exfat","driverProvided":false,"message":"Host filesystem driver determines access and write support."},
            {"id":"ext-xfs-btrfs-zfs","driverProvided":false,"message":"Requires a compatible host filesystem driver. OpenDrobo does not install one."}
        ],
        "droboThunderbolt":{"macOS":"Original driver required; replacement remains disabled.",
                            "linux":"Unverified; no implemented transport adapter.",
                            "windows":"No implemented transport adapter."},
        "filesystemView":{"minimumMacOS":"26.0","activated":false},
        "otherRaidManagement":"Provider adapters are not implemented. Mounted arrays may be visible as ordinary host volumes."
    })
}

#[cfg(target_os = "macos")]
pub fn storage_status() -> Result<Value, String> {
    use std::ffi::{CStr, c_char};
    #[repr(C)]
    struct NativeVolume {
        registry_id: u64,
        total_bytes: u64,
        used_bytes: u64,
        available_bytes: u64,
        read_only: u32,
        statistics_available: u32,
        id: [c_char; 64],
        name: [c_char; 256],
        filesystem: [c_char; 32],
        mount_path: [c_char; 4096],
    }
    const _: () = assert!(std::mem::size_of::<NativeVolume>() == 4488);
    unsafe extern "C" {
        fn od_list_volumes(volumes: *mut NativeVolume, limit: usize, count: *mut usize) -> i32;
    }
    fn text(value: &[c_char]) -> String {
        // The C adapter clears each bounded record and terminates its strings.
        unsafe { CStr::from_ptr(value.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }
    let mut records: Vec<NativeVolume> = (0..128)
        .map(|_| NativeVolume {
            registry_id: 0,
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
            read_only: 0,
            statistics_available: 0,
            id: [0; 64],
            name: [0; 256],
            filesystem: [0; 32],
            mount_path: [0; 4096],
        })
        .collect();
    let mut count = 0;
    let status = unsafe { od_list_volumes(records.as_mut_ptr(), records.len(), &mut count) };
    if status != 0 || count > records.len() {
        return Err("Host volume metadata is unavailable or exceeds the bounded inventory.".into());
    }
    let volumes: Vec<Value> = records[..count]
        .iter()
        .filter(|v| v.registry_id != 0)
        .map(|v| {
            let measured = v.statistics_available != 0;
            json!({"id":text(&v.id),"name":text(&v.name),"filesystem":text(&v.filesystem),
            "mountPath":text(&v.mount_path),
            "totalBytes":measured.then(||v.total_bytes.to_string()),
            "usedBytes":measured.then(||v.used_bytes.to_string()),
            "availableBytes":measured.then(||v.available_bytes.to_string()),
            "deviceId":(v.registry_id!=0).then(||v.registry_id.to_string()),
            "provider":if v.registry_id!=0 {"drobo"} else {"host"},
            "accounting":"os-logical","readOnly":v.read_only!=0})
        })
        .collect();
    Ok(json!({"platform":"macos","volumes":volumes,
        "message":"Drobo-mounted volumes only. OS filesystem usage and protected-pool allocation are separate accounting measures."}))
}

#[cfg(not(target_os = "macos"))]
pub fn storage_status() -> Result<Value, String> {
    Ok(json!({"platform":std::env::consts::OS,"volumes":[],
        "message":"The shared UI/MCP layer is available. Host-volume discovery for this platform is not implemented."}))
}

#[cfg(target_os = "macos")]
fn read_command(program: &'static str, arguments: &[&str]) -> Result<(bool, String), String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let mut child = Command::new(program)
        .args(arguments)
        .env("LC_ALL", "C")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Host sharing inspection is unavailable.".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Sharing inspection output unavailable.".to_string())?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(131_073).read_to_end(&mut bytes).map(|_| bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| "Sharing inspection reader stopped.".to_string())?
        .map_err(|_| "Sharing inspection output unavailable.".to_string())?;
    if bytes.len() > 131_072 {
        return Err("Sharing inspection exceeded its output limit.".into());
    }
    Ok((success, String::from_utf8_lossy(&bytes).into_owned()))
}

fn parse_shares(text: &str) -> Option<Vec<Value>> {
    if text
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(str::trim)
        != Some("List of Share Points")
    {
        return None;
    }
    let mut shares = Vec::new();
    let mut name = String::new();
    let mut path = String::new();
    let mut smb = false;
    let mut shared = None;
    let mut saw_smb = false;
    let mut records = 0usize;
    let mut readonly = None;
    let mut guest = None;
    fn push(
        shares: &mut Vec<Value>,
        name: &str,
        path: &str,
        shared: Option<bool>,
        saw_smb: bool,
        readonly: Option<bool>,
        guest: Option<bool>,
    ) -> bool {
        if name.is_empty()
            || name.len() > 256
            || !path.starts_with('/')
            || path.len() > 4096
            || !saw_smb
            || shared.is_none()
        {
            return false;
        }
        if shared == Some(true) {
            shares.push(
                json!({"id":format!("smb-{}",shares.len()),"name":name,"path":path,
                "protocol":"SMB","readOnly":readonly,"guestAllowed":guest}),
            );
        }
        true
    }
    for line in text
        .lines()
        .skip_while(|line| line.trim().is_empty())
        .skip(1)
    {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if line.starts_with("name:") {
            if smb {
                return None;
            }
            if records > 0 && !push(&mut shares, &name, &path, shared, saw_smb, readonly, guest) {
                return None;
            }
            records += 1;
            if records > 128 {
                return None;
            }
            name = trimmed
                .strip_prefix("name:")
                .unwrap_or("")
                .trim()
                .chars()
                .collect();
            path.clear();
            smb = false;
            shared = None;
            saw_smb = false;
            readonly = None;
            guest = None;
        } else if line.starts_with("path:") && !smb && records > 0 {
            if !path.is_empty() {
                return None;
            }
            let value = trimmed.strip_prefix("path:")?;
            path = value.trim().to_string();
        } else if trimmed
            .strip_prefix("smb:")
            .is_some_and(|suffix| suffix.trim() == "{")
        {
            if smb || saw_smb || records == 0 {
                return None;
            }
            smb = true;
            saw_smb = true;
        } else if trimmed == "}" {
            if !smb {
                return None;
            }
            smb = false;
        } else if smb {
            if let Some(value) = trimmed.strip_prefix("shared:") {
                if shared.is_some() {
                    return None;
                }
                shared = match value.trim() {
                    "1" => Some(true),
                    "0" => Some(false),
                    _ => return None,
                };
            } else if let Some(value) = trimmed
                .strip_prefix("read-only:")
                .or_else(|| trimmed.strip_prefix("readonly:"))
            {
                readonly = match value.trim() {
                    "1" => Some(true),
                    "0" => Some(false),
                    _ => None,
                };
            } else if let Some(value) = trimmed.strip_prefix("guest access:") {
                guest = match value.trim() {
                    "1" => Some(true),
                    "0" => Some(false),
                    _ => None,
                };
            } else if let Some(value) = trimmed.strip_prefix("name:") {
                if value.trim().is_empty() || value.len() > 256 {
                    return None;
                }
            } else if let Some(value) = trimmed.strip_prefix("sealed:") {
                if !matches!(value.trim(), "0" | "1") {
                    return None;
                }
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    if smb || records > 0 && !push(&mut shares, &name, &path, shared, saw_smb, readonly, guest) {
        return None;
    }
    Some(shares)
}

fn drobo_exports(exports: Vec<Value>, storage: &Value) -> Option<Vec<Value>> {
    fn scoped_path(path: &str) -> bool {
        path.starts_with('/')
            && path != "/"
            && !path.chars().any(char::is_control)
            && !path.split('/').any(|part| matches!(part, "." | ".."))
    }
    let roots: Vec<&str> = storage
        .get("volumes")?
        .as_array()?
        .iter()
        .filter(|volume| volume.get("provider").and_then(Value::as_str) == Some("drobo"))
        .filter_map(|volume| volume.get("mountPath").and_then(Value::as_str))
        .filter(|path| scoped_path(path))
        .collect();
    Some(
        exports
            .into_iter()
            .filter(|export| {
                export
                    .get("path")
                    .and_then(Value::as_str)
                    .is_some_and(|path| {
                        scoped_path(path) && {
                            roots.iter().any(|root| {
                                path == *root
                                    || path
                                        .strip_prefix(root)
                                        .is_some_and(|suffix| suffix.starts_with('/'))
                            })
                        }
                    })
            })
            .collect(),
    )
}

pub fn sharing_status() -> Result<Value, String> {
    #[cfg(target_os = "macos")]
    {
        let mut services = Vec::new();
        for (id, name, label) in [
            ("smb", "SMB", "system/com.apple.smbd"),
            ("nfs", "NFS", "system/com.apple.nfsd"),
        ] {
            let result = read_command("/bin/launchctl", &["print", label]);
            let state = match &result {
                Ok((true, text)) if text.lines().any(|l| l.trim() == "state = running") => {
                    "running"
                }
                Ok((true, _)) => "stopped",
                _ => "unknown",
            };
            services.push(json!({"id":id,"name":name,"state":state,"supported":true,
                "message":"Host-managed service. OpenDrobo does not start, reconfigure or expose it automatically."}));
        }
        let inspection = read_command("/usr/sbin/sharing", &["-l"]);
        let storage = storage_status();
        let parsed = match inspection {
            Ok((true, text)) => parse_shares(&text).and_then(|exports| {
                storage
                    .as_ref()
                    .ok()
                    .and_then(|volumes| drobo_exports(exports, volumes))
            }),
            _ => None,
        };
        let available = parsed.is_some();
        let exports = parsed.unwrap_or_default();
        let exports_state = if !available {
            "unknown"
        } else if exports.is_empty() {
            "none"
        } else {
            "present"
        };
        return Ok(
            json!({"platform":"macos","services":services,"exports":exports,"exportsAvailable":available,"exportsState":exports_state,
            "configuration":{"permissions":"os-native","settingsActionAvailable":true,
                "message":"Configure shares with macOS File Sharing. Native accounts and folder ACLs govern access; an export can additionally restrict writes."},
            "message":if available {"Only exports inside detected Drobo-mounted volumes are listed. macOS manages other local shares separately. No folders are scanned for files."} else {"Drobo share records or mounted-volume scope could not be verified. Use macOS File Sharing to inspect native permissions."}}),
        );
    }
    #[cfg(not(target_os = "macos"))]
    Ok(
        json!({"platform":std::env::consts::OS,"services":[],"exports":[],"exportsAvailable":false,"exportsState":"unknown",
        "configuration":{"permissions":"os-native","settingsActionAvailable":false,
            "message":"Use host-native account, ACL and share controls. This platform's sharing adapter is not implemented."},
        "message":"SMB/NFS adapters for this platform are planned; no network service is enabled by OpenDrobo."}),
    )
}

pub fn open_sharing_settings() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let (success, _) = read_command(
            "/usr/bin/open",
            &["x-apple.systempreferences:com.apple.Sharing-Settings.extension?FileSharing"],
        )?;
        return success.then(||"Opened macOS File Sharing. Accounts and folder permissions remain managed by macOS.".into())
            .ok_or_else(||"Could not open macOS File Sharing settings.".into());
    }
    #[cfg(not(target_os = "macos"))]
    Err("Native sharing-settings handoff is not implemented on this platform.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sharing_parser_keeps_export_and_folder_permissions_separate() {
        let text = "List of Share Points\nname:\tMedia\npath:\t/Volumes/Test/Media\nsmb: {\n\tname: Media\n\tshared: 1\n\tguest access: 0\n\tread-only: 1\n}\nname: Disabled\npath: /Volumes/Test/Private\nsmb: {\nshared: 0\n}\n";
        let shares = parse_shares(text).expect("recognized SMB share records");
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0]["path"], "/Volumes/Test/Media");
        assert_eq!(shares[0]["readOnly"], true);
        assert_eq!(shares[0]["guestAllowed"], false);
        assert!(parse_shares("Unable to query share point records.").is_none());
        assert!(parse_shares("").is_none());
        assert!(
            parse_shares(
                "List of Share Points\nname: Unknown\npath: /Volumes/Test\nSMB records changed\n"
            )
            .is_none()
        );
        assert!(parse_shares("List of Share Points\n").unwrap().is_empty());
        assert!(parse_shares("List of Share Points\ngarbage\n").is_none());
        assert!(parse_shares("List of Share Points\nname: Broken\npath: /Volumes/Test\nsmb: {\nshared: 0\nname: Next\npath: /Volumes/Next\nsmb: {\nshared: 0\n}\n").is_none());
        assert!(
            parse_shares(
                "List of Share Points\nname: Disabled\npath: /Volumes/Test\nsmb: {\nshared: 0\n}\n"
            )
            .unwrap()
            .is_empty()
        );
    }
    #[test]
    fn capabilities_never_claim_new_drivers_or_management_backends() {
        let result = capabilities();
        assert_eq!(result["mcp"]["networkListener"], false);
        for fs in result["filesystems"].as_array().unwrap() {
            assert_eq!(fs["driverProvided"], false);
        }
        assert_eq!(result["permissions"]["model"], "os-native");
    }
    #[test]
    fn share_scope_excludes_local_volumes_and_path_prefix_lookalikes() {
        let storage = json!({"volumes":[{"provider":"drobo","mountPath":"/Volumes/Drobo"},
            {"provider":"host","mountPath":"/Volumes/Local"}]});
        let input = vec![
            json!({"path":"/Volumes/Drobo"}),
            json!({"path":"/Volumes/Drobo/Media"}),
            json!({"path":"/Volumes/DroboOther/Private"}),
            json!({"path":"/Volumes/Local/Photos"}),
            json!({"path":"/Users/Test/Public"}),
            json!({"path":"/Volumes/Drobo/../Local/Share"}),
            json!({"path":"/Volumes/Drobo/./Media"}),
        ];
        let filtered = drobo_exports(input, &storage).unwrap();
        assert_eq!(filtered.len(), 2);
        assert!(drobo_exports(vec![], &json!({})).is_none());
    }
}
