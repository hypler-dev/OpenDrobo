use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    id: String,
    name: String,
    connection: String,
    driver_ready: bool,
}

#[derive(Serialize)]
pub struct Inventory {
    platform: &'static str,
    devices: Vec<Device>,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapacityResponse {
    device_id: String,
    state: &'static str,
    protected_total_bytes: Option<String>,
    protected_used_bytes: Option<String>,
    protected_free_bytes: Option<String>,
    observed_at_ms: Option<i64>,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawCapacityCapture {
    device_id: String,
    captured_at_ms: i64,
    page_hex: String,
    unit_validation: &'static str,
    message: &'static str,
}

impl CapacityResponse {
    fn unavailable(id: &str, message: impl Into<String>) -> Self {
        Self {
            device_id: id.into(),
            state: "unavailable",
            protected_total_bytes: None,
            protected_used_bytes: None,
            protected_free_bytes: None,
            observed_at_ms: None,
            message: message.into(),
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use opendrobo_core::{CapacityConfirmation, decode_capacity};
    use std::ffi::{CStr, c_char};
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static READ_LOCK: Mutex<()> = Mutex::new(());
    #[repr(C)]
    struct NativeDevice {
        registry_id: u64,
        driver_ready: u32,
        thunderbolt: u32,
        name: [c_char; 256],
    }
    unsafe extern "C" {
        fn od_list_devices(devices: *mut NativeDevice, limit: usize, count: *mut usize) -> i32;
        fn od_capture_capacity(id: u64, page: *mut u8, limit: usize, count: *mut usize) -> i32;
        fn od_device_scope(id: u64, mount_path: *mut c_char, limit: usize, count: *mut u32) -> i32;
        fn od_bind_volume(mount_path: *const c_char, id: *mut u64, count: *mut u32) -> i32;
    }

    pub fn inventory() -> Result<Inventory, String> {
        let mut native: [NativeDevice; 128] = std::array::from_fn(|_| NativeDevice {
            registry_id: 0,
            driver_ready: 0,
            thunderbolt: 0,
            name: [0; 256],
        });
        let mut count = 0;
        let status = unsafe { od_list_devices(native.as_mut_ptr(), native.len(), &mut count) };
        if status != 0 {
            return Err(explain(status));
        }
        if count > native.len() {
            return Err("Registry returned too many endpoints".into());
        }
        let devices = native[..count]
            .iter()
            .map(|item| Device {
                id: item.registry_id.to_string(),
                name: unsafe { CStr::from_ptr(item.name.as_ptr()) }
                    .to_string_lossy()
                    .into_owned(),
                connection: if item.thunderbolt != 0 {
                    "Thunderbolt"
                } else {
                    "SCSI"
                }
                .into(),
                driver_ready: item.driver_ready != 0,
            })
            .collect::<Vec<_>>();
        let message = if devices.is_empty() {
            "No supported Drobo endpoint is reported by this Mac."
        } else {
            "Registry identity only. Read protected capacity to query an available management driver."
        };
        Ok(Inventory {
            platform: "macos",
            devices,
            message: message.into(),
        })
    }

    fn capture_bound_page_with(
        id: &str,
        device_scope: impl FnOnce(u64, &mut [c_char], &mut u32) -> Result<(), String>,
        capture_capacity: impl FnOnce(u64, &mut [u8], &mut usize) -> Result<(), String>,
        bind_volume: impl FnOnce(&[c_char], &mut u64, &mut u32) -> Result<(), String>,
        captured_at: impl FnOnce() -> Result<i64, String>,
    ) -> Result<(Vec<u8>, i64), String> {
        let registry_id = id
            .parse::<u64>()
            .map_err(|_| "Invalid device identifier".to_string())?;
        if registry_id == 0 {
            return Err("Invalid device identifier".into());
        }
        let mut path = [0 as c_char; 4096];
        let mut count = 0;
        device_scope(registry_id, &mut path, &mut count)?;
        if count != 1 {
            return Err(
                "A unique mounted volume is required for this experimental capacity path.".into(),
            );
        }
        let captured_at = captured_at()?;
        let mut page = [0_u8; 1308];
        let mut length = 0;
        capture_capacity(registry_id, &mut page, &mut length)?;
        if length > page.len() {
            return Err("Capacity response exceeded its buffer".into());
        }
        let mut bound_id = 0;
        bind_volume(&path, &mut bound_id, &mut count)?;
        if bound_id != registry_id || count != 1 {
            return Err("Device or volume binding changed during the capacity read.".into());
        }
        Ok((page[..length].to_vec(), captured_at))
    }

    fn capture_bound_page(id: &str) -> Result<(Vec<u8>, i64), String> {
        let registry_id = id
            .parse::<u64>()
            .map_err(|_| "Invalid device identifier".to_string())?;
        if registry_id == 0 {
            return Err("Invalid device identifier".into());
        }
        let _guard = READ_LOCK
            .lock()
            .map_err(|_| "Capacity worker is unavailable".to_string())?;
        capture_bound_page_with(
            id,
            |registry_id, path, count| {
                let status =
                    unsafe { od_device_scope(registry_id, path.as_mut_ptr(), path.len(), count) };
                if status == 0 {
                    Ok(())
                } else {
                    Err(explain(status))
                }
            },
            |registry_id, page, length| {
                let status = unsafe {
                    od_capture_capacity(registry_id, page.as_mut_ptr(), page.len(), length)
                };
                if status == 0 {
                    Ok(())
                } else {
                    Err(explain(status))
                }
            },
            |path, bound_id, count| {
                let status = unsafe { od_bind_volume(path.as_ptr(), bound_id, count) };
                if status == 0 {
                    Ok(())
                } else {
                    Err(explain(status))
                }
            },
            now_ms,
        )
    }

    pub fn raw_capture(id: &str) -> Result<RawCapacityCapture, String> {
        let (page, captured_at_ms) = capture_bound_page(id)?;
        Ok(RawCapacityCapture {
            device_id: id.into(),
            captured_at_ms,
            page_hex: page.iter().map(|byte| format!("{byte:02x}")).collect(),
            unit_validation: "unverified",
            message: "Raw read-only capacity page for laboratory verification. No field is certified as a byte count.",
        })
    }

    fn capacity_from_capture(
        id: &str,
        capture: impl FnOnce(&str) -> Result<(Vec<u8>, i64), String>,
        clock: impl FnOnce() -> Result<i64, String>,
    ) -> CapacityResponse {
        let result = (|| {
            let (page, captured_at) = capture(id)?;
            // No model has completed the spare-unit byte-scale validation yet.
            // Raw captures remain available without bypassing the decoder gate.
            let record = decode_capacity(
                &page,
                CapacityConfirmation {
                    byte_units: false,
                    same_device: true,
                    single_volume: true,
                },
                captured_at,
                clock()?,
            )
            .map_err(|error| error.to_string())?;
            Ok(CapacityResponse {
                device_id: id.into(),
                state: "available",
                protected_total_bytes: Some(record.protected_total_bytes.to_string()),
                protected_used_bytes: Some(record.protected_used_bytes.to_string()),
                protected_free_bytes: Some(record.protected_free_bytes.to_string()),
                observed_at_ms: Some(record.captured_at_ms),
                message: "Validated device-reported protected capacity.".into(),
            })
        })();
        result.unwrap_or_else(|error: String| CapacityResponse::unavailable(id, error))
    }

    pub fn capacity(id: &str) -> CapacityResponse {
        capacity_from_capture(id, capture_bound_page, now_ms)
    }

    fn now_ms() -> Result<i64, String> {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Invalid system clock".to_string())?;
        i64::try_from(elapsed.as_millis())
            .map_err(|_| "System clock exceeds supported range".to_string())
    }

    fn explain(status: i32) -> String {
        match status {
            -10002 => "The selected device disconnected.".into(),
            -10003 => {
                "A recognized Drobo management driver and mounted volume are required.".into()
            }
            -10004 | -10005 => {
                "The driver returned an incomplete or unsupported capacity response.".into()
            }
            -10007 => {
                "Multiple volumes share this device; per-volume accounting is not yet supported."
                    .into()
            }
            _ => format!(
                "Native capacity interface unavailable (0x{:08x}). Driver or access policy needs validation.",
                status as u32
            ),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::cell::RefCell;

        fn valid_page() -> Vec<u8> {
            let mut page = vec![0x7a, 0x02, 0, 32];
            for value in [60_u64, 40, 100, 100] {
                page.extend(value.to_be_bytes());
            }
            page
        }

        #[test]
        fn capture_seam_keeps_scope_capture_rebind_order() {
            let calls = RefCell::new(Vec::new());
            let page = capture_bound_page_with(
                "7",
                |id, path, count| {
                    calls.borrow_mut().push("scope");
                    assert_eq!(id, 7);
                    path[0] = b'/' as c_char;
                    *count = 1;
                    Ok(())
                },
                |id, page, length| {
                    calls.borrow_mut().push("capture");
                    assert_eq!(id, 7);
                    page[..4].copy_from_slice(&[0x7a, 0x02, 0, 32]);
                    *length = 36;
                    Ok(())
                },
                |path, bound_id, count| {
                    calls.borrow_mut().push("rebind");
                    assert_eq!(path[0], b'/' as c_char);
                    *bound_id = 7;
                    *count = 1;
                    Ok(())
                },
                || Ok(100),
            )
            .unwrap();
            assert_eq!(calls.into_inner(), ["scope", "capture", "rebind"]);
            assert_eq!(page.1, 100);
            assert_eq!(page.0.len(), 36);
        }

        #[test]
        fn capture_seam_rejects_scope_and_capture_failures_before_rebind() {
            let scope = capture_bound_page_with(
                "7",
                |_, _, _| Err("scope failed".into()),
                |_, _, _| panic!("capture must not run"),
                |_, _, _| panic!("rebind must not run"),
                || Ok(100),
            );
            assert_eq!(scope.unwrap_err(), "scope failed");

            let capture = capture_bound_page_with(
                "7",
                |_, _, count| {
                    *count = 1;
                    Ok(())
                },
                |_, _, _| Err("capture failed".into()),
                |_, _, _| panic!("rebind must not run"),
                || Ok(100),
            );
            assert_eq!(capture.unwrap_err(), "capture failed");
        }

        #[test]
        fn capture_seam_refuses_ambiguous_scope_oversized_page_and_changed_binding() {
            for count in [0, 2] {
                let ambiguous = capture_bound_page_with(
                    "7",
                    |_, _, observed| {
                        *observed = count;
                        Ok(())
                    },
                    |_, _, _| panic!("capture must not run"),
                    |_, _, _| panic!("rebind must not run"),
                    || Ok(100),
                );
                assert!(ambiguous.unwrap_err().contains("unique mounted volume"));
            }

            let oversized = capture_bound_page_with(
                "7",
                |_, _, count| {
                    *count = 1;
                    Ok(())
                },
                |_, page, length| {
                    *length = page.len() + 1;
                    Ok(())
                },
                |_, _, _| panic!("rebind must not run"),
                || Ok(100),
            );
            assert_eq!(
                oversized.unwrap_err(),
                "Capacity response exceeded its buffer"
            );

            let changed = capture_bound_page_with(
                "7",
                |_, _, count| {
                    *count = 1;
                    Ok(())
                },
                |_, _, length| {
                    *length = 0;
                    Ok(())
                },
                |_, bound_id, count| {
                    *bound_id = 8;
                    *count = 1;
                    Ok(())
                },
                || Ok(100),
            );
            assert_eq!(
                changed.unwrap_err(),
                "Device or volume binding changed during the capacity read."
            );
        }

        #[test]
        fn byte_units_remain_disabled_and_capacity_is_unavailable() {
            let response = capacity_from_capture("7", |_| Ok((valid_page(), 100)), || Ok(100));
            assert_eq!(response.state, "unavailable");
            assert_eq!(response.message, "capacity byte units are unconfirmed");
            assert!(response.protected_total_bytes.is_none());
        }
    }
}

pub fn inventory() -> Result<Inventory, String> {
    #[cfg(target_os = "macos")]
    {
        macos::inventory()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(Inventory { platform: std::env::consts::OS, devices: vec![],
        message: "Shared UI and Rust core. Device discovery/transport for this OS is not implemented yet.".into() })
    }
}

pub fn capacity(id: &str) -> CapacityResponse {
    #[cfg(target_os = "macos")]
    {
        macos::capacity(id)
    }
    #[cfg(not(target_os = "macos"))]
    {
        CapacityResponse::unavailable(id, "Device transport for this OS is not implemented yet.")
    }
}

pub fn raw_capture(id: &str) -> Result<RawCapacityCapture, String> {
    #[cfg(target_os = "macos")]
    {
        macos::raw_capture(id)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = id;
        Err("Device transport for this OS is not implemented yet.".into())
    }
}
