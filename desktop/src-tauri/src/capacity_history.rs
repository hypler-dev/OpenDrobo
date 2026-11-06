//! Sanitized, bounded capacity observations. This is trend data, not disk geometry.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_SAMPLES: usize = 2_048;
const MAX_FILE: u64 = 1_048_576;
const BUCKET_MS: u64 = 300_000;
const FUTURE_GRACE_MS: u64 = 300_000;

static MEMORY: OnceLock<Mutex<Memory>> = OnceLock::new();

#[derive(Clone)]
struct Sample {
    timestamp_ms: u64,
    total_bytes: String,
    used_bytes: String,
    free_bytes: String,
    os_used_bytes: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredRecord {
    id: String,
    model: String,
    timestamp_ms: u64,
    total_bytes: String,
    used_bytes: String,
    free_bytes: String,
    os_used_bytes: String,
}

impl StoredRecord {
    fn sample(&self) -> Sample {
        Sample {
            timestamp_ms: self.timestamp_ms,
            total_bytes: self.total_bytes.clone(),
            used_bytes: self.used_bytes.clone(),
            free_bytes: self.free_bytes.clone(),
            os_used_bytes: self.os_used_bytes.clone(),
        }
    }
}

#[derive(Clone, Default)]
struct Pool {
    model: String,
    samples: VecDeque<Sample>,
}

#[derive(Clone, Default)]
struct History {
    pools: BTreeMap<String, Pool>,
}

#[derive(Default)]
struct Memory {
    history: History,
    // A failed truncate/write/flush leaves RAM authoritative until a full write succeeds.
    dirty: bool,
}

enum PersistenceOutcome {
    Written(History),
    Failed(History),
}

fn memory() -> &'static Mutex<Memory> {
    MEMORY.get_or_init(|| Mutex::new(Memory::default()))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn canonical_u64(value: &str) -> Option<u64> {
    if value.is_empty()
        || value.len() > 20
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    value.parse().ok()
}

fn valid_id(id: &str) -> bool {
    let Some(hash) = id.strip_prefix("opendrobo/device/") else {
        return false;
    };
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_model(model: &str) -> bool {
    !model.is_empty()
        && model.len() <= 100
        && model
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
}

fn valid_sample(sample: &Sample, current_ms: u64) -> bool {
    let (Some(total), Some(used), Some(free), Some(_os_used)) = (
        canonical_u64(&sample.total_bytes),
        canonical_u64(&sample.used_bytes),
        canonical_u64(&sample.free_bytes),
        canonical_u64(&sample.os_used_bytes),
    ) else {
        return false;
    };
    sample.timestamp_ms != 0
        && sample.timestamp_ms <= current_ms.saturating_add(FUTURE_GRACE_MS)
        && total > 0
        && used.checked_add(free) == Some(total)
}

fn record_from_value(pool: &Value, observed_at_ms: u64, current_ms: u64) -> Option<StoredRecord> {
    let pool = pool.as_object()?;
    let string = |key: &str| pool.get(key)?.as_str().map(str::to_owned);
    let record = StoredRecord {
        id: string("id")?,
        model: string("model")?,
        timestamp_ms: observed_at_ms,
        total_bytes: string("protectedTotalBytes")?,
        used_bytes: string("protectedUsedBytes")?,
        free_bytes: string("protectedFreeBytes")?,
        os_used_bytes: string("osUsedBytes")?,
    };
    (valid_id(&record.id)
        && valid_model(&record.model)
        && valid_sample(&record.sample(), current_ms))
    .then_some(record)
}

fn sample_count(history: &History) -> usize {
    history.pools.values().map(|pool| pool.samples.len()).sum()
}

fn drop_oldest(history: &mut History) {
    let oldest = history
        .pools
        .iter()
        .filter_map(|(id, pool)| {
            pool.samples
                .front()
                .map(|sample| (id.clone(), sample.timestamp_ms))
        })
        .min_by_key(|(_, timestamp)| *timestamp)
        .map(|(id, _)| id);
    if let Some(id) = oldest {
        if let Some(pool) = history.pools.get_mut(&id) {
            pool.samples.pop_front();
            if pool.samples.is_empty() {
                history.pools.remove(&id);
            }
        }
    }
}

fn merge(history: &mut History, record: StoredRecord) {
    let sample = record.sample();
    let pool = history.pools.entry(record.id).or_default();
    if pool.samples.is_empty() {
        pool.model = record.model;
        pool.samples.push_back(sample);
    } else {
        let last = pool.samples.back().expect("nonempty sample history");
        if sample.timestamp_ms > last.timestamp_ms {
            pool.model = record.model;
            if last.timestamp_ms / BUCKET_MS == sample.timestamp_ms / BUCKET_MS {
                pool.samples.pop_back();
            }
            pool.samples.push_back(sample);
        }
    }
    while sample_count(history) > MAX_SAMPLES {
        drop_oldest(history);
    }
}

fn parse_lines(bytes: &[u8], current_ms: u64) -> History {
    let mut history = History::default();
    for line in bytes.split(|byte| *byte == b'\n') {
        if line.is_empty() || line.len() > 1024 {
            continue;
        }
        let Ok(record) = serde_json::from_slice::<StoredRecord>(line) else {
            continue;
        };
        if valid_id(&record.id)
            && valid_model(&record.model)
            && valid_sample(&record.sample(), current_ms)
        {
            merge(&mut history, record);
        }
    }
    history
}

fn records(history: &History) -> Vec<StoredRecord> {
    history
        .pools
        .iter()
        .flat_map(|(id, pool)| {
            pool.samples.iter().cloned().map(|sample| StoredRecord {
                id: id.clone(),
                model: pool.model.clone(),
                timestamp_ms: sample.timestamp_ms,
                total_bytes: sample.total_bytes,
                used_bytes: sample.used_bytes,
                free_bytes: sample.free_bytes,
                os_used_bytes: sample.os_used_bytes,
            })
        })
        .collect()
}

fn merge_histories(ram: &History, disk: &History) -> History {
    let mut combined = records(ram);
    combined.extend(records(disk));
    // Each source is capped at 2048; keep this bound explicit before rebuilding.
    combined.sort_by(|left, right| {
        left.timestamp_ms
            .cmp(&right.timestamp_ms)
            .then_with(|| left.id.cmp(&right.id))
    });
    if combined.len() > MAX_SAMPLES * 2 {
        combined.truncate(MAX_SAMPLES * 2);
    }
    let mut merged = History::default();
    for record in combined {
        merge(&mut merged, record);
    }
    merged
}

fn serialized(history: &History) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    for record in records(history) {
        let mut line = serde_json::to_vec(&record).ok()?;
        line.push(b'\n');
        if bytes.len().checked_add(line.len())? > MAX_FILE as usize {
            return None;
        }
        bytes.extend(line);
    }
    Some(bytes)
}

fn read_file(file: &mut std::fs::File, current_ms: u64) -> std::io::Result<History> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE {
        return Err(std::io::Error::other("capacity history exceeds bound"));
    }
    Ok(parse_lines(&bytes, current_ms))
}

fn bounded_bytes(history: &mut History) -> Vec<u8> {
    let bytes = loop {
        if let Some(bytes) = serialized(&history) {
            break bytes;
        }
        if sample_count(&history) == 0 {
            break Vec::new();
        }
        drop_oldest(history);
    };
    bytes
}

fn persist_merged<F>(mut history: History, write: F) -> PersistenceOutcome
where
    F: FnOnce(&[u8]) -> std::io::Result<()>,
{
    let bytes = bounded_bytes(&mut history);
    if write(&bytes).is_ok() {
        PersistenceOutcome::Written(history)
    } else {
        PersistenceOutcome::Failed(history)
    }
}

fn write_file(desired: &History, current_ms: u64) -> PersistenceOutcome {
    let mut file = match crate::diagnostics::capacity_history_file(true) {
        Ok(file) => file,
        Err(_) => return PersistenceOutcome::Failed(desired.clone()),
    };
    if crate::diagnostics::lock_private(&file, true).is_err() {
        return PersistenceOutcome::Failed(desired.clone());
    }
    // Read, merge, and rewrite while holding one exclusive private-file lock.
    let mut history = match read_file(&mut file, current_ms) {
        Ok(history) => history,
        Err(_) => return PersistenceOutcome::Failed(desired.clone()),
    };
    for record in records(desired) {
        merge(&mut history, record);
    }
    // The helper returns only the fixed private file. A crash may lose trends, never device data.
    persist_merged(history, |bytes| {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(bytes)?;
        file.flush()
    })
}

fn disk_history(current_ms: u64) -> std::io::Result<History> {
    let mut file = crate::diagnostics::capacity_history_file(false)?;
    crate::diagnostics::lock_private(&file, false)?;
    read_file(&mut file, current_ms)
}

/// Record only a fully validated protected-pool observation. Unknown fields are ignored.
pub fn observe(snapshot: &Value) {
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
    if pools.len() > 32 {
        return;
    }
    let current_ms = now_ms();
    let mut incoming = Vec::new();
    for pool in pools {
        let Some(record) = record_from_value(pool, observed_at_ms, current_ms) else {
            return;
        };
        if incoming
            .iter()
            .any(|existing: &StoredRecord| existing.id == record.id)
        {
            return;
        }
        incoming.push(record);
    }
    let mut memory = memory()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for record in &incoming {
        merge(&mut memory.history, record.clone());
    }
    let desired = memory.history.clone();
    match write_file(&desired, current_ms) {
        PersistenceOutcome::Written(history) => {
            memory.history = history;
            memory.dirty = false;
        }
        PersistenceOutcome::Failed(history) => {
            memory.history = history;
            memory.dirty = true;
        }
    }
}

fn output(
    history: &History,
    unit_id: Option<&str>,
    limit: usize,
    persistence: &'static str,
) -> Value {
    let pools = history
        .pools
        .iter()
        .filter(|(id, _)| unit_id.is_none_or(|wanted| wanted == id.as_str()))
        .map(|(id, pool)| {
            let samples = pool
                .samples
                .iter()
                .skip(pool.samples.len().saturating_sub(limit))
                .map(|sample| {
                    json!({
                        "timestampMs": sample.timestamp_ms,
                        "totalBytes": sample.total_bytes,
                        "usedBytes": sample.used_bytes,
                        "freeBytes": sample.free_bytes,
                        "osUsedBytes": sample.os_used_bytes,
                    })
                })
                .collect::<Vec<_>>();
            json!({"id":id,"model":pool.model,"samples":samples})
        })
        .collect::<Vec<_>>();
    json!({
        "state": if pools.is_empty() { "unavailable" } else { "available" },
        "source": "opendrobo-observations",
        "persistence": persistence,
        "pools": pools,
        "message": "Sanitized protected-pool observations recorded by OpenDrobo. Historical data is not inferred and does not describe Finder, disk geometry, or bay history.",
    })
}

/// Return latest bounded observations from the shared private file, or process memory on failure.
pub fn snapshot(unit_id: Option<&str>, limit: usize) -> Result<Value, String> {
    if !(1..=MAX_SAMPLES).contains(&limit) {
        return Err("Capacity history limit must be between 1 and 2048.".into());
    }
    if let Some(id) = unit_id
        && !valid_id(id)
    {
        return Err("Capacity history unit identifier is invalid.".into());
    }
    let current_ms = now_ms();
    let memory = memory()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match disk_history(current_ms) {
        Ok(history) if memory.dirty => {
            let merged = merge_histories(&memory.history, &history);
            Ok(output(&merged, unit_id, limit, "memory"))
        }
        Ok(history) => Ok(output(&history, unit_id, limit, "disk")),
        Err(_) => Ok(output(&memory.history, unit_id, limit, "memory")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn reset() {
        *memory()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Memory::default();
    }
    fn id() -> &'static str {
        "opendrobo/device/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    }
    fn record(timestamp_ms: u64, used: &str) -> StoredRecord {
        record_for(id().into(), "Drobo 5D".into(), timestamp_ms, used)
    }
    fn record_for(id: String, model: String, timestamp_ms: u64, used: &str) -> StoredRecord {
        StoredRecord {
            id,
            model,
            timestamp_ms,
            total_bytes: "100".into(),
            used_bytes: used.into(),
            free_bytes: (100 - used.parse::<u64>().unwrap()).to_string(),
            os_used_bytes: used.into(),
        }
    }

    #[test]
    fn two_luns_are_one_pool_and_same_bucket_coalesces() {
        let _guard = TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset();
        let mut order = History::default();
        merge(&mut order, record(BUCKET_MS * 2, "40"));
        merge(&mut order, record(BUCKET_MS, "41"));
        assert_eq!(
            order
                .pools
                .get(id())
                .unwrap()
                .samples
                .back()
                .unwrap()
                .used_bytes,
            "40"
        );
        let mut history = History::default();
        merge(&mut history, record(10, "40"));
        merge(&mut history, record(20, "41"));
        assert_eq!(sample_count(&history), 1);
        let service = json!({"state":"available","observedAtMs":300_010,"pools":[{
            "id":id(),"model":"Drobo 5D","protectedTotalBytes":"100","protectedUsedBytes":"40","protectedFreeBytes":"60","osUsedBytes":"45",
            "volumes":[{"logicalTotalBytes":"1000"},{"logicalTotalBytes":"1000"}]
        }]});
        let entry = record_from_value(&service["pools"][0], 300_010, 300_010).unwrap();
        merge(&mut history, entry);
        assert_eq!(history.pools.len(), 1);
    }

    #[test]
    fn validation_privacy_and_invalid_rows_are_bounded() {
        let _guard = TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset();
        let valid = serde_json::to_string(&record(10, "40")).unwrap();
        let rows = format!(
            "{{bad}}\n{valid}\n{{\"id\":\"{}\",\"model\":\"Drobo 5D\",\"timestampMs\":1,\"totalBytes\":\"100\",\"usedBytes\":\"99\",\"freeBytes\":\"2\",\"osUsedBytes\":\"1\"}}\n",
            id()
        );
        let history = parse_lines(rows.as_bytes(), 1_000);
        assert_eq!(sample_count(&history), 1);
        assert!(
            serde_json::to_string(&output(&history, None, 10, "memory"))
                .unwrap()
                .find("mESAID")
                .is_none()
        );
        assert!(canonical_u64("18446744073709551616").is_none());
        assert!(canonical_u64("01").is_none());
    }

    #[test]
    fn global_memory_ring_and_out_of_order_are_bounded() {
        let _guard = TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset();
        let mut history = History::default();
        for index in 0..(MAX_SAMPLES + 10) {
            merge(
                &mut history,
                record_for(
                    format!("opendrobo/device/{index:064x}"),
                    "Drobo 5D".into(),
                    (index as u64 + 1) * BUCKET_MS,
                    "40",
                ),
            );
        }
        assert_eq!(sample_count(&history), MAX_SAMPLES);
        assert_eq!(history.pools.len(), MAX_SAMPLES);
    }

    #[test]
    fn flat_jsonl_roundtrips_multiple_models_and_buckets() {
        let _guard = TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset();
        let mut history = History::default();
        merge(&mut history, record(10, "40"));
        merge(&mut history, record(BUCKET_MS + 10, "41"));
        merge(
            &mut history,
            record_for(
                "opendrobo/device/ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                    .into(),
                "Drobo 5C".into(),
                BUCKET_MS + 20,
                "42",
            ),
        );
        let bytes = serialized(&history).expect("bounded JSONL");
        assert!(!bytes.is_empty());
        let parsed = parse_lines(&bytes, BUCKET_MS + 30);
        assert_eq!(sample_count(&parsed), 3);
        assert_eq!(parsed.pools.len(), 2);
        assert_eq!(parsed.pools.get(id()).unwrap().samples.len(), 2);
    }

    #[test]
    fn failed_write_retains_merged_ram_history_and_marks_memory_authoritative() {
        let _guard = TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset();
        let mut prior_disk = History::default();
        merge(&mut prior_disk, record(10, "40"));
        let mut desired = prior_disk.clone();
        merge(&mut desired, record(BUCKET_MS + 10, "41"));
        let outcome = persist_merged(desired, |_| {
            Err(std::io::Error::other("injected flush failure"))
        });
        let PersistenceOutcome::Failed(history) = outcome else {
            panic!("injected write must fail");
        };
        assert_eq!(sample_count(&history), 2);
        let mut ram = memory()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        ram.history = history;
        ram.dirty = true;
        assert_eq!(
            output(&ram.history, None, 10, "memory")["persistence"],
            "memory"
        );
        assert_eq!(ram.history.pools.get(id()).unwrap().samples.len(), 2);
    }

    #[test]
    fn dirty_snapshot_merge_preserves_ram_and_partial_disk_history() {
        let _guard = TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset();
        let mut disk = History::default();
        merge(&mut disk, record(BUCKET_MS, "40"));
        merge(&mut disk, record(BUCKET_MS * 2, "41"));
        let mut ram = disk.clone();
        merge(&mut ram, record(BUCKET_MS * 3, "42"));
        let merged = merge_histories(&ram, &disk);
        assert_eq!(merged.pools.get(id()).unwrap().samples.len(), 3);

        let mut partial_disk = History::default();
        merge(&mut partial_disk, record(BUCKET_MS * 3, "42"));
        let mut older_ram = History::default();
        merge(&mut older_ram, record(BUCKET_MS, "40"));
        merge(&mut older_ram, record(BUCKET_MS * 2, "41"));
        let merged = merge_histories(&older_ram, &partial_disk);
        assert_eq!(merged.pools.get(id()).unwrap().samples.len(), 3);
    }
}
