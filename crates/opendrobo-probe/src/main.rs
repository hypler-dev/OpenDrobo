//! Bounded offline inspection for copied Drobo protocol snapshots.
//!
//! This binary opens only regular files, never hardware paths. It neither
//! mounts data nor submits requests to a Drobo.

use std::env;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::process::ExitCode;

use opendrobo_core::beyondraid::{RMR_WINDOW_BYTES, parse_rmr_candidate};
use opendrobo_core::thunderbolt::parse_port_snapshot;

const MAX_PORT_SNAPSHOT_BYTES: u64 = 16 * 1024 * 1024;

#[cfg(any(
    all(
        target_os = "macos",
        any(target_arch = "aarch64", target_arch = "x86_64")
    ),
    all(
        target_os = "linux",
        any(target_arch = "aarch64", target_arch = "x86_64")
    )
))]
const HOST_SUPPORTED: bool = true;

#[cfg(not(any(
    all(
        target_os = "macos",
        any(target_arch = "aarch64", target_arch = "x86_64")
    ),
    all(
        target_os = "linux",
        any(target_arch = "aarch64", target_arch = "x86_64")
    )
)))]
const HOST_SUPPORTED: bool = false;

const USAGE: &str = "usage:\n  opendrobo-probe port SNAPSHOT\n  opendrobo-probe rmr IMAGE OFFSET [EXPECTED_WORD14 EXPECTED_WORD18 [EXPECTED_PACK_ID_HEX]]\n\nAll inputs must be regular files. rmr OFFSET is decimal or 0x-prefixed unsigned.\nFor a revision-two RMR candidate, a caller-supplied identity check requires the pack ID.";

#[derive(Debug)]
enum ProbeError {
    Usage,
    Message(&'static str),
    Io(io::Error),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => f.write_str(USAGE),
            Self::Message(message) => f.write_str(message),
            Self::Io(error) => write!(f, "file read failed: {error}"),
        }
    }
}

impl From<io::Error> for ProbeError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn main() -> ExitCode {
    let arguments = env::args_os()
        .skip(1)
        .map(|argument| argument.into_string())
        .collect::<Result<Vec<_>, _>>();
    let result = arguments.map_or(Err(ProbeError::Usage), run);
    match result {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(ProbeError::Usage) => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

fn run(arguments: Vec<String>) -> Result<String, ProbeError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(ProbeError::Usage);
    };

    match command {
        "port" if arguments.len() == 2 => inspect_port(Path::new(&arguments[1])),
        "rmr" if matches!(arguments.len(), 3 | 5 | 6) => inspect_rmr(&arguments),
        _ => Err(ProbeError::Usage),
    }
}

fn inspect_port(path: &Path) -> Result<String, ProbeError> {
    let bytes = read_regular_capped(path, MAX_PORT_SNAPSHOT_BYTES)?;
    let snapshot =
        parse_port_snapshot(&bytes).map_err(|_| ProbeError::Message("snapshot format rejected"))?;

    let mut json = String::from("{\"kind\":\"copied-drobo-port-snapshot\",\"interfaceVersion\":");
    push_u32(&mut json, snapshot.capabilities.interface_version);
    json.push_str(",\"enabledLunCount\":");
    push_usize(&mut json, snapshot.luns.len());
    json.push_str(",\"managementRequestOffset\":");
    push_usize(&mut json, snapshot.directory.management_request_list_offset);
    json.push_str(",\"managementResponseOffset\":");
    push_usize(
        &mut json,
        snapshot.directory.management_response_list_offset,
    );
    json.push_str(",\"luns\":[");
    for (position, lun) in snapshot.luns.iter().enumerate() {
        if position != 0 {
            json.push(',');
        }
        json.push_str("{\"index\":");
        push_usize(&mut json, lun.index);
        json.push_str(",\"blockSize\":");
        push_u32(&mut json, lun.block_size);
        json.push_str(",\"logicalBlockCount\":\"");
        push_u64(&mut json, lun.logical_block_count);
        json.push_str("\",\"virtualByteCeiling\":\"");
        push_u64(&mut json, lun.logical_bytes);
        json.push_str("\"}");
    }
    json.push_str("],\"protectedCapacity\":\"unavailable\",\"hardwareValidated\":false,\"virtualCeilingIsActualPhysicalCapacity\":false}");
    Ok(json)
}

fn inspect_rmr(arguments: &[String]) -> Result<String, ProbeError> {
    let offset = parse_u64(&arguments[2])?;
    let window_end = offset
        .checked_add(RMR_WINDOW_BYTES as u64)
        .ok_or(ProbeError::Message(
            "RMR offset overflows the required window",
        ))?;
    let mut file = open_regular_readonly(Path::new(&arguments[1]))?;
    let size = file.metadata()?.len();
    if size < window_end {
        return Err(ProbeError::Message(
            "RMR file is shorter than the required window",
        ));
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut window = [0u8; RMR_WINDOW_BYTES];
    file.read_exact(&mut window)?;

    let candidate = parse_rmr_candidate(&window)
        .map_err(|_| ProbeError::Message("RMR candidate format rejected"))?;
    let identity = if arguments.len() == 3 {
        "unverified"
    } else {
        let expected_word_14 = parse_u32(&arguments[3])?;
        let expected_word_18 = parse_u32(&arguments[4])?;
        let expected_pack_id = if arguments.len() == 6 {
            Some(parse_pack_id(&arguments[5])?)
        } else {
            None
        };
        candidate
            .check_identity(expected_word_14, expected_word_18, expected_pack_id)
            .map_err(|_| ProbeError::Message("caller-supplied identity values do not match"))?;
        "caller-supplied values match; provenance unverified"
    };

    let mut json = String::from("{\"kind\":\"rmr-header-candidate\",\"revision\":");
    push_u32(&mut json, candidate.revision);
    json.push_str(",\"callerWord14\":");
    push_u32(&mut json, candidate.caller_word_14);
    json.push_str(",\"callerWord18\":");
    push_u32(&mut json, candidate.caller_word_18);
    json.push_str(",\"rawWord1c\":");
    push_u32(&mut json, candidate.raw_word_1c);
    json.push_str(",\"geometryShift\":");
    push_u32(&mut json, candidate.geometry_shift);
    json.push_str(",\"packId\":\"");
    push_hex(&mut json, &candidate.pack_id);
    json.push_str("\",\"identity\":\"");
    json.push_str(identity);
    json.push_str("\",\"structuralCrcMatched\":true,\"arrayValidated\":false,\"protectedCapacity\":\"unavailable\",\"hardwareValidated\":false}");
    Ok(json)
}

fn read_regular_capped(path: &Path, cap: u64) -> Result<Vec<u8>, ProbeError> {
    let file = open_regular_readonly(path)?;
    if file.metadata()?.len() > cap {
        return Err(ProbeError::Message(
            "snapshot exceeds the 16 MiB read limit",
        ));
    }
    let mut bytes = Vec::new();
    file.take(cap + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > cap {
        return Err(ProbeError::Message(
            "snapshot exceeds the 16 MiB read limit",
        ));
    }
    Ok(bytes)
}

fn open_regular_readonly(path: &Path) -> Result<File, ProbeError> {
    if !HOST_SUPPORTED {
        return Err(ProbeError::Message(
            "unsupported host: regular-file opening is implemented only for macOS/Linux on x86_64/aarch64",
        ));
    }
    let initial = fs::symlink_metadata(path)?;
    if !initial.file_type().is_file() {
        return Err(ProbeError::Message("input is not a regular file"));
    }

    let mut options = OpenOptions::new();
    options.read(true);
    add_platform_safe_open_flags(&mut options);
    let file = options.open(path)?;
    if !file.metadata()?.file_type().is_file() {
        return Err(ProbeError::Message("opened input is not a regular file"));
    }
    Ok(file)
}

#[cfg(any(
    all(
        target_os = "macos",
        any(target_arch = "aarch64", target_arch = "x86_64")
    ),
    all(
        target_os = "linux",
        any(target_arch = "aarch64", target_arch = "x86_64")
    )
))]
fn add_platform_safe_open_flags(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;

    #[cfg(target_os = "macos")]
    const FLAGS: i32 = 0x4 | 0x100; // O_NONBLOCK | O_NOFOLLOW from macOS <fcntl.h>
    #[cfg(target_os = "linux")]
    const FLAGS: i32 = 0x800 | 0x20000; // O_NONBLOCK | O_NOFOLLOW from Linux <fcntl.h>
    options.custom_flags(FLAGS);
}

// Unsupported hosts fail closed before metadata or a file open. Windows and
// non-x86_64/non-aarch64 targets have not been validated for this probe.
#[cfg(not(any(
    all(
        target_os = "macos",
        any(target_arch = "aarch64", target_arch = "x86_64")
    ),
    all(
        target_os = "linux",
        any(target_arch = "aarch64", target_arch = "x86_64")
    )
)))]
fn add_platform_safe_open_flags(_options: &mut OpenOptions) {}

fn parse_u64(input: &str) -> Result<u64, ProbeError> {
    let value = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
        .map_or_else(
            || u64::from_str_radix(input, 10),
            |hex| u64::from_str_radix(hex, 16),
        );
    value.map_err(|_| ProbeError::Message("expected an unsigned decimal or 0x-prefixed integer"))
}

fn parse_u32(input: &str) -> Result<u32, ProbeError> {
    let value = parse_u64(input)?;
    u32::try_from(value).map_err(|_| ProbeError::Message("value exceeds the u32 range"))
}

fn parse_pack_id(input: &str) -> Result<[u8; 8], ProbeError> {
    if input.len() != 16 || !input.as_bytes().iter().all(u8::is_ascii_hexdigit) {
        return Err(ProbeError::Message(
            "expected exactly 16 ASCII hexadecimal characters",
        ));
    }
    let mut pack_id = [0u8; 8];
    for (index, byte) in pack_id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&input[index * 2..index * 2 + 2], 16)
            .map_err(|_| ProbeError::Message("expected exactly 16 ASCII hexadecimal characters"))?;
    }
    Ok(pack_id)
}

fn push_u32(output: &mut String, value: u32) {
    output.push_str(&value.to_string());
}

fn push_u64(output: &mut String, value: u64) {
    output.push_str(&value.to_string());
}

fn push_usize(output: &mut String, value: usize) {
    output.push_str(&value.to_string());
}

fn push_hex(output: &mut String, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        env::temp_dir().join(format!(
            "opendrobo-probe-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn write_fixture(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = temp_path(name);
        let mut file = File::create(&path).expect("create temporary fixture");
        file.write_all(bytes).expect("write temporary fixture");
        file.sync_all().expect("sync temporary fixture");
        path
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn valid_port_snapshot() -> Vec<u8> {
        let mut bytes = vec![0u8; 0x500];
        bytes[..10].copy_from_slice(b"DroboPort\0");
        put_u32(&mut bytes, 0x18, 0x100);
        put_u32(&mut bytes, 0x1c, 0x400);
        put_u32(&mut bytes, 0x20, 0x80);
        put_u32(&mut bytes, 0x24, 0x84);
        put_u32(&mut bytes, 0x28, 0x88);
        put_u32(&mut bytes, 0x2c, 0x8c);
        put_u32(&mut bytes, 0x30, 0x90);
        put_u32(&mut bytes, 0x34, 0x94);
        put_u32(&mut bytes, 0x38, 0x98);
        put_u32(&mut bytes, 0x3c, 0x9c);
        put_u32(&mut bytes, 0x40, 0xa0);
        put_u32(&mut bytes, 0x400, 4);
        put_u32(&mut bytes, 0x410, 32);
        put_u32(&mut bytes, 0x100, 1);
        put_u32(&mut bytes, 0x104, 4096);
        put_u64(&mut bytes, 0x108, 7);
        bytes
    }

    fn crc_update(mut crc: u32, bytes: &[u8]) -> u32 {
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        crc
    }

    fn valid_rmr(revision: u32) -> [u8; RMR_WINDOW_BYTES] {
        let mut bytes = [0u8; RMR_WINDOW_BYTES];
        put_u32(&mut bytes, 0, 0x726d_7268);
        put_u32(&mut bytes, 4, 0x4246_432d);
        put_u32(&mut bytes, 8, revision);
        put_u32(&mut bytes, 12, RMR_WINDOW_BYTES as u32);
        put_u32(&mut bytes, 0x14, 0x1122_3344);
        put_u32(&mut bytes, 0x18, 0x5566_7800);
        put_u32(&mut bytes, 0x20, 12);
        if revision == 1 {
            put_u32(&mut bytes, 0x1c, 1);
        } else {
            bytes[0x88..0x90].copy_from_slice(b"testpack");
        }
        let crc = crc_update(0, &bytes[..0x10]);
        let crc = crc_update(crc, &[0; 4]);
        let crc = crc_update(crc, &bytes[0x14..]);
        put_u32(&mut bytes, 0x10, crc);
        bytes
    }

    #[test]
    fn port_json_marks_the_virtual_ceiling_nonphysical_and_keeps_fixture_bytes() {
        let fixture = valid_port_snapshot();
        let path = write_fixture("port", &fixture);
        let json = inspect_port(&path).expect("inspect port fixture");
        assert!(json.contains("\"virtualByteCeiling\":\"28672\""));
        assert!(json.contains("\"virtualCeilingIsActualPhysicalCapacity\":false"));
        assert_eq!(fs::read(&path).expect("read fixture after probe"), fixture);
        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn rmr_reads_only_the_window_at_an_explicit_nonzero_offset() {
        let fixture = valid_rmr(2);
        let offset = 37usize;
        let mut image = vec![0xaa; offset];
        image.extend_from_slice(&fixture);
        image.extend_from_slice(&[0xbb; 19]);
        let before = image.clone();
        let path = write_fixture("rmr", &image);
        let json = run(vec![
            "rmr".into(),
            path.to_string_lossy().into_owned(),
            format!("0x{offset:x}"),
        ])
        .expect("inspect unbound RMR candidate");
        assert!(json.contains("\"structuralCrcMatched\":true"));
        assert!(json.contains("\"identity\":\"unverified\""));
        assert!(json.contains("\"arrayValidated\":false"));
        assert_eq!(fs::read(&path).expect("read fixture after probe"), before);
        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn rmr_rejects_invalid_crc_and_mismatched_identity_material() {
        let mut bad_crc = valid_rmr(1);
        bad_crc[0x200] ^= 0x80;
        let bad_crc_path = write_fixture("bad-crc", &bad_crc);
        assert!(
            inspect_rmr(&[
                "rmr".into(),
                bad_crc_path.to_string_lossy().into_owned(),
                "0".into()
            ])
            .is_err()
        );
        fs::remove_file(bad_crc_path).expect("remove fixture");

        let rev_one_path = write_fixture("rev-one", &valid_rmr(1));
        assert!(
            run(vec![
                "rmr".into(),
                rev_one_path.to_string_lossy().into_owned(),
                "0".into(),
                "1".into(),
                "0x55667800".into()
            ])
            .is_err()
        );
        fs::remove_file(rev_one_path).expect("remove fixture");

        let rev_two_path = write_fixture("rev-two", &valid_rmr(2));
        assert!(
            run(vec![
                "rmr".into(),
                rev_two_path.to_string_lossy().into_owned(),
                "0".into(),
                "0x11223344".into(),
                "0x55667800".into(),
                "0011223344556677".into()
            ])
            .is_err()
        );
        fs::remove_file(rev_two_path).expect("remove fixture");
    }

    #[test]
    fn malformed_arguments_numeric_overflow_and_hex_are_rejected() {
        assert!(matches!(run(vec!["port".into()]), Err(ProbeError::Usage)));
        assert!(parse_u64("-1").is_err());
        assert!(parse_u64("0x").is_err());
        assert!(parse_u32("4294967296").is_err());
        assert!(parse_pack_id("001122334455667").is_err());
        assert!(parse_pack_id("001122334455667z").is_err());
        assert!(parse_u64("18446744073709551615").is_ok());
    }

    #[test]
    fn truncated_files_and_offset_overflow_are_rejected() {
        let short_port = write_fixture("short-port", &[0; 8]);
        assert!(inspect_port(&short_port).is_err());
        fs::remove_file(short_port).expect("remove fixture");

        let short_rmr = write_fixture("short-rmr", &[0; RMR_WINDOW_BYTES - 1]);
        assert!(
            inspect_rmr(&[
                "rmr".into(),
                short_rmr.to_string_lossy().into_owned(),
                "0".into()
            ])
            .is_err()
        );
        assert!(
            inspect_rmr(&[
                "rmr".into(),
                short_rmr.to_string_lossy().into_owned(),
                u64::MAX.to_string()
            ])
            .is_err()
        );
        fs::remove_file(short_rmr).expect("remove fixture");
    }

    #[test]
    fn snapshot_cap_accepts_the_boundary_and_rejects_one_byte_over() {
        let boundary = write_fixture("cap-boundary", &vec![0; MAX_PORT_SNAPSHOT_BYTES as usize]);
        assert_eq!(
            read_regular_capped(&boundary, MAX_PORT_SNAPSHOT_BYTES)
                .unwrap()
                .len(),
            MAX_PORT_SNAPSHOT_BYTES as usize
        );
        fs::remove_file(boundary).expect("remove fixture");

        let over = write_fixture("cap-over", &vec![0; MAX_PORT_SNAPSHOT_BYTES as usize + 1]);
        assert!(read_regular_capped(&over, MAX_PORT_SNAPSHOT_BYTES).is_err());
        fs::remove_file(over).expect("remove fixture");
    }

    #[test]
    fn directories_and_unix_symlinks_are_rejected_before_opening() {
        let directory = temp_path("directory");
        fs::create_dir(&directory).expect("create temporary directory");
        assert!(open_regular_readonly(&directory).is_err());
        fs::remove_dir(directory).expect("remove temporary directory");

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            use std::process::Command;
            let target = write_fixture("symlink-target", &[0; 16]);
            let link = temp_path("symlink");
            symlink(&target, &link).expect("create temporary symlink");
            assert!(open_regular_readonly(&link).is_err());
            fs::remove_file(link).expect("remove symlink");
            fs::remove_file(target).expect("remove target");

            let fifo = temp_path("fifo");
            let status = Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .expect("run mkfifo for temporary fixture");
            assert!(status.success(), "mkfifo failed");
            assert!(open_regular_readonly(&fifo).is_err());
            fs::remove_file(fifo).expect("remove FIFO");
        }
    }

    #[cfg(not(any(
        all(
            target_os = "macos",
            any(target_arch = "aarch64", target_arch = "x86_64")
        ),
        all(
            target_os = "linux",
            any(target_arch = "aarch64", target_arch = "x86_64")
        )
    )))]
    #[test]
    fn unsupported_hosts_reject_before_metadata_or_open() {
        assert!(matches!(
            open_regular_readonly(Path::new("not-opened-by-this-test")),
            Err(ProbeError::Message(
                "unsupported host: this probe is validated only on macOS or Linux x86_64/aarch64"
            ))
        ));
    }
}
