#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod native;
mod updates;

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: &'static str,
    platform: &'static str,
    filesystem_module_built: bool,
    message: &'static str,
}

#[tauri::command]
async fn get_inventory() -> Result<native::Inventory, String> {
    tauri::async_runtime::spawn_blocking(native::inventory)
        .await
        .map_err(|_| "Inventory worker stopped".to_string())?
}

#[tauri::command]
async fn read_capacity(device_id: String) -> Result<native::CapacityResponse, String> {
    tauri::async_runtime::spawn_blocking(move || native::capacity(&device_id))
        .await
        .map_err(|_| "Capacity worker stopped".to_string())
}

#[tauri::command]
async fn check_updates() -> updates::UpdateResponse {
    updates::check().await
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
        message: "Read-only development build. Driver replacement and Finder mount validation remain pending.",
    }
}

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
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
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_inventory,
            read_capacity,
            check_updates,
            open_release,
            get_app_info
        ])
        .run(tauri::generate_context!())
        .expect("OpenDrobo desktop runtime failed");
}
