//! Read-only original Drobo service observer. This module never manages a device.

use quick_xml::{Reader, events::Event};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(target_os = "macos")]
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "macos")]
static STATUS_READ: Mutex<()> = Mutex::new(());
#[cfg(target_os = "macos")]
static STATUS_CACHE: OnceLock<Mutex<Option<(std::time::Instant, Value)>>> = OnceLock::new();

#[cfg(target_os = "macos")]
fn management_transaction<T>(operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let _guard = STATUS_READ
        .lock()
        .map_err(|_| ERR_UNAVAILABLE.to_string())?;
    operation()
}

const MAX_XML_BYTES: usize = 524_288;
const MAX_FRAMES: usize = 16;
const MAX_TOTAL_BYTES: usize = 2 * 1024 * 1024;
const MAX_DEPTH: usize = 16;
const MAX_NODES: usize = 4_096;

const ERR_UNAVAILABLE: &str = "Original Drobo service status is unavailable.";
const ERR_PROTOCOL: &str = "Original Drobo service returned an invalid status response.";
const ERR_INCOMPLETE: &str = "Original Drobo service did not provide a complete status snapshot.";
const ERR_NO_UNIT: &str = "Original Drobo service reports no active unit.";

#[derive(Debug)]
struct Node {
    name: String,
    text: String,
    children: Vec<Node>,
}

#[derive(Debug)]
struct Update {
    // Service correlation only. Never serialize this serial-derived identifier.
    id: String,
    model: String,
    firmware_version: Option<String>,
    firmware_release_date: Option<String>,
    connection_type: u32,
    firmware_features: u64,
    status: u64,
    status_ex: u64,
    disk_pack_status: u64,
    lun_count: u32,
    protected_total: u64,
    protected_used: u64,
    protected_free: u64,
    os_used: u64,
    volumes: Vec<Volume>,
    slots: Vec<Slot>,
}

#[derive(Debug)]
struct Volume {
    lun: u32,
    logical_total: u64,
    os_used: u64,
}

#[derive(Debug)]
struct Slot {
    slot: u32,
    kind: &'static str,
    status: u64,
    disk_state: u64,
    disk_type: u32,
    physical_capacity: u64,
    ssd_life_remaining: Option<u8>,
    manufacturer_model: Option<String>,
    firmware_revision: Option<String>,
}

/// A fresh, service-validated control destination. This is crate-internal only.
/// The serial-derived ESA ID must never cross an application boundary.
pub(crate) struct ControlTarget {
    pub esa_id: String,
    pub public_id: String,
    pub model: String,
    pub connection_type: u32,
    pub firmware_features: u64,
}

/// Read one bounded, read-only snapshot from the original macOS DDService.
#[cfg(target_os = "macos")]
pub fn status() -> Result<Value, String> {
    management_transaction(|| {
        use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
        let cache = STATUS_CACHE.get_or_init(|| Mutex::new(None));
        if let Ok(held) = cache.lock()
            && let Some((received, snapshot)) = held.as_ref()
            && received.elapsed() < std::time::Duration::from_secs(5)
        {
            return Ok(snapshot.clone());
        }
        let result = status_from_addr(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 4502)));
        match &result {
            Ok(snapshot) => {
                crate::device_events::observe(snapshot.clone());
                crate::capacity_history::observe(snapshot);
                if let Ok(mut held) = cache.lock() {
                    *held = Some((std::time::Instant::now(), snapshot.clone()));
                }
            }
            Err(error) if error == ERR_NO_UNIT => crate::device_events::unavailable(),
            Err(_) => crate::device_events::read_error(),
        }
        result.map_err(|error| {
            if error == ERR_NO_UNIT {
                ERR_UNAVAILABLE.into()
            } else {
                error
            }
        })
    })
}

/// This platform cannot use the original macOS local-status service.
#[cfg(not(target_os = "macos"))]
pub fn status() -> Result<Value, String> {
    Err("Original Drobo service status is unavailable on this platform; macOS original-service support is planned.".into())
}

/// Hold the same transaction lock as GUI status polling through fresh lookup and its command.
#[cfg(target_os = "macos")]
pub(crate) fn with_control_target<T>(
    public_id: &str,
    perform: impl FnOnce(ControlTarget) -> Result<T, String>,
) -> Result<T, String> {
    management_transaction(|| {
        use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
        if let Ok(mut held) = STATUS_CACHE.get_or_init(|| Mutex::new(None)).lock() {
            *held = None;
        }
        let updates =
            updates_from_addr(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 4502)))?;
        perform(control_target_from_updates(updates, public_id)?)
    })
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn with_control_target<T>(
    _: &str,
    _: impl FnOnce(ControlTarget) -> Result<T, String>,
) -> Result<T, String> {
    Err("Original Drobo service control target is unavailable on this platform.".into())
}

fn parse_xml(bytes: &[u8]) -> Result<Node, String> {
    if bytes.is_empty() || bytes.len() > MAX_XML_BYTES || !std::str::from_utf8(bytes).is_ok() {
        return Err(ERR_PROTOCOL.into());
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut stack: Vec<Node> = Vec::new();
    let mut root = None;
    let mut nodes = 0usize;
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(start)) => {
                if start.attributes().next().is_some()
                    || stack.len() >= MAX_DEPTH
                    || root.is_some() && stack.is_empty()
                {
                    return Err(ERR_PROTOCOL.into());
                }
                let qualified_name = start.name();
                let name = qualified_name.as_ref();
                if name.is_empty()
                    || name.len() > 64
                    || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    return Err(ERR_PROTOCOL.into());
                }
                nodes += 1;
                if nodes > MAX_NODES {
                    return Err(ERR_PROTOCOL.into());
                }
                stack.push(Node {
                    name: name.to_owned(),
                    text: String::new(),
                    children: Vec::new(),
                });
            }
            Ok(Event::End(end)) => {
                let qualified_name = end.name();
                let end_name = qualified_name.as_ref();
                let node = stack.pop().ok_or_else(|| ERR_PROTOCOL.to_string())?;
                if node.name != end_name {
                    return Err(ERR_PROTOCOL.into());
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else if root.replace(node).is_some() {
                    return Err(ERR_PROTOCOL.into());
                }
            }
            Ok(Event::Empty(start)) => {
                if start.attributes().next().is_some() || stack.len() >= MAX_DEPTH {
                    return Err(ERR_PROTOCOL.into());
                }
                let qualified_name = start.name();
                let name = qualified_name.as_ref();
                if name.is_empty()
                    || name.len() > 64
                    || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    return Err(ERR_PROTOCOL.into());
                }
                nodes += 1;
                if nodes > MAX_NODES {
                    return Err(ERR_PROTOCOL.into());
                }
                let node = Node {
                    name: name.to_owned(),
                    text: String::new(),
                    children: Vec::new(),
                };
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else if root.replace(node).is_some() {
                    return Err(ERR_PROTOCOL.into());
                }
            }
            Ok(Event::Text(text)) => {
                if text.as_ref().contains('&') {
                    return Err(ERR_PROTOCOL.into());
                }
                let value = text.as_ref();
                let node = stack.last_mut().ok_or_else(|| ERR_PROTOCOL.to_string())?;
                if node
                    .text
                    .len()
                    .checked_add(value.len())
                    .ok_or_else(|| ERR_PROTOCOL.to_string())?
                    > 1024
                {
                    return Err(ERR_PROTOCOL.into());
                }
                node.text.push_str(value);
            }
            Ok(Event::Decl(_)) | Ok(Event::Comment(_)) => {}
            Ok(Event::Eof) => break,
            Ok(Event::DocType(_))
            | Ok(Event::GeneralRef(_))
            | Ok(Event::CData(_))
            | Ok(Event::PI(_)) => return Err(ERR_PROTOCOL.into()),
            Err(_) => return Err(ERR_PROTOCOL.into()),
        }
        buffer.clear();
    }
    if !stack.is_empty() {
        return Err(ERR_PROTOCOL.into());
    }
    root.ok_or_else(|| ERR_PROTOCOL.to_string())
}

fn direct<'a>(node: &'a Node, name: &str) -> impl Iterator<Item = &'a Node> {
    node.children.iter().filter(move |child| child.name == name)
}

fn one_text<'a>(node: &'a Node, name: &str) -> Result<&'a str, String> {
    let mut matches = direct(node, name);
    let field = matches.next().ok_or_else(|| ERR_PROTOCOL.to_string())?;
    if matches.next().is_some() || !field.children.is_empty() {
        return Err(ERR_PROTOCOL.into());
    }
    Ok(field.text.trim())
}

fn canonical_u64(text: &str) -> Result<u64, String> {
    if text.is_empty()
        || text.len() > 20
        || !text.bytes().all(|byte| byte.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(ERR_PROTOCOL.into());
    }
    text.parse().map_err(|_| ERR_PROTOCOL.to_string())
}

fn canonical_u32(text: &str) -> Result<u32, String> {
    let value = canonical_u64(text)?;
    u32::try_from(value).map_err(|_| ERR_PROTOCOL.to_string())
}

fn safe_model(text: &str) -> Result<String, String> {
    if text.is_empty()
        || text.chars().count() > 100
        || !text.chars().all(|c| c.is_ascii_graphic() || c == ' ')
    {
        return Err(ERR_PROTOCOL.into());
    }
    Ok(text.to_owned())
}

/// Optional display metadata remains a bounded, unique leaf. Missing or blank
/// firmware fields are unknown; they are not invented from device status bits.
fn optional_display_text(node: &Node, name: &str) -> Result<Option<String>, String> {
    let mut fields = direct(node, name);
    let Some(field) = fields.next() else {
        return Ok(None);
    };
    if fields.next().is_some() || !field.children.is_empty() {
        return Err(ERR_PROTOCOL.into());
    }
    let text = field.text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    safe_model(text).map(Some)
}

fn numbered_children<'a>(node: &'a Node) -> Result<Vec<&'a Node>, String> {
    let mut records = Vec::new();
    for child in &node.children {
        if let Some(number) = child.name.strip_prefix('n') {
            let index = canonical_u32(number)?;
            if index > 31 {
                return Err(ERR_PROTOCOL.into());
            }
            records.push(child);
        }
    }
    if records.len() > 32 {
        return Err(ERR_PROTOCOL.into());
    }
    Ok(records)
}

fn parse_slots(root: &Node) -> Result<Vec<Slot>, String> {
    let mut containers = direct(root, "mSlotsExp");
    let Some(container) = containers.next() else {
        return Ok(Vec::new());
    };
    if containers.next().is_some() {
        return Err(ERR_PROTOCOL.into());
    }
    let mut seen = BTreeSet::new();
    let mut slots = Vec::new();
    for record in numbered_children(container)? {
        let slot = canonical_u32(one_text(record, "mSlotNumber")?)?;
        if slot > 31 || !seen.insert(slot) {
            return Err(ERR_PROTOCOL.into());
        }
        let disk_type = canonical_u32(one_text(record, "mDiskType")?)?;
        let status = canonical_u64(one_text(record, "mStatus")?)?;
        let disk_state = canonical_u64(one_text(record, "mDiskState")?)?;
        let physical_capacity = canonical_u64(one_text(record, "mPhysicalCapacity")?)?;
        let life = match direct(record, "SSDLifeRemaining").next() {
            None => None,
            Some(value)
                if direct(record, "SSDLifeRemaining").count() == 1 && value.children.is_empty() =>
            {
                let percent = canonical_u64(value.text.trim())?;
                Some(u8::try_from(percent).map_err(|_| ERR_PROTOCOL.to_string())?)
            }
            Some(_) => return Err(ERR_PROTOCOL.into()),
        };
        slots.push(Slot {
            slot,
            kind: if disk_type == 4 { "ssd" } else { "drive" },
            status,
            disk_state,
            disk_type,
            physical_capacity,
            manufacturer_model: optional_display_text(record, "mMake")?,
            firmware_revision: optional_display_text(record, "mDiskFwRev")?,
            ssd_life_remaining: if disk_type == 4 {
                life.filter(|value| *value <= 100)
            } else {
                None
            },
        });
    }
    slots.sort_by_key(|slot| slot.slot);
    Ok(slots)
}

fn parse_volumes(root: &Node, expected_count: u32) -> Result<Vec<Volume>, String> {
    let mut containers = direct(root, "mLUNUpdates");
    let container = containers.next().ok_or_else(|| ERR_PROTOCOL.to_string())?;
    if containers.next().is_some() {
        return Err(ERR_PROTOCOL.into());
    }
    let mut seen = BTreeSet::new();
    let mut volumes = Vec::new();
    for record in numbered_children(container)? {
        let lun = canonical_u32(one_text(record, "mLUN")?)?;
        if lun > 31 || !seen.insert(lun) {
            return Err(ERR_PROTOCOL.into());
        }
        volumes.push(Volume {
            lun,
            logical_total: canonical_u64(one_text(record, "mMaximumLUNSize")?)?,
            os_used: canonical_u64(one_text(record, "mUsedCapacityOS")?)?,
        });
    }
    if expected_count > 32 || volumes.len() != expected_count as usize {
        return Err(ERR_PROTOCOL.into());
    }
    volumes.sort_by_key(|volume| volume.lun);
    Ok(volumes)
}

fn opaque_esa_id(text: &str) -> Result<String, String> {
    if text.is_empty()
        || text.len() > 128
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(ERR_PROTOCOL.into());
    }
    Ok(text.to_owned())
}

fn zero_esa_id(id: &str) -> bool {
    id.bytes().all(|byte| byte == b'0')
}

/// Parse a bounded original-service command result after its transport frame is verified.
/// Details stay crate-internal; callers must not publish them on errors.
pub(crate) fn command_result(
    bytes: &[u8],
    expected_command: u8,
) -> Result<(u64, Option<String>), String> {
    let root = parse_xml(bytes)?;
    let command = canonical_u64(one_text(&root, "CmdID")?)?;
    if command != u64::from(expected_command) {
        return Err(ERR_PROTOCOL.into());
    }
    let result = canonical_u64(one_text(&root, "Result")?)?;
    let mut details = direct(&root, "ResultDetails");
    let details = match details.next() {
        None => None,
        Some(value) if details.next().is_none() && value.children.is_empty() => {
            let text = value.text.trim();
            if text.len() > 1024 || text.chars().any(char::is_control) {
                return Err(ERR_PROTOCOL.into());
            }
            Some(text.to_owned())
        }
        Some(_) => return Err(ERR_PROTOCOL.into()),
    };
    Ok((result, details))
}

fn parse_update(root: &Node) -> Result<Update, String> {
    let id = opaque_esa_id(one_text(root, "mESAID")?)?;
    if zero_esa_id(&id) {
        return Err(ERR_PROTOCOL.into());
    }
    let _version = canonical_u64(one_text(root, "mESAUpdateVersion")?)?;
    let connection_type = canonical_u32(one_text(root, "mConnectionType")?)?;
    let firmware_features = canonical_u64(one_text(root, "mFirmwareFeatures")?)?;
    let status = canonical_u64(one_text(root, "mStatus")?)?;
    let status_ex = canonical_u64(one_text(root, "mStatusEx")?)?;
    let disk_pack_status = canonical_u64(one_text(root, "mDiskPackStatus")?)?;
    let lun_count = canonical_u32(one_text(root, "mLUNCount")?)?;
    let protected_total = canonical_u64(one_text(root, "mTotalCapacityProtected")?)?;
    let protected_used = canonical_u64(one_text(root, "mUsedCapacityProtected")?)?;
    let protected_free = canonical_u64(one_text(root, "mFreeCapacityProtected")?)?;
    if protected_total == 0 || protected_used.checked_add(protected_free) != Some(protected_total) {
        return Err(ERR_PROTOCOL.into());
    }
    Ok(Update {
        id,
        model: safe_model(one_text(root, "mModel")?)?,
        firmware_version: optional_display_text(root, "mVersion")?,
        firmware_release_date: optional_display_text(root, "mReleaseDate")?,
        connection_type,
        firmware_features,
        status,
        status_ex,
        disk_pack_status,
        lun_count,
        protected_total,
        protected_used,
        protected_free,
        os_used: canonical_u64(one_text(root, "mUsedCapacityOS")?)?,
        volumes: parse_volumes(root, lun_count)?,
        slots: parse_slots(root)?,
    })
}

fn advertised_ids(root: &Node) -> Result<Vec<String>, String> {
    let mut ids = BTreeSet::new();
    for id in direct(root, "mESAID") {
        if !id.children.is_empty() {
            return Err(ERR_PROTOCOL.into());
        }
        // Original DDService emits an empty list placeholder before actual IDs.
        if id.text.trim().is_empty() {
            continue;
        }
        let value = opaque_esa_id(id.text.trim())?;
        if !zero_esa_id(&value) {
            ids.insert(value);
        }
        if ids.len() > 32 {
            return Err(ERR_PROTOCOL.into());
        }
    }
    Ok(ids.into_iter().collect())
}

fn public_id(esa_id: &str) -> String {
    let digest = Sha256::digest(esa_id.as_bytes());
    format!("opendrobo/device/{digest:x}")
}

fn pool_json(update: Update, _position: usize, observed_at_ms: u64) -> Value {
    json!({
        "id": public_id(&update.id),
        "brand": "Drobo",
        "model": update.model,
        "firmwareVersion": update.firmware_version,
        "firmwareReleaseDate": update.firmware_release_date,
        "connectionType": update.connection_type,
        "unitState": "active",
        "deviceStatus": {
            "mStatus": update.status.to_string(),
            "mStatusEx": update.status_ex.to_string(),
            "mDiskPackStatus": update.disk_pack_status.to_string(),
        },
        "lunCount": update.lun_count,
        "protectedTotalBytes": update.protected_total.to_string(),
        "protectedUsedBytes": update.protected_used.to_string(),
        "protectedFreeBytes": update.protected_free.to_string(),
        "osUsedBytes": update.os_used.to_string(),
        "volumes": update.volumes.into_iter().map(|volume| json!({
            "lun": volume.lun,
            "logicalTotalBytes": volume.logical_total.to_string(),
            "osUsedBytes": volume.os_used.to_string(),
        })).collect::<Vec<_>>(),
        "slots": update.slots.into_iter().map(|slot| json!({
            "slot": slot.slot,
            "kind": slot.kind,
            "driveState": if slot.physical_capacity > 0 { "active" } else { "unknown" },
            "rawStatus": {
                "mStatus": slot.status.to_string(),
                "mDiskState": slot.disk_state.to_string(),
                "mDiskType": slot.disk_type.to_string(),
            },
            "physicalCapacityBytes": slot.physical_capacity.to_string(),
            "manufacturerModel": slot.manufacturer_model,
            "firmwareRevision": slot.firmware_revision,
            "ssdLifeRemainingPercent": slot.ssd_life_remaining,
            "cacheUsedBytes": Value::Null,
        })).collect::<Vec<_>>(),
        "_observedAtMs": observed_at_ms,
    })
}

fn result_json(updates: Vec<Update>, observed_at_ms: u64) -> Value {
    let mut pools: Vec<Value> = updates
        .into_iter()
        .enumerate()
        .map(|(position, update)| pool_json(update, position, observed_at_ms))
        .collect();
    for pool in &mut pools {
        pool.as_object_mut()
            .expect("pool object")
            .remove("_observedAtMs");
    }
    json!({
        "state": "available",
        "source": "original-service",
        "sourceTrust": "unverified-loopback",
        "observedAtMs": observed_at_ms,
        "unitValidation": "service-reported-bytes",
        "reportingScope": "shared-pool",
        "filesystemReportingEnabled": false,
        "pools": pools,
        "message": "Cached device-reported protected-pool and OS accounting are separate; this does not enable Finder.",
    })
}

fn observed_at_ms() -> Result<u64, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ERR_UNAVAILABLE.to_string())?
        .as_millis()
        .try_into()
        .map_err(|_| ERR_UNAVAILABLE.to_string())
}

fn control_target_from_updates(
    updates: Vec<Update>,
    requested_public_id: &str,
) -> Result<ControlTarget, String> {
    let update = updates
        .into_iter()
        .find(|update| public_id(&update.id) == requested_public_id)
        .ok_or_else(|| "Original Drobo service control target is unavailable.".to_string())?;
    if !is_5d_model(&update.model) {
        return Err("Original Drobo service control target is unsupported.".into());
    }
    Ok(ControlTarget {
        public_id: public_id(&update.id),
        esa_id: update.id,
        model: update.model,
        connection_type: update.connection_type,
        firmware_features: update.firmware_features,
    })
}

pub(crate) fn is_5d_model(model: &str) -> bool {
    matches!(model, "Drobo 5D" | "5D")
}

#[cfg(any(target_os = "macos", test))]
fn read_before_deadline(
    stream: &mut std::net::TcpStream,
    output: &mut [u8],
    deadline: std::time::Instant,
) -> Result<(), String> {
    use std::io::Read;
    let mut offset = 0;
    while offset < output.len() {
        if std::time::Instant::now() >= deadline {
            return Err(ERR_INCOMPLETE.into());
        }
        match stream.read(&mut output[offset..]) {
            Ok(0) => return Err(ERR_INCOMPLETE.into()),
            Ok(count) => offset += count,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err(ERR_INCOMPLETE.into()),
        }
    }
    Ok(())
}

#[cfg(any(target_os = "macos", test))]
fn updates_from_addr(address: std::net::SocketAddr) -> Result<Vec<Update>, String> {
    use std::io::Write;
    use std::net::TcpStream;
    use std::time::{Duration, Instant};
    let began = Instant::now();
    // The home service can defer a full update until its next polling cycle.
    let deadline = began + Duration::from_secs(35);
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))
        .map_err(|_| ERR_UNAVAILABLE.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .map_err(|_| ERR_UNAVAILABLE.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .map_err(|_| ERR_UNAVAILABLE.to_string())?;
    stream
        .write_all(b"DRIDDSVC\x01\x01\x00\x00\x00\x00\x00\x00")
        .map_err(|_| ERR_UNAVAILABLE.to_string())?;
    stream.flush().map_err(|_| ERR_UNAVAILABLE.to_string())?;
    let mut advertised: Option<Vec<String>> = None;
    let mut updates = BTreeMap::new();
    let mut total = 0usize;
    for _ in 0..MAX_FRAMES {
        let mut header = [0u8; 16];
        read_before_deadline(&mut stream, &mut header, deadline)?;
        if &header[..8] != b"DRIDDSVC"
            || header[8] != 7
            || header[9] != 1
            || !matches!(header[10], 1 | 3 | 4)
            || header[11] != 0
        {
            return Err(ERR_PROTOCOL.into());
        }
        let length = u32::from_be_bytes(header[12..16].try_into().expect("frame length")) as usize;
        if length == 0
            || length > MAX_XML_BYTES
            || total
                .checked_add(length)
                .ok_or_else(|| ERR_PROTOCOL.to_string())?
                > MAX_TOTAL_BYTES
        {
            return Err(ERR_PROTOCOL.into());
        }
        total += length;
        let mut xml = vec![0u8; length];
        read_before_deadline(&mut stream, &mut xml, deadline)?;
        if xml.last() == Some(&0) {
            xml.pop();
        }
        let root = parse_xml(&xml)?;
        match (header[10], root.name.as_str()) {
            (4, "ESATMDevices") => advertised = Some(advertised_ids(&root)?),
            (1 | 3, "ESATMUpdate") => {
                let update = parse_update(&root)?;
                updates.insert(update.id.clone(), update);
            }
            _ => return Err(ERR_PROTOCOL.into()),
        }
        if let Some(ids) = &advertised {
            if ids.is_empty() && updates.is_empty() {
                return Err(ERR_NO_UNIT.into());
            }
            if !ids.is_empty() && ids.iter().all(|id| updates.contains_key(id)) {
                let complete = ids.iter().filter_map(|id| updates.remove(id)).collect();
                return Ok(complete);
            }
        }
    }
    Err(ERR_INCOMPLETE.into())
}

#[cfg(any(target_os = "macos", test))]
fn status_from_addr(address: std::net::SocketAddr) -> Result<Value, String> {
    Ok(result_json(updates_from_addr(address)?, observed_at_ms()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
    use std::thread;

    #[test]
    #[cfg(target_os = "macos")]
    fn status_waits_for_entire_control_transaction() {
        use std::sync::mpsc;
        let (entered, entered_rx) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        let first = thread::spawn(move || {
            management_transaction(|| {
                entered.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(())
            })
        });
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        let (second_entered, second_rx) = mpsc::channel();
        let second = thread::spawn(move || {
            management_transaction(|| {
                second_entered.send(()).unwrap();
                Ok(())
            })
        });
        assert!(
            second_rx
                .recv_timeout(std::time::Duration::from_millis(25))
                .is_err()
        );
        release.send(()).unwrap();
        second_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
    }

    fn update_xml(id: &str, luns: &[(u32, u64, u64)]) -> String {
        let lun_count = luns.len();
        let luns = luns.iter().enumerate().map(|(n, (lun, total, used))| format!("<n{n}><mLUN>{lun}</mLUN><mMaximumLUNSize>{total}</mMaximumLUNSize><mUsedCapacityOS>{used}</mUsedCapacityOS></n{n}>")).collect::<String>();
        format!(
            "<ESATMUpdate><mESAID>{id}</mESAID><mESAUpdateVersion>1</mESAUpdateVersion><mModel>Drobo 5D</mModel><mConnectionType>1</mConnectionType><mFirmwareFeatures>268435456</mFirmwareFeatures><mStatus>1</mStatus><mStatusEx>2</mStatusEx><mDiskPackStatus>3</mDiskPackStatus><mLUNCount>{}</mLUNCount><mTotalCapacityProtected>1000</mTotalCapacityProtected><mUsedCapacityProtected>400</mUsedCapacityProtected><mFreeCapacityProtected>600</mFreeCapacityProtected><mUsedCapacityOS>450</mUsedCapacityOS><mSlotsExp><n0><mSlotNumber>5</mSlotNumber><mDiskType>4</mDiskType><mStatus>4</mStatus><mDiskState>5</mDiskState><mPhysicalCapacity>500</mPhysicalCapacity><SSDLifeRemaining>92</SSDLifeRemaining></n0></mSlotsExp><mLUNUpdates>{luns}</mLUNUpdates></ESATMUpdate>",
            lun_count
        )
    }

    #[test]
    fn device_list_placeholder_does_not_weaken_real_update_identity() {
        let root =
            parse_xml(b"<ESATMDevices><mESAID/><mESAID>SYNTHETIC-5D</mESAID></ESATMDevices>")
                .unwrap();
        assert_eq!(advertised_ids(&root).unwrap(), vec!["SYNTHETIC-5D"]);
        assert!(
            advertised_ids(&parse_xml(b"<ESATMDevices><mESAID/></ESATMDevices>").unwrap())
                .unwrap()
                .is_empty()
        );
        assert!(
            parse_update(&parse_xml(update_xml("", &[(0, 100, 30)]).as_bytes()).unwrap()).is_err()
        );
    }

    #[test]
    fn valid_two_luns_are_one_pool_and_ssd_capacity_is_not_cache_usage() {
        let xml = update_xml("E5D-77", &[(0, 800, 330), (1, 800, 120)]);
        let root = parse_xml(xml.as_bytes()).unwrap();
        assert_eq!(one_text(&root, "mESAID").unwrap(), "E5D-77");
        assert_eq!(one_text(&root, "mLUNCount").unwrap(), "2");
        assert!(parse_slots(&root).is_ok());
        assert!(parse_volumes(&root, 2).is_ok());
        let parsed = parse_update(&root).unwrap();
        let json = result_json(vec![parsed], 1);
        assert_eq!(json["pools"].as_array().unwrap().len(), 1);
        assert!(
            json["pools"][0]["id"]
                .as_str()
                .unwrap()
                .starts_with("opendrobo/device/")
        );
        assert_eq!(json["pools"][0]["brand"], "Drobo");
        assert_eq!(json["sourceTrust"], "unverified-loopback");
        assert!(json["pools"][0]["firmwareVersion"].is_null());
        assert!(json["pools"][0]["slots"][0]["manufacturerModel"].is_null());
        assert_eq!(json["pools"][0]["deviceStatus"]["mStatus"], "1");
        assert_eq!(json["pools"][0]["lunCount"], 2);
        assert_eq!(json["pools"][0]["slots"][0]["kind"], "ssd");
        assert_eq!(json["pools"][0]["slots"][0]["physicalCapacityBytes"], "500");
        assert!(json["pools"][0]["slots"][0]["cacheUsedBytes"].is_null());
        assert!(
            serde_json::to_string(&json)
                .unwrap()
                .find("E5D-77")
                .is_none()
        );
    }

    #[test]
    fn display_metadata_is_optional_bounded_and_never_serializes_serials() {
        let xml = update_xml("SYNTHETIC-ID", &[(0, 800, 450)])
            .replace("<mModel>", "<mVersion> 4.0.0-test </mVersion><mReleaseDate>Jan 01 2020</mReleaseDate><mSerial>PRIVATE-UNIT-SERIAL</mSerial><mModel>")
            .replace("<mSlotNumber>", "<mMake> EXAMPLE SSD MODEL </mMake><mDiskFwRev> FW01 </mDiskFwRev><mSerial>PRIVATE-DRIVE-SERIAL</mSerial><mSlotNumber>");
        let value = result_json(
            vec![parse_update(&parse_xml(xml.as_bytes()).unwrap()).unwrap()],
            1,
        );
        assert_eq!(value["pools"][0]["firmwareVersion"], "4.0.0-test");
        assert_eq!(value["pools"][0]["firmwareReleaseDate"], "Jan 01 2020");
        assert_eq!(
            value["pools"][0]["slots"][0]["manufacturerModel"],
            "EXAMPLE SSD MODEL"
        );
        assert_eq!(value["pools"][0]["slots"][0]["firmwareRevision"], "FW01");
        let serialized = value.to_string();
        for private in [
            "SYNTHETIC-ID",
            "PRIVATE-UNIT-SERIAL",
            "PRIVATE-DRIVE-SERIAL",
        ] {
            assert!(!serialized.contains(private));
        }
        for invalid in [
            xml.replace("<mVersion>", "<mVersion>duplicate</mVersion><mVersion>"),
            xml.replace("4.0.0-test", &"x".repeat(101)),
            xml.replace("EXAMPLE SSD MODEL", "EXAMPLE\tMODEL"),
            xml.replace("FW01", "<nested>FW01</nested>"),
        ] {
            assert!(parse_update(&parse_xml(invalid.as_bytes()).unwrap()).is_err());
        }
        let blank = xml.replace(" EXAMPLE SSD MODEL ", "   ");
        assert!(
            parse_update(&parse_xml(blank.as_bytes()).unwrap())
                .unwrap()
                .slots[0]
                .manufacturer_model
                .is_none()
        );
    }

    #[test]
    fn command_result_requires_exact_command_and_unique_leaf_values() {
        let valid = b"<TMCmd><CmdID>21</CmdID><Result>0</Result><ResultDetails>done</ResultDetails></TMCmd>";
        assert_eq!(command_result(valid, 21).unwrap(), (0, Some("done".into())));
        assert!(command_result(b"<TMCmd><CmdID>22</CmdID><Result>0</Result></TMCmd>", 21).is_err());
        assert!(
            command_result(
                b"<TMCmd><CmdID>21</CmdID><Result>0</Result><Result>1</Result></TMCmd>",
                21
            )
            .is_err()
        );
    }

    #[test]
    fn internal_control_target_requires_hashed_id_and_5d_model() {
        let parsed = parse_update(
            &parse_xml(update_xml("E5D-opaque", &[(0, 800, 330), (1, 800, 120)]).as_bytes())
                .unwrap(),
        )
        .unwrap();
        let id = public_id(&parsed.id);
        let target = control_target_from_updates(vec![parsed], &id).unwrap();
        assert_eq!(target.public_id, id);
        assert_eq!(target.firmware_features, 268_435_456);
        assert_eq!(target.connection_type, 1);
        assert_eq!(target.model, "Drobo 5D");
    }

    #[test]
    fn rejects_duplicate_critical_field_and_noncanonical_overflow_and_bad_count() {
        let duplicate = "<ESATMUpdate><mESAID>1</mESAID><mESAID>2</mESAID></ESATMUpdate>";
        assert!(parse_update(&parse_xml(duplicate.as_bytes()).unwrap()).is_err());
        assert!(canonical_u64("18446744073709551616").is_err());
        assert!(canonical_u64("01").is_err());
        assert!(
            parse_update(
                &parse_xml(
                    update_xml("E5D-1", &[(0, 1, 1)])
                        .replace("<mLUNCount>1</mLUNCount>", "<mLUNCount>2</mLUNCount>")
                        .as_bytes()
                )
                .unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_entities_doctype_and_depth() {
        assert!(parse_xml(b"<!DOCTYPE x><x/>").is_err());
        assert!(parse_xml(b"<x>&amp;</x>").is_err());
        let deep = format!("{}x{}", "<x>".repeat(17), "</x>".repeat(17));
        assert!(parse_xml(deep.as_bytes()).is_err());
    }

    #[test]
    fn synthetic_local_fixture_rejects_bad_frame() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut request = [0u8; 16];
            s.read_exact(&mut request).unwrap();
            assert_eq!(&request, b"DRIDDSVC\x01\x01\x00\x00\x00\x00\x00\x00");
            s.write_all(b"DRIDDSVC\x07\x01\x09\x00\x00\x00\x00\x01x")
                .unwrap();
        });
        assert!(status_from_addr(address).is_err());
        server.join().unwrap();
    }

    #[test]
    fn synthetic_local_fixture_returns_after_advertised_full_update() {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let xml = update_xml("E5D-7", &[(0, 100, 40), (1, 100, 30)]);
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 16];
            stream.read_exact(&mut request).unwrap();
            assert_eq!(&request, b"DRIDDSVC\x01\x01\x00\x00\x00\x00\x00\x00");
            for (kind, body) in [
                (
                    4,
                    b"<ESATMDevices><mESAID/><mESAID>E5D-7</mESAID></ESATMDevices>".as_slice(),
                ),
                (1, xml.as_bytes()),
            ] {
                let mut header = *b"DRIDDSVC\x07\x01\x00\x00\x00\x00\x00\x00";
                header[10] = kind;
                header[12..16].copy_from_slice(&(body.len() as u32).to_be_bytes());
                stream.write_all(&header).unwrap();
                stream.write_all(body).unwrap();
            }
        });
        let value = status_from_addr(address).unwrap();
        assert_eq!(value["state"], "available");
        assert_eq!(value["pools"][0]["volumes"].as_array().unwrap().len(), 2);
        server.join().unwrap();
    }
}
