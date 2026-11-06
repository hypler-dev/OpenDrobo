//! Fixed, GUI-only management commands recovered from the original macOS service.
//! There is no arbitrary command interface and no automatic retry after a send.

use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const SERVICE_HASH: &str = "6348d0374c0193c628380426bb7c9a814c41f67bc02c4304c0eaf4116efd5732";
const SERVICE_PATH: &str =
    "/Library/Application Support/Data Robotics/Drobo Dashboard/DDService64d";
static CONTROL_LOCK: Mutex<()> = Mutex::new(());
// Original Dashboard kDimmingInfo maps its ten presets to these device values.
const DIM_PRESETS: [u16; 10] = [1, 2, 5, 7, 9, 11, 15, 25, 40, 59];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Identify,
    Brightness,
    GetBrightness,
    Restart,
    Shutdown,
    Standby,
}

impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Identify => "identify",
            Self::Brightness => "brightness",
            Self::GetBrightness => "get-brightness",
            Self::Restart => "restart",
            Self::Shutdown => "shutdown",
            Self::Standby => "standby",
        }
    }

    fn command(self) -> u8 {
        match self {
            Self::Identify => 26,
            Self::Brightness => 8,
            Self::GetBrightness => 9,
            Self::Restart => 21,
            Self::Shutdown => 23,
            Self::Standby => 22,
        }
    }

    fn power(self) -> bool {
        matches!(self, Self::Restart | Self::Shutdown | Self::Standby)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    unit_id: String,
    action: Action,
    value: Option<u16>,
    confirmation: Option<String>,
}

fn verify_service() -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("Device controls require the supported original macOS service.".into());
    }
    let mut file = std::fs::File::open(SERVICE_PATH)
        .map_err(|_| "The analyzed original service is not installed.".to_string())?;
    let metadata = file
        .metadata()
        .map_err(|_| "Original service verification failed.".to_string())?;
    if !metadata.is_file() || metadata.len() != 7_271_776 {
        return Err("This original service version has not been validated for controls.".into());
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0; 65_536];
    let mut count = 0u64;
    loop {
        let size = file
            .read(&mut buffer)
            .map_err(|_| "Original service verification failed.".to_string())?;
        if size == 0 {
            break;
        }
        count += size as u64;
        if count > metadata.len() {
            return Err("Original service changed during verification.".into());
        }
        hasher.update(&buffer[..size]);
    }
    if count != metadata.len() || format!("{:x}", hasher.finalize()) != SERVICE_HASH {
        return Err("This original service version has not been validated for controls.".into());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn mounted_volumes() -> Result<u32, String> {
    unsafe extern "C" {
        fn od_mounted_drobo_count(count: *mut u32) -> i32;
    }
    let mut count = 0;
    if unsafe { od_mounted_drobo_count(&mut count) } != 0 {
        return Err("Mounted Drobo volume state is unknown; power commands are blocked.".into());
    }
    Ok(count)
}

#[cfg(not(target_os = "macos"))]
fn mounted_volumes() -> Result<u32, String> {
    Err("Mounted Drobo volume inspection is unavailable on this platform.".into())
}

fn power_guard(volumes: Result<u32, String>) -> Result<(), String> {
    match volumes {
        Ok(0) => Ok(()),
        Ok(_) => Err("Eject all mounted Drobo volumes before requesting a power operation.".into()),
        Err(error) => Err(error),
    }
}

fn supported(target: &crate::service::ControlTarget) -> bool {
    crate::service::is_5d_model(&target.model) && target.connection_type == 6
}

pub fn status() -> Result<Value, String> {
    let verification = verify_service();
    let snapshot = crate::service::status()?;
    let pools = snapshot["pools"]
        .as_array()
        .ok_or_else(|| "Device control targets are unavailable.".to_string())?;
    let power = power_guard(mounted_volumes());
    let targets: Vec<Value> = pools.iter().map(|pool| {
        let compatible = pool["model"].as_str().is_some_and(crate::service::is_5d_model)
            && pool["connectionType"].as_u64() == Some(6);
        let common_reason = verification.as_ref().err().cloned().or_else(||
            (!compatible).then(|| "This model's command path has not been validated.".to_string()));
        let controls: Vec<Value> = [
            (Action::Identify, "Identify lights"),
            (Action::Brightness, "LED brightness"),
            (Action::GetBrightness, "Read LED brightness"),
            (Action::Restart, "Restart unit"),
            (Action::Shutdown, "Shut down unit"),
            (Action::Standby, "Standby unit"),
        ].into_iter().map(|(action,label)| {
            let reason = common_reason.clone()
                .or_else(|| action.power().then(|| power.as_ref().err().cloned()).flatten());
            json!({"action":action.name(),"label":label,"enabled":reason.is_none(),
                "reason":reason.unwrap_or_else(|| "Recovered original-service command; hardware behavior is not yet verified.".into()),
                "requiresConfirmation":action.power(),
                "minimum":if action == Action::Brightness {json!(0)} else {Value::Null},
                "maximum":if action == Action::Brightness {json!(9)} else {Value::Null}})
        }).collect();
        json!({"id":pool["id"],"brand":"Drobo","model":pool["model"],"controls":controls})
    }).collect();
    Ok(
        json!({"state":if verification.is_ok(){"available"}else{"unavailable"},"targets":targets,
        "message":"Controls use the original local service. Power commands require ejected volumes, explicit confirmation and the service's own safety checks. No command is sent by this inspection."}),
    )
}

fn encode(request: &Request, target: &crate::service::ControlTarget) -> Result<Vec<u8>, String> {
    if !supported(target) {
        return Err("This model or connection has not been validated for controls.".into());
    }
    if request.unit_id != target.public_id
        || target.esa_id.is_empty()
        || target.esa_id.len() > 128
        || !target
            .esa_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("Device control target changed; refresh the device list.".into());
    }
    if request.action.power()
        && request.confirmation.as_deref() != Some(request.action.name().to_uppercase().as_str())
    {
        return Err("Power operation requires an explicit matching confirmation.".into());
    }
    if request.value.is_some() && request.action != Action::Brightness {
        return Err("This command does not accept a value.".into());
    }
    let params = match request.action {
        Action::Identify => {
            // Match the original Dashboard's feature-dependent default. Units are unknown.
            let interval = if target.firmware_features & 0x1000_0000 != 0 {
                0
            } else {
                900
            };
            format!("<IdentifyInterval>{interval}</IdentifyInterval>")
        }
        Action::Restart => "<ForceRestart>0</ForceRestart>".into(),
        Action::Shutdown => "<ForceShutdown>0</ForceShutdown>".into(),
        Action::Standby | Action::GetBrightness => String::new(),
        Action::Brightness => {
            let preset = request
                .value
                .and_then(|index| DIM_PRESETS.get(index as usize))
                .ok_or_else(|| {
                    "Choose one of the ten recovered LED presets (0 through 9).".to_string()
                })?;
            format!("<Value>{preset}</Value>")
        }
    };
    let body = format!(
        "<TMCmd><CmdID>{}</CmdID><ESAID>{}</ESAID><Params>{params}</Params></TMCmd>\0",
        request.action.command(),
        target.esa_id
    );
    let mut bytes = b"DRIDDSVC\x03\x01\x00\x00".to_vec();
    bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
    bytes.extend_from_slice(body.as_bytes());
    Ok(bytes)
}

fn read_exact_deadline(
    stream: &mut std::net::TcpStream,
    output: &mut [u8],
    deadline: Instant,
) -> Result<(), ()> {
    let mut offset = 0;
    while offset < output.len() {
        if Instant::now() >= deadline {
            return Err(());
        }
        match stream.read(&mut output[offset..]) {
            Ok(0) => return Err(()),
            Ok(count) => offset += count,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err(()),
        }
    }
    Ok(())
}

fn exchange_at(
    address: std::net::SocketAddr,
    bytes: &[u8],
    command: u8,
) -> Result<(u64, Option<String>), String> {
    let mut stream = std::net::TcpStream::connect_timeout(&address, Duration::from_secs(1))
        .map_err(|_| "Command was not sent: the local service is unavailable.".to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .map_err(|_| "Command was not sent: socket configuration failed.".to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .map_err(|_| "Command was not sent: socket configuration failed.".to_string())?;
    let unknown = "Command outcome is unknown. Do not retry automatically; inspect the unit and refresh its status.";
    stream.write_all(bytes).map_err(|_| unknown.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut header = [0; 16];
    read_exact_deadline(&mut stream, &mut header, deadline).map_err(|_| unknown.to_string())?;
    if &header[..12] != b"DRIDDSVC\x06\x01\x00\x00" {
        return Err(unknown.into());
    }
    let length = u32::from_be_bytes(header[12..].try_into().expect("header size")) as usize;
    if !(2..=16_384).contains(&length) {
        return Err(unknown.into());
    }
    let mut body = vec![0; length];
    read_exact_deadline(&mut stream, &mut body, deadline).map_err(|_| unknown.to_string())?;
    if body.last() != Some(&0) || body[..length - 1].contains(&0) {
        return Err(unknown.into());
    }
    crate::service::command_result(&body[..length - 1], command).map_err(|_| unknown.to_string())
}

pub fn execute(request: Request) -> Result<Value, String> {
    let _lock = CONTROL_LOCK
        .try_lock()
        .map_err(|_| "Another device command is in progress.".to_string())?;
    verify_service()?;
    crate::service::with_control_target(&request.unit_id, |target| {
        let bytes = encode(&request, &target)?;
        if request.action.power() {
            power_guard(mounted_volumes())?;
        }
        crate::diagnostics::record(
            crate::diagnostics::Level::Info,
            "device",
            "control-request",
            "A user-confirmed, allowlisted device control was requested.",
        );
        let reply = exchange_at(
            "127.0.0.1:4502".parse().expect("fixed loopback"),
            &bytes,
            request.action.command(),
        );
        let response = match reply {
            Ok((0, details)) => {
                let value = if request.action == Action::GetBrightness {
                    details.and_then(|s| s.parse::<u16>().ok()).map(u64::from)
                } else {
                    None
                };
                json!({"state":"accepted","action":request.action.name(),"value":value,
                "message":if request.action == Action::GetBrightness {"The original service returned the LED brightness value; its scale remains under validation."}
                    else {"The original service accepted the request. Refresh device status to observe the result."}})
            }
            Ok((_, _)) => {
                json!({"state":"rejected","action":request.action.name(),"message":"The original service refused the command. No automatic retry was made."})
            }
            Err(message) => {
                json!({"state":"unknown","action":request.action.name(),"message":message})
            }
        };
        crate::diagnostics::record(
            crate::diagnostics::Level::Info,
            "device",
            "control-result",
            "Device control finished; service acceptance is separate from observed hardware behavior.",
        );
        Ok(response)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target() -> crate::service::ControlTarget {
        crate::service::ControlTarget {
            esa_id: "SYNTHETIC_UNIT".into(),
            public_id: "pool-synthetic".into(),
            model: "Drobo 5D".into(),
            connection_type: 6,
            firmware_features: 0x1000_0000,
        }
    }
    fn request(action: Action) -> Request {
        Request {
            unit_id: "pool-synthetic".into(),
            action,
            value: None,
            confirmation: None,
        }
    }
    #[test]
    fn allowlist_and_target_cannot_be_bypassed() {
        assert!(serde_json::from_value::<Request>(json!({"unitId":"x","action":"reset"})).is_err());
        assert!(
            serde_json::from_value::<Request>(
                json!({"unitId":"x","action":"identify","command":89})
            )
            .is_err()
        );
        let mut t = target();
        t.esa_id = "<other>".into();
        assert!(encode(&request(Action::Identify), &t).is_err());
        let mut r = request(Action::Identify);
        r.unit_id = "other".into();
        assert!(encode(&r, &target()).is_err());
        let mut t = target();
        t.model = "Drobo 5N".into();
        assert!(encode(&request(Action::Identify), &t).is_err());
        t.model = "5D".into();
        assert!(supported(&t));
        assert!(encode(&request(Action::Identify), &t).is_ok());
        t.connection_type = 1;
        assert!(!supported(&t));
    }
    #[test]
    fn power_never_forces_or_accepts_busy_unknown_volumes() {
        for action in [Action::Restart, Action::Shutdown, Action::Standby] {
            let mut r = request(action);
            assert!(encode(&r, &target()).is_err());
            r.confirmation = Some(action.name().to_uppercase());
            let bytes = encode(&r, &target()).unwrap();
            assert_eq!(&bytes[..12], b"DRIDDSVC\x03\x01\x00\x00");
            assert_eq!(
                u32::from_be_bytes(bytes[12..16].try_into().unwrap()) as usize,
                bytes.len() - 16
            );
            let body = std::str::from_utf8(&bytes[16..]).unwrap();
            assert!(!body.contains(">1</Force"));
            assert!(body.ends_with('\0'));
        }
        assert!(power_guard(Ok(0)).is_ok());
        assert!(power_guard(Ok(1)).is_err());
        assert!(power_guard(Err("unknown".into())).is_err());
    }
    #[test]
    fn identify_matches_original_feature_branch_and_brightness_uses_fixed_presets() {
        let mut t = target();
        let b = encode(&request(Action::Identify), &t).unwrap();
        assert!(
            String::from_utf8(b)
                .unwrap()
                .contains("<IdentifyInterval>0</IdentifyInterval>")
        );
        t.firmware_features = 0;
        let b = encode(&request(Action::Identify), &t).unwrap();
        assert!(
            String::from_utf8(b)
                .unwrap()
                .contains("<IdentifyInterval>900</IdentifyInterval>")
        );
        let mut r = request(Action::Brightness);
        r.value = Some(65535);
        assert!(encode(&r, &t).is_err());
        for (index, value) in DIM_PRESETS.iter().enumerate() {
            r.value = Some(index as u16);
            let b = encode(&r, &t).unwrap();
            assert!(
                String::from_utf8(b)
                    .unwrap()
                    .contains(&format!("<Value>{value}</Value>"))
            );
        }
    }
    #[test]
    fn bounded_mock_reply_requires_original_result_and_matching_command() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let bytes = encode(&request(Action::GetBrightness), &target()).unwrap();
        let expected = bytes.clone();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut got = vec![0; expected.len()];
            stream.read_exact(&mut got).unwrap();
            assert_eq!(got, expected);
            let body=b"<TMCmd><CmdID>9</CmdID><Result>0</Result><ResultDetails>60</ResultDetails></TMCmd>\0";
            let mut frame = b"DRIDDSVC\x06\x01\x00\x00".to_vec();
            frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
            frame.extend_from_slice(body);
            stream.write_all(&frame).unwrap();
        });
        assert_eq!(
            exchange_at(address, &bytes, 9).unwrap(),
            (0, Some("60".into()))
        );
        worker.join().unwrap();
    }
}
