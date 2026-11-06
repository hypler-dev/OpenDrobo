//! Bounded application events. Callers use fixed summaries, never payloads,
//! credentials, device serials, filesystem contents, or raw service messages.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
use std::fs::OpenOptions;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_FILE: u64 = 1_048_576;
const MAX_MEMORY: usize = 500;
static EVENTS: OnceLock<Mutex<VecDeque<Event>>> = OnceLock::new();
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Event {
    id: String,
    timestamp_ms: u64,
    level: Level,
    source: String,
    operation: String,
    message: String,
}

#[derive(Serialize)]
pub struct Snapshot {
    records: Vec<Event>,
    persistence: &'static str,
    message: &'static str,
}

fn memory() -> &'static Mutex<VecDeque<Event>> {
    EVENTS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn clean(text: &str, limit: usize) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(limit)
        .collect()
}

fn log_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let directory = PathBuf::from(std::env::var_os("HOME")?).join("Library/Logs/OpenDrobo");
    #[cfg(target_os = "linux")]
    let directory = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?
        .join("opendrobo");
    #[cfg(target_os = "windows")]
    let directory = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("OpenDrobo/Logs");
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return None;
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    directory
        .is_absolute()
        .then(|| directory.join("events.jsonl"))
}

/// Open only the fixed capacity-history file beside the private application log.
pub(crate) fn capacity_history_file(write: bool) -> std::io::Result<File> {
    let path = log_path()
        .ok_or_else(|| std::io::Error::other("application state directory unavailable"))?
        .with_file_name("capacity-history.jsonl");
    open_file(&path, write)
}

pub(crate) fn lock_private(file: &File, exclusive: bool) -> std::io::Result<()> {
    lock(file, exclusive)
}

#[cfg(unix)]
fn check_file(file: &File) -> std::io::Result<()> {
    use std::os::unix::fs::MetadataExt;
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.mode() & 0o077 != 0
        || metadata.uid() != unsafe { geteuid() }
    {
        return Err(std::io::Error::other(
            "console file is not private regular storage",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_file(file: &File) -> std::io::Result<()> {
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::other("console file is not regular storage"));
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn open_file(path: &Path, write: bool) -> std::io::Result<File> {
    use std::ffi::{CString, c_char};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    use std::path::Component;
    unsafe extern "C" {
        fn openat(directory: i32, name: *const c_char, flags: i32, ...) -> i32;
        fn mkdirat(directory: i32, name: *const c_char, mode: u32) -> i32;
        fn geteuid() -> u32;
    }
    #[cfg(target_os = "macos")]
    const DIRECTORY_FLAGS: i32 = 0x100000 | 0x100 | 4 | 0x1000000;
    #[cfg(target_os = "linux")]
    const DIRECTORY_FLAGS: i32 = 0x10000 | 0x20000 | 0x800 | 0x80000;
    #[cfg(target_os = "macos")]
    const FILE_FLAGS: i32 = 0x100 | 4 | 0x1000000;
    #[cfg(target_os = "linux")]
    const FILE_FLAGS: i32 = 0x20000 | 0x800 | 0x80000;
    #[cfg(target_os = "macos")]
    const APPEND_CREATE: i32 = 2 | 8 | 0x200;
    #[cfg(target_os = "linux")]
    const APPEND_CREATE: i32 = 2 | 0x400 | 0x40;
    if !path.is_absolute() {
        return Err(std::io::Error::other("console path must be absolute"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("console directory missing"))?;
    let mut directory = File::open("/")?;
    for component in parent.components() {
        let Component::Normal(name) = component else {
            if component == Component::RootDir {
                continue;
            }
            return Err(std::io::Error::other("noncanonical console directory"));
        };
        let name = CString::new(name.as_bytes()).map_err(std::io::Error::other)?;
        let mut fd = unsafe { openat(directory.as_raw_fd(), name.as_ptr(), DIRECTORY_FLAGS) };
        if fd < 0 && write && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound
        {
            if unsafe { mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o700) } != 0
                && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
            {
                return Err(std::io::Error::last_os_error());
            }
            fd = unsafe { openat(directory.as_raw_fd(), name.as_ptr(), DIRECTORY_FLAGS) };
        }
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // Each descriptor pins its directory while the next component is opened.
        // O_DIRECTORY/O_NOFOLLOW refuse every symbolic-link component.
        directory = unsafe { File::from_raw_fd(fd) };
    }
    let metadata = directory.metadata()?;
    if metadata.uid() != unsafe { geteuid() } || metadata.mode() & 0o077 != 0 {
        return Err(std::io::Error::other("console directory must be private"));
    }
    let name = CString::new(
        path.file_name()
            .ok_or_else(|| std::io::Error::other("console filename missing"))?
            .as_bytes(),
    )
    .map_err(std::io::Error::other)?;
    let fd = unsafe {
        openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            FILE_FLAGS | if write { APPEND_CREATE } else { 0 },
            0o600u32,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    check_file(&file)?;
    Ok(file)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn open_file(path: &Path, write: bool) -> std::io::Result<File> {
    if write {
        let parent = path
            .parent()
            .ok_or_else(|| std::io::Error::other("console directory missing"))?;
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(parent)?;
    }
    if fs::symlink_metadata(path).is_ok_and(|m| !m.is_file()) {
        return Err(std::io::Error::other("console links are refused"));
    }
    let mut options = OpenOptions::new();
    options.read(true).write(write).create(write).append(write);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
        #[cfg(target_os = "macos")]
        options.custom_flags(0x104); // O_NOFOLLOW | O_NONBLOCK
        #[cfg(target_os = "linux")]
        options.custom_flags(0x20800); // O_NOFOLLOW | O_NONBLOCK
    }
    let file = options.open(path)?;
    check_file(&file)?;
    Ok(file)
}

#[cfg(unix)]
fn lock(file: &File, exclusive: bool) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    // Closing this owned file releases the lock on every return path.
    if unsafe { flock(file.as_raw_fd(), (if exclusive { 2 } else { 1 }) | 4) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(unix))]
fn lock(_: &File, _: bool) -> std::io::Result<()> {
    Ok(())
}

fn append_file(path: &Path, event: &Event) -> std::io::Result<()> {
    let mut file = open_file(path, true)?;
    lock(&file, true)?;
    let mut line = serde_json::to_vec(event)?;
    line.push(b'\n');
    if file.metadata()?.len() + line.len() as u64 > MAX_FILE {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
    }
    file.write_all(&line)
}

fn read_file(path: &Path, limit: usize) -> std::io::Result<Vec<Event>> {
    let mut file = open_file(path, false)?;
    lock(&file, false)?;
    let length = file.metadata()?.len();
    let truncated = length > MAX_FILE;
    file.seek(SeekFrom::Start(length.saturating_sub(MAX_FILE)))?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE + 1).read_to_end(&mut bytes)?;
    let mut records = VecDeque::new();
    for (index, line) in bytes.split(|b| *b == b'\n').enumerate() {
        if truncated && index == 0 {
            continue;
        }
        if line.len() > 4096 {
            continue;
        }
        if let Ok(record) = serde_json::from_slice::<Event>(line) {
            records.push_back(record);
            if records.len() > limit {
                records.pop_front();
            }
        }
    }
    Ok(records.into_iter().collect())
}

pub fn record(level: Level, source: &str, operation: &str, message: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .unwrap_or(0);
    let event = Event {
        id: format!(
            "{now}-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ),
        timestamp_ms: now,
        level,
        source: clean(source, 40),
        operation: clean(operation, 64),
        message: clean(message, 240),
    };
    let mut events = memory().lock().unwrap_or_else(|e| e.into_inner());
    events.push_back(event.clone());
    if events.len() > MAX_MEMORY {
        events.pop_front();
    }
    if let Some(path) = log_path() {
        let _ = append_file(&path, &event);
    }
}

pub fn snapshot(limit: usize) -> Result<Snapshot, String> {
    if !(1..=200).contains(&limit) {
        return Err("Console limit must be between 1 and 200.".into());
    }
    if let Some(path) = log_path()
        && let Ok(records) = read_file(&path, limit)
    {
        return Ok(Snapshot {
            records,
            persistence: "disk",
            message: "Bounded OpenDrobo events shared by this user's app and MCP processes. System and kernel logs are not collected.",
        });
    }
    let events = memory().lock().unwrap_or_else(|e| e.into_inner());
    Ok(Snapshot {
        records: events
            .iter()
            .skip(events.len().saturating_sub(limit))
            .cloned()
            .collect(),
        persistence: "memory",
        message: "Private log storage unavailable; showing this process's bounded OpenDrobo events.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "opendrobo-console-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        path
    }

    fn event(id: &str) -> Event {
        Event {
            id: id.into(),
            timestamp_ms: 1,
            level: Level::Info,
            source: "test".into(),
            operation: "read".into(),
            message: "Read completed".into(),
        }
    }

    #[test]
    fn private_log_bounds_and_rotation() {
        let root = fixture();
        let path = root.join("events.jsonl");
        for i in 0..4 {
            append_file(&path, &event(&i.to_string())).unwrap();
        }
        let rows = read_file(&path, 2).unwrap();
        assert_eq!(
            rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["2", "3"]
        );
        let file = open_file(&path, true).unwrap();
        file.set_len(MAX_FILE).unwrap();
        drop(file);
        append_file(&path, &event("rotated")).unwrap();
        assert_eq!(read_file(&path, 200).unwrap()[0].id, "rotated");
        assert!(fs::metadata(&path).unwrap().len() < MAX_FILE);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn links_and_nonprivate_files_are_refused() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = fixture();
        let target = root.join("target");
        fs::write(&target, "private target remains unchanged").unwrap();
        let link = root.join("events.jsonl");
        symlink(&target, &link).unwrap();
        assert!(append_file(&link, &event("refused")).is_err());
        assert!(read_file(&link, 1).is_err());
        assert_eq!(
            fs::read_to_string(&target).unwrap(),
            "private target remains unchanged"
        );
        let private = root.join("private");
        fs::create_dir(&private).unwrap();
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        append_file(&private.join("events.jsonl"), &event("original")).unwrap();
        let parent_link = root.join("redirect");
        symlink(&private, &parent_link).unwrap();
        assert!(append_file(&parent_link.join("events.jsonl"), &event("refused")).is_err());
        assert!(read_file(&parent_link.join("events.jsonl"), 1).is_err());
        assert_eq!(
            read_file(&private.join("events.jsonl"), 1).unwrap()[0].id,
            "original"
        );
        fs::remove_file(&link).unwrap();
        fs::write(&link, b"{}").unwrap();
        fs::set_permissions(&link, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(append_file(&link, &event("refused")).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn limits_and_control_character_sanitization() {
        assert!(snapshot(0).is_err());
        assert!(snapshot(201).is_err());
        assert_eq!(clean("hello\n\r\0world", 20), "helloworld");
        assert_eq!(clean("ééé", 2), "éé");
    }
}
