//! Bounded, process-local interpretation of already-sanitized service snapshots.
//! This module never starts a poller or reads hardware.

#[cfg(not(test))]
use crate::diagnostics::{self, Level};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_EVENTS: usize = 500;
const MAX_UNITS: usize = 32;
const MAX_SLOTS: usize = 32;
const STALE_AFTER_MS: u64 = 45_000;

static STORE: OnceLock<Mutex<Store>> = OnceLock::new();

#[derive(Clone, Copy, Eq, PartialEq)]
enum State {
    Unknown,
    Active,
    Inactive,
    Stale,
    ReadError,
}

impl State {
    fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Stale => "stale",
            Self::ReadError => "read-error",
        }
    }
}

#[derive(Clone)]
struct Slot {
    state: State,
    physical_capacity: String,
    ssd_wear: Option<String>,
    raw_status: (String, String, String),
}

#[derive(Clone)]
struct Unit {
    state: State,
    last_observed_at_ms: u64,
    protected_used: String,
    protected_total: String,
    protected_free: String,
    raw_health_state: Option<String>,
    raw_device_status: (String, String, String),
    slots: BTreeMap<u32, Slot>,
}

#[derive(Clone)]
struct Event {
    id: String,
    timestamp_ms: u64,
    severity: &'static str,
    unit_id: Option<String>,
    message: &'static str,
    raw_health_state: Option<String>,
    kind: &'static str,
    slot: Option<u32>,
}

struct Store {
    state: State,
    last_read_state: Option<State>,
    observed_since_ms: Option<u64>,
    sequence: u64,
    units: BTreeMap<String, Unit>,
    events: VecDeque<Event>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            state: State::Unknown,
            last_read_state: None,
            observed_since_ms: None,
            sequence: 0,
            units: BTreeMap::new(),
            events: VecDeque::new(),
        }
    }
}

fn store() -> &'static Mutex<Store> {
    STORE.get_or_init(|| Mutex::new(Store::default()))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn canonical_decimal(value: &str) -> Option<String> {
    (!value.is_empty()
        && value.len() <= 20
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value.len() == 1 || !value.starts_with('0')))
    .then(|| value.to_owned())
}

fn event(
    store: &mut Store,
    severity: &'static str,
    unit_id: Option<&str>,
    message: &'static str,
    raw_health_state: Option<&str>,
    kind: &'static str,
    slot: Option<u32>,
) {
    let timestamp_ms = now_ms();
    let entry = Event {
        id: format!("device-event-{}", store.sequence),
        timestamp_ms,
        severity,
        unit_id: unit_id.map(str::to_owned),
        message,
        raw_health_state: raw_health_state.map(str::to_owned),
        kind,
        slot,
    };
    store.sequence = store.sequence.saturating_add(1);
    store.events.push_back(entry);
    if store.events.len() > MAX_EVENTS {
        store.events.pop_front();
    }
    #[cfg(not(test))]
    diagnostics::record(
        match severity {
            "warning" => Level::Warning,
            "error" => Level::Error,
            _ => Level::Info,
        },
        "device-events",
        kind,
        message,
    );
}

fn value_string(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object.get(key)?.as_str().and_then(canonical_decimal)
}

fn observed_unit(pool: &Value, observed_at_ms: u64) -> Option<(String, Unit)> {
    let pool = pool.as_object()?;
    let id = pool.get("id")?.as_str()?;
    if !id.starts_with("opendrobo/device/")
        || id.len() > 96
        || !id.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'/' || byte == b'-' || byte == b'_'
        })
    {
        return None;
    }
    let protected_used = value_string(pool, "protectedUsedBytes")?;
    let protected_total = value_string(pool, "protectedTotalBytes")?;
    let protected_free = value_string(pool, "protectedFreeBytes")?;
    let status = pool.get("deviceStatus")?.as_object()?;
    let device_status = (
        value_string(status, "mStatus")?,
        value_string(status, "mStatusEx")?,
        value_string(status, "mDiskPackStatus")?,
    );
    // These strings remain raw numeric service values. No status-bit meaning is inferred.
    let raw_health_state = Some(device_status.0.clone());
    let mut slots = BTreeMap::new();
    for raw_slot in pool.get("slots")?.as_array()?.iter().take(MAX_SLOTS + 1) {
        let slot = raw_slot.as_object()?;
        let index = u32::try_from(slot.get("slot")?.as_u64()?).ok()?;
        if index > 31 || slots.contains_key(&index) {
            return None;
        }
        let physical_capacity = value_string(slot, "physicalCapacityBytes")?;
        let raw_status = slot.get("rawStatus")?.as_object()?;
        let slot_status = (
            value_string(raw_status, "mStatus")?,
            value_string(raw_status, "mDiskState")?,
            value_string(raw_status, "mDiskType")?,
        );
        let ssd_wear = slot
            .get("ssdLifeRemainingPercent")
            .and_then(Value::as_u64)
            .map(|value| value.to_string());
        slots.insert(
            index,
            Slot {
                // A zero capacity record does not prove an empty bay; keep it unknown.
                state: if physical_capacity == "0" {
                    State::Unknown
                } else {
                    State::Active
                },
                physical_capacity,
                ssd_wear,
                raw_status: slot_status,
            },
        );
    }
    if slots.len() > MAX_SLOTS {
        return None;
    }
    Some((
        id.to_owned(),
        Unit {
            state: State::Active,
            last_observed_at_ms: observed_at_ms,
            protected_used,
            protected_total,
            protected_free,
            raw_health_state,
            raw_device_status: device_status,
            slots,
        },
    ))
}

/// Observe a successful, already-sanitized service snapshot. Invalid shapes are ignored.
pub fn observe(snapshot: Value) {
    let Some(root) = snapshot.as_object() else {
        return;
    };
    if root.get("state").and_then(Value::as_str) != Some("available") {
        return;
    }
    let Some(observed_at_ms) = root.get("observedAtMs").and_then(Value::as_u64) else {
        return;
    };
    let Some(pools) = root.get("pools").and_then(Value::as_array) else {
        return;
    };
    if pools.len() > MAX_UNITS {
        return;
    }
    let mut current = BTreeMap::new();
    for pool in pools {
        let Some((id, unit)) = observed_unit(pool, observed_at_ms) else {
            return;
        };
        if current.insert(id, unit).is_some() {
            return;
        }
    }
    let mut store = store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if store
        .units
        .values()
        .map(|unit| unit.last_observed_at_ms)
        .max()
        .is_some_and(|latest| observed_at_ms < latest)
    {
        return;
    }
    if store.observed_since_ms.is_none() {
        store.observed_since_ms = Some(observed_at_ms);
    }
    let old = std::mem::take(&mut store.units);
    let mut next = BTreeMap::new();
    for (id, mut unit) in current {
        match old.get(&id) {
            None => event(
                &mut store,
                "info",
                Some(&id),
                "Unit first observed.",
                unit.raw_health_state.as_deref(),
                "first-observed",
                None,
            ),
            Some(previous) if previous.state == State::Inactive => event(
                &mut store,
                "info",
                Some(&id),
                "Unit reconnected.",
                unit.raw_health_state.as_deref(),
                "reconnected",
                None,
            ),
            Some(_) => {}
        }
        if let Some(previous) = old.get(&id) {
            if previous.protected_used != unit.protected_used
                || previous.protected_total != unit.protected_total
                || previous.protected_free != unit.protected_free
            {
                event(
                    &mut store,
                    "info",
                    Some(&id),
                    "Protected capacity changed.",
                    unit.raw_health_state.as_deref(),
                    "protected-capacity-changed",
                    None,
                );
            }
            if previous.raw_device_status != unit.raw_device_status {
                event(
                    &mut store,
                    "info",
                    Some(&id),
                    "Numeric device status changed.",
                    unit.raw_health_state.as_deref(),
                    "numeric-status-changed",
                    None,
                );
            }
            for (slot, current_slot) in &unit.slots {
                let previous_slot = previous.slots.get(slot);
                if current_slot.physical_capacity != "0"
                    && previous_slot.is_none_or(|previous| previous.physical_capacity == "0")
                {
                    event(
                        &mut store,
                        "info",
                        Some(&id),
                        "Positive drive capacity observed.",
                        unit.raw_health_state.as_deref(),
                        "positive-drive-capacity-observed",
                        Some(*slot),
                    );
                }
                if previous_slot
                    .is_some_and(|previous| previous.raw_status != current_slot.raw_status)
                {
                    event(
                        &mut store,
                        "info",
                        Some(&id),
                        "Numeric slot status changed.",
                        unit.raw_health_state.as_deref(),
                        "numeric-slot-status-changed",
                        Some(*slot),
                    );
                }
                if current_slot.state == State::Active
                    && previous_slot.and_then(|previous| previous.ssd_wear.as_ref())
                        != current_slot.ssd_wear.as_ref()
                    && previous_slot.is_some()
                {
                    event(
                        &mut store,
                        "info",
                        Some(&id),
                        "SSD wear changed.",
                        unit.raw_health_state.as_deref(),
                        "ssd-wear-changed",
                        Some(*slot),
                    );
                }
            }
        }
        // A missing slot is not a removal: this protocol has no proven empty-bay enum.
        unit.state = State::Active;
        next.insert(id, unit);
    }
    let mut inactive: Vec<(String, Unit)> = old
        .into_iter()
        .filter(|(id, _)| !next.contains_key(id))
        .collect();
    inactive.sort_by(|left, right| right.1.last_observed_at_ms.cmp(&left.1.last_observed_at_ms));
    for (id, previous) in inactive
        .into_iter()
        .take(MAX_UNITS.saturating_sub(next.len()))
    {
        let was_inactive = previous.state == State::Inactive;
        let mut retained = previous;
        retained.state = State::Inactive;
        for slot in retained.slots.values_mut() {
            slot.state = State::Inactive;
        }
        if !was_inactive {
            event(
                &mut store,
                "info",
                Some(&id),
                "Unit removed.",
                retained.raw_health_state.as_deref(),
                "removed",
                None,
            );
        }
        next.insert(id, retained);
    }
    store.units = next;
    store.state = if store.units.values().any(|unit| unit.state == State::Active) {
        State::Active
    } else {
        State::Inactive
    };
    store.last_read_state = None;
}

/// Record a definitive service report that no unit is advertised.
pub fn unavailable() {
    let mut store = store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let ids: Vec<String> = store.units.keys().cloned().collect();
    for id in ids {
        let (was_active, raw_health_state) = match store.units.get(&id) {
            Some(unit) => (unit.state != State::Inactive, unit.raw_health_state.clone()),
            None => continue,
        };
        if was_active {
            event(
                &mut store,
                "info",
                Some(&id),
                "Unit removed.",
                raw_health_state.as_deref(),
                "removed",
                None,
            );
        }
        if let Some(unit) = store.units.get_mut(&id) {
            unit.state = State::Inactive;
            for slot in unit.slots.values_mut() {
                slot.state = State::Inactive;
            }
        }
    }
    store.state = State::Inactive;
    store.last_read_state = None;
}

/// Record a failed read without inferring a disconnect or discarding known data.
pub fn read_error() {
    let mut store = store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if store.last_read_state == Some(State::ReadError) {
        return;
    }
    let has_known = !store.units.is_empty();
    store.state = if has_known {
        State::Stale
    } else {
        State::ReadError
    };
    store.last_read_state = Some(State::ReadError);
    if has_known {
        for unit in store.units.values_mut() {
            unit.state = State::Stale;
            for slot in unit.slots.values_mut() {
                if slot.state != State::Unknown {
                    slot.state = State::Stale;
                }
            }
        }
        event(
            &mut store,
            "error",
            None,
            "Service read failed; known data retained as stale.",
            None,
            "read-error",
            None,
        );
    } else {
        event(
            &mut store,
            "error",
            None,
            "Service read failed; no unit state is known.",
            None,
            "read-error",
            None,
        );
    }
}

fn age_observations(store: &mut Store, current_ms: u64) {
    for unit in store.units.values_mut() {
        if unit.state != State::Inactive
            && current_ms.saturating_sub(unit.last_observed_at_ms) > STALE_AFTER_MS
        {
            unit.state = State::Stale;
            for slot in unit.slots.values_mut() {
                if slot.state == State::Active {
                    slot.state = State::Stale;
                }
            }
        }
    }
    if store.units.values().any(|unit| unit.state == State::Active) {
        store.state = State::Active;
    } else if store.units.values().any(|unit| unit.state == State::Stale) {
        store.state = State::Stale;
    } else if !store.units.is_empty() {
        store.state = State::Inactive;
    }
}

/// Return bounded device events plus current observed state. No service data is read here.
pub fn snapshot(limit: usize) -> Value {
    snapshot_at(limit, now_ms())
}

fn snapshot_at(limit: usize, current_ms: u64) -> Value {
    let limit = limit.clamp(1, MAX_EVENTS);
    let mut store = store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    age_observations(&mut store, current_ms);
    let events = store
        .events
        .iter()
        .skip(store.events.len().saturating_sub(limit))
        .map(|event| {
            json!({
                "id": event.id,
                "timestampMs": event.timestamp_ms,
                "severity": event.severity,
                "source": "original-service",
                "unitId": event.unit_id,
                "message": event.message,
                "rawHealthState": event.raw_health_state,
                "kind": event.kind,
                "slot": event.slot,
            })
        })
        .collect::<Vec<_>>();
    let units = store.units.iter().map(|(id, unit)| json!({
        "id": id,
        "state": unit.state.name(),
        "lastObservedAtMs": unit.last_observed_at_ms,
        "slots": unit.slots.iter().map(|(slot, state)| json!({"slot":slot,"state":state.state.name()})).collect::<Vec<_>>(),
    })).collect::<Vec<_>>();
    json!({
        "events": events,
        "observedSinceMs": store.observed_since_ms,
        "historyScope": "observed-since-observation",
        "message": "Bounded in-memory events from sanitized original-service snapshots; no hardware health meaning is inferred.",
        "state": store.state.name(),
        "lastReadState": store.last_read_state.map(State::name),
        "lastObservedAtMs": store.units.values().map(|unit| unit.last_observed_at_ms).max(),
        "units": units,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn reset() {
        *store()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Store::default();
    }

    fn service(pool_id: &str, used: &str, status: &str, capacity: &str, wear: u64) -> Value {
        json!({"state":"available","observedAtMs":10,"pools":[{
            "id":pool_id,"protectedTotalBytes":"100","protectedUsedBytes":used,"protectedFreeBytes":"90",
            "deviceStatus":{"mStatus":status,"mStatusEx":"0","mDiskPackStatus":"0"},
            "slots":[{"slot":5,"physicalCapacityBytes":capacity,"ssdLifeRemainingPercent":wear,
                "rawStatus":{"mStatus":"1","mDiskState":"2","mDiskType":"4"}}]
        }]})
    }

    #[test]
    fn synthetic_events_are_bounded_and_never_include_raw_esa_id() {
        let _guard = TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset();
        observe(service("opendrobo/device/abc", "10", "1", "100", 90));
        observe(service("opendrobo/device/abc", "11", "2", "100", 89));
        let value = snapshot_at(500, 10);
        assert!(value["events"].as_array().unwrap().len() >= 4);
        assert!(
            serde_json::to_string(&value)
                .unwrap()
                .find("ESA-")
                .is_none()
        );
        assert_eq!(value["units"][0]["state"], "active");
    }

    #[test]
    fn zero_slot_is_unknown_and_read_error_keeps_unit_without_false_removal() {
        let _guard = TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset();
        observe(service("opendrobo/device/zero", "10", "1", "0", 90));
        read_error();
        read_error();
        let value = snapshot_at(500, 10);
        assert_eq!(value["state"], "stale");
        assert_eq!(value["units"][0]["slots"][0]["state"], "unknown");
        assert_eq!(
            value["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|event| event["kind"] == "removed")
                .count(),
            0
        );
        assert_eq!(
            value["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|event| event["kind"] == "read-error")
                .count(),
            1
        );
    }

    #[test]
    fn inactive_history_is_bounded_and_fresh_data_does_not_reinsert_stale_drive() {
        let _guard = TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset();
        observe(service("opendrobo/device/old", "10", "1", "100", 90));
        read_error();
        let mut newer = service("opendrobo/device/new", "10", "1", "100", 90);
        newer["observedAtMs"] = json!(11);
        observe(newer);
        let value = snapshot_at(500, 11);
        assert_eq!(value["units"].as_array().unwrap().len(), 2);
        assert_eq!(value["units"][0]["state"], "active");
        assert_eq!(value["units"][1]["state"], "inactive");

        reset();
        observe(service("opendrobo/device/recover", "10", "1", "100", 90));
        read_error();
        let mut recovered = service("opendrobo/device/recover", "10", "1", "100", 90);
        recovered["observedAtMs"] = json!(11);
        observe(recovered);
        let recovered_events = snapshot_at(500, 11)["events"].as_array().unwrap().clone();
        assert_eq!(
            recovered_events
                .iter()
                .filter(|event| event["kind"] == "positive-drive-capacity-observed")
                .count(),
            0
        );
    }

    #[test]
    fn full_raw_status_and_all_capacity_values_drive_events_without_regression() {
        let _guard = TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset();
        observe(service("opendrobo/device/status", "10", "1", "100", 90));
        let mut changed = service("opendrobo/device/status", "10", "1", "100", 90);
        changed["observedAtMs"] = json!(11);
        changed["pools"][0]["protectedTotalBytes"] = json!("101");
        changed["pools"][0]["protectedFreeBytes"] = json!("91");
        changed["pools"][0]["deviceStatus"]["mStatusEx"] = json!("8");
        changed["pools"][0]["slots"][0]["rawStatus"]["mDiskState"] = json!("9");
        observe(changed);
        let events = snapshot_at(500, 11)["events"].as_array().unwrap().clone();
        assert!(
            events
                .iter()
                .any(|event| event["kind"] == "protected-capacity-changed")
        );
        assert!(
            events
                .iter()
                .any(|event| event["kind"] == "numeric-status-changed")
        );
        assert!(
            events
                .iter()
                .any(|event| event["kind"] == "numeric-slot-status-changed")
        );

        let mut old = service("opendrobo/device/status", "1", "1", "100", 90);
        old["observedAtMs"] = json!(10);
        observe(old);
        assert_eq!(snapshot_at(500, 11)["units"][0]["lastObservedAtMs"], 11);
    }

    #[test]
    fn stale_age_and_memory_limit_are_deterministic() {
        let _guard = TEST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        reset();
        observe(service("opendrobo/device/age", "10", "1", "100", 90));
        assert_eq!(snapshot_at(500, 45_011)["state"], "stale");
        let mut state = store()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for _ in 0..(MAX_EVENTS + 1) {
            event(
                &mut state,
                "info",
                None,
                "Bounded synthetic event.",
                None,
                "synthetic",
                None,
            );
        }
        assert_eq!(state.events.len(), MAX_EVENTS);
    }
}
