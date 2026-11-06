#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capacity_history;
mod controls;
mod device_events;
mod diagnostics;
mod mcp;
mod native;
mod service;
mod system;
mod updates;

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: &'static str,
    platform: &'static str,
    filesystem_module_built: bool,
    mcp_command_path: Option<String>,
    message: &'static str,
}

#[tauri::command]
async fn get_inventory() -> Result<native::Inventory, String> {
    diagnostics::record(
        diagnostics::Level::Info,
        "app",
        "inventory",
        "Registry refresh requested.",
    );
    let result = tauri::async_runtime::spawn_blocking(native::inventory)
        .await
        .map_err(|_| "Inventory worker stopped".to_string())?;
    diagnostics::record(
        if result.is_ok() {
            diagnostics::Level::Info
        } else {
            diagnostics::Level::Warning
        },
        "app",
        "inventory",
        "Registry refresh finished.",
    );
    result
}

#[tauri::command]
async fn read_capacity(device_id: String) -> Result<native::CapacityResponse, String> {
    diagnostics::record(
        diagnostics::Level::Info,
        "app",
        "capacity",
        "Guarded protected-capacity read requested.",
    );
    let result = tauri::async_runtime::spawn_blocking(move || native::capacity(&device_id))
        .await
        .map_err(|_| "Capacity worker stopped".to_string());
    diagnostics::record(
        diagnostics::Level::Info,
        "app",
        "capacity",
        "Capacity read finished; unit and volume-scope guards remain enforced.",
    );
    result
}

#[tauri::command]
async fn check_updates() -> updates::UpdateResponse {
    diagnostics::record(
        diagnostics::Level::Info,
        "app",
        "releases",
        "Project release check requested.",
    );
    let result = updates::check().await;
    diagnostics::record(
        diagnostics::Level::Info,
        "app",
        "releases",
        "Project release check finished.",
    );
    result
}

#[tauri::command]
async fn get_storage_status() -> Result<serde_json::Value, String> {
    let result = tauri::async_runtime::spawn_blocking(system::storage_status)
        .await
        .map_err(|_| "Host storage inspection stopped.".to_string())?;
    diagnostics::record(
        if result.is_ok() {
            diagnostics::Level::Info
        } else {
            diagnostics::Level::Warning
        },
        "app",
        "volumes",
        "Host logical-volume inspection finished.",
    );
    result
}

#[tauri::command]
async fn get_sharing_status() -> Result<serde_json::Value, String> {
    let result = tauri::async_runtime::spawn_blocking(system::sharing_status)
        .await
        .map_err(|_| "Host sharing inspection stopped.".to_string())?;
    diagnostics::record(
        if result.is_ok() {
            diagnostics::Level::Info
        } else {
            diagnostics::Level::Warning
        },
        "app",
        "sharing",
        "Host sharing inspection finished; no service or permission changes requested.",
    );
    result
}

#[tauri::command]
async fn get_pool_status() -> Result<serde_json::Value, String> {
    diagnostics::record(
        diagnostics::Level::Info,
        "app",
        "pool",
        "Original service pool snapshot requested.",
    );
    let result = tauri::async_runtime::spawn_blocking(service::status)
        .await
        .map_err(|_| "Pool status worker stopped.".to_string())?;
    diagnostics::record(
        if result.is_ok() {
            diagnostics::Level::Info
        } else {
            diagnostics::Level::Warning
        },
        "app",
        "pool",
        "Pool snapshot read finished; no filesystem reporting was enabled.",
    );
    result
}

#[tauri::command]
fn get_device_events(limit: Option<usize>) -> serde_json::Value {
    device_events::snapshot(limit.unwrap_or(200))
}

#[tauri::command]
async fn get_capacity_history(
    unit_id: Option<String>,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        capacity_history::snapshot(unit_id.as_deref(), limit.unwrap_or(2048))
    })
    .await
    .map_err(|_| "Capacity history inspection stopped.".to_string())?
}

#[tauri::command]
async fn get_control_status() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(controls::status)
        .await
        .map_err(|_| "Device control inspection stopped.".to_string())?
}

#[tauri::command]
async fn execute_device_control(request: controls::Request) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || controls::execute(request))
        .await
        .map_err(|_| {
            "Device control worker stopped; inspect the unit before retrying.".to_string()
        })?
}

#[tauri::command]
fn get_console_records(limit: Option<usize>) -> Result<diagnostics::Snapshot, String> {
    diagnostics::snapshot(limit.unwrap_or(200))
}

#[tauri::command]
async fn open_sharing_settings() -> Result<String, String> {
    let result = tauri::async_runtime::spawn_blocking(system::open_sharing_settings)
        .await
        .map_err(|_| "Sharing settings worker stopped.".to_string())?;
    diagnostics::record(
        if result.is_ok() {
            diagnostics::Level::Info
        } else {
            diagnostics::Level::Warning
        },
        "app",
        "sharing-settings",
        "Native sharing-settings handoff requested; OS accounts and permissions remain authoritative.",
    );
    result
}

struct AppMcpBackend;
impl mcp::Backend for AppMcpBackend {
    fn call(
        &mut self,
        name: &str,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let result = match name {
            "opendrobo_inventory" => native::inventory().and_then(|r| {
                serde_json::to_value(r).map_err(|_| "Inventory serialization failed.".into())
            }),
            "opendrobo_read_capacity" => {
                let id = args
                    .get("deviceId")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Device identifier required.".to_string())?;
                serde_json::to_value(native::capacity(id))
                    .map_err(|_| "Capacity serialization failed.".into())
            }
            "opendrobo_storage_status" => system::storage_status(),
            "opendrobo_sharing_status" => system::sharing_status(),
            "opendrobo_pool_status" => service::status(),
            "opendrobo_events" => Ok(device_events::snapshot(
                args.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize,
            )),
            "opendrobo_controls" => controls::status(),
            "opendrobo_capacity_history" => capacity_history::snapshot(
                args.get("unitId").and_then(|value| value.as_str()),
                args.get("limit")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(2048) as usize,
            ),
            "opendrobo_capabilities" => Ok(system::capabilities()),
            "opendrobo_console" => {
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
                diagnostics::snapshot(limit).and_then(|r| {
                    serde_json::to_value(r).map_err(|_| "Console serialization failed.".into())
                })
            }
            _ => Err("Unknown read-only tool.".into()),
        };
        diagnostics::record(
            if result.is_ok() {
                diagnostics::Level::Info
            } else {
                diagnostics::Level::Warning
            },
            "mcp",
            name,
            "Read-only MCP tool finished.",
        );
        result
    }
}

#[tauri::command]
fn open_release(url: String) -> Result<(), String> {
    updates::validate_release_url(&url)?;
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("/usr/bin/open")
        .arg(&url)
        .status();
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(&url).status();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &url])
        .status();
    result
        .map_err(|_| "Could not open the release page".to_string())?
        .success()
        .then_some(())
        .ok_or_else(|| "Could not open the release page".to_string())
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    let bundled = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent()?.parent().map(|p| p.to_path_buf()))
        .is_some_and(|contents| contents.join("Extensions/OpenDroboFS.appex").is_dir());
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        filesystem_module_built: bundled,
        mcp_command_path: std::env::current_exe()
            .ok()
            .and_then(|path| path.to_str().map(str::to_owned)),
        message: "Development preview with read-only monitoring and explicit GUI device controls. Driver replacement and Finder validation remain pending.",
    }
}

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.first().is_some_and(|arg| arg == "--mcp-stdio") {
        if arguments.len() != 1 {
            eprintln!("Usage: opendrobo-desktop --mcp-stdio");
            std::process::exit(2);
        }
        diagnostics::record(
            diagnostics::Level::Info,
            "mcp",
            "startup",
            "Read-only MCP stdio server started; no network listener.",
        );
        if mcp::serve_stdio(AppMcpBackend).is_err() {
            diagnostics::record(
                diagnostics::Level::Error,
                "mcp",
                "transport",
                "MCP stdio transport stopped with an error.",
            );
            eprintln!("MCP stdio transport failed.");
            std::process::exit(1);
        }
        return;
    }
    if arguments == ["--storage-json"]
        || arguments == ["--sharing-json"]
        || arguments == ["--console-json"]
        || arguments == ["--pool-json"]
        || arguments == ["--events-json"]
        || arguments == ["--controls-json"]
        || arguments == ["--history-json"]
    {
        let result = match arguments[0].as_str() {
            "--storage-json" => system::storage_status(),
            "--sharing-json" => system::sharing_status(),
            "--pool-json" => service::status(),
            "--controls-json" => controls::status(),
            "--history-json" => capacity_history::snapshot(None, 2048),
            "--events-json" => {
                let _ = service::status();
                Ok(device_events::snapshot(200))
            }
            _ => diagnostics::snapshot(200).and_then(|r| {
                serde_json::to_value(r).map_err(|_| "Console serialization failed.".into())
            }),
        };
        match result {
            Ok(value) => println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("metadata serializes")
            ),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if arguments
        .first()
        .is_some_and(|arg| arg == "--capture-capacity-json")
    {
        if arguments.len() != 2 {
            eprintln!("Usage: opendrobo-desktop --capture-capacity-json <registry-id>");
            std::process::exit(2);
        }
        match native::raw_capture(&arguments[1]) {
            Ok(value) => println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("capture serializes")
            ),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if arguments == ["--scan-json"] {
        match native::inventory() {
            Ok(value) => println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("inventory serializes")
            ),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if !arguments.is_empty() {
        eprintln!(
            "Supported modes: --scan-json, --capture-capacity-json <id>, --storage-json, --sharing-json, --pool-json, --events-json, --controls-json, --history-json, --console-json, --mcp-stdio"
        );
        std::process::exit(2);
    }
    diagnostics::record(
        diagnostics::Level::Info,
        "app",
        "startup",
        "OpenDrobo started with read-only monitoring and explicit GUI-only device controls.",
    );
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Emitter;
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    match service::status() {
                        Ok(snapshot) => {
                            let _ = handle.emit("opendrobo-pool-status", snapshot);
                        }
                        Err(_) => {
                            let _ = handle.emit(
                                "opendrobo-pool-error",
                                "Original service status is unavailable.",
                            );
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_inventory,
            read_capacity,
            check_updates,
            open_release,
            get_app_info,
            get_storage_status,
            get_sharing_status,
            get_pool_status,
            get_device_events,
            get_capacity_history,
            get_control_status,
            execute_device_control,
            get_console_records,
            open_sharing_settings
        ])
        .run(tauri::generate_context!())
        .expect("OpenDrobo desktop runtime failed");
}
