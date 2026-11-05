//! Offline codecs for recovered Drobo Thunderbolt shared-memory layouts.
//!
//! These helpers only validate and encode byte slices. They do not map PCI
//! memory, allocate DMA buffers, submit SCSI commands, or perform device I/O.
//! The logical byte count exposed here is the device's virtual LUN ceiling; it
//! is not a BeyondRAID protected-capacity measurement.

use core::fmt;

/// Fixed prefix at the beginning of a recovered DroboPort BAR1 root.
pub const PORT_SIGNATURE: &[u8; 10] = b"DroboPort\0";
/// Minimum recovered DroboPort directory size.
pub const PORT_DIRECTORY_BYTES: usize = 0x44;
/// Recovered capability block size.
pub const CAPABILITIES_BYTES: usize = 64;
/// Recovered LUN record size.
pub const LUN_RECORD_BYTES: usize = 16;
/// The original initialization path scans this many LUN slots.
pub const MAX_LUNS: usize = 32;
/// Request burst header size.
pub const REQUEST_BURST_HEADER_BYTES: usize = 8;
/// Request burst entry size.
pub const REQUEST_BURST_ENTRY_BYTES: usize = 16;
/// Completion burst header size.
pub const COMPLETION_BURST_HEADER_BYTES: usize = 8;
/// Completion entry size for interface versions one through three.
pub const LEGACY_COMPLETION_ENTRY_BYTES: usize = 8;
/// Completion entry size for interface version four.
pub const V4_COMPLETION_ENTRY_BYTES: usize = 32;

const MAX_FRAME_ENTRIES: u32 = 65_536;

/// Offline decoding and validation failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThunderboltError {
    Truncated,
    InvalidPortSignature,
    OffsetInsideHeader,
    OffsetOutOfRange,
    OffsetMisaligned,
    UnsupportedInterfaceVersion(u32),
    TooManyLogicalUnits(u32),
    InvalidLogicalUnit { index: usize },
    ArithmeticOverflow,
    CountExceedsWindow,
    CountExceedsLimit,
    AllOnesCount,
    UnexpectedCompletion { queue_index: u32 },
    CompletionTransactionMismatch { queue_index: u32 },
    DuplicateCompletion { queue_index: u32 },
    CompletionTooLarge { queue_index: u32 },
    CompletionStatusFailed { queue_index: u32, srb_response: u32 },
}

impl fmt::Display for ThunderboltError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("Thunderbolt frame is truncated"),
            Self::InvalidPortSignature => f.write_str("invalid DroboPort signature"),
            Self::OffsetInsideHeader => f.write_str("directory offset points inside the header"),
            Self::OffsetOutOfRange => f.write_str("directory offset is outside the snapshot"),
            Self::OffsetMisaligned => f.write_str("directory offset has invalid alignment"),
            Self::UnsupportedInterfaceVersion(version) => {
                write!(f, "unsupported Thunderbolt interface version {version}")
            }
            Self::TooManyLogicalUnits(count) => write!(f, "unsupported LUN count {count}"),
            Self::InvalidLogicalUnit { index } => write!(f, "invalid LUN record {index}"),
            Self::ArithmeticOverflow => f.write_str("Thunderbolt frame arithmetic overflow"),
            Self::CountExceedsWindow => f.write_str("frame count exceeds its window"),
            Self::CountExceedsLimit => f.write_str("frame count exceeds caller limit"),
            Self::AllOnesCount => f.write_str("frame count is the removed-device sentinel"),
            Self::UnexpectedCompletion { queue_index } => {
                write!(f, "unexpected completion queue index {queue_index}")
            }
            Self::CompletionTransactionMismatch { queue_index } => {
                write!(
                    f,
                    "completion transaction mismatch for queue index {queue_index}"
                )
            }
            Self::DuplicateCompletion { queue_index } => {
                write!(f, "duplicate completion for queue index {queue_index}")
            }
            Self::CompletionTooLarge { queue_index } => {
                write!(
                    f,
                    "completion realized too many bytes for queue index {queue_index}"
                )
            }
            Self::CompletionStatusFailed {
                queue_index,
                srb_response,
            } => write!(
                f,
                "completion SRB response {srb_response:#x} failed for queue index {queue_index}"
            ),
        }
    }
}

impl std::error::Error for ThunderboltError {}

/// Relative offsets recovered from the read-only DroboPort directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortDirectory {
    pub lun_table_offset: usize,
    pub capabilities_offset: usize,
    pub scsi_request_list_offset: usize,
    pub response_list_offset: usize,
    pub system_request_list_offset: usize,
    pub system_response_list_offset: usize,
    pub management_request_list_offset: usize,
    pub management_response_list_offset: usize,
    pub pci_interrupt_block_offset: usize,
    pub endpoint_interrupt_block_offset: usize,
    pub inquiry_identity_offset: usize,
}

/// The recovered portion of the 64-byte capability block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortCapabilities {
    pub interface_version: u32,
    pub transfer_mode: u32,
    pub max_transfer_bytes: u32,
    pub max_outstanding_transactions: u32,
    pub max_luns: u32,
    pub port_flags: u32,
    pub max_burst: u32,
    pub stall_size: u32,
    pub max_sgl_entries: u32,
    pub host_interface_version: u32,
}

/// An enabled logical unit from the fixed 32-record table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogicalUnit {
    pub index: usize,
    pub block_size: u32,
    pub logical_block_count: u64,
    /// Virtual logical ceiling reported by the original driver's LUN table.
    /// This is not protected capacity and must not be used as such.
    pub logical_bytes: u64,
}

/// A portable, immutable view of recovered DroboPort data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortSnapshot {
    pub directory: PortDirectory,
    pub capabilities: PortCapabilities,
    pub luns: Vec<LogicalUnit>,
}

impl PortSnapshot {
    /// Returns the sum of enabled LUN virtual ceilings, not protected capacity.
    pub fn virtual_byte_ceiling(&self) -> Result<u64, ThunderboltError> {
        self.luns.iter().try_fold(0_u64, |total, lun| {
            total
                .checked_add(lun.logical_bytes)
                .ok_or(ThunderboltError::ArithmeticOverflow)
        })
    }
}

/// Decodes a complete, copied DroboPort snapshot without mutating its source.
pub fn parse_port_snapshot(bytes: &[u8]) -> Result<PortSnapshot, ThunderboltError> {
    let header = bytes
        .get(..PORT_DIRECTORY_BYTES)
        .ok_or(ThunderboltError::Truncated)?;
    if header.get(..PORT_SIGNATURE.len()) != Some(PORT_SIGNATURE) {
        return Err(ThunderboltError::InvalidPortSignature);
    }

    let directory = PortDirectory {
        lun_table_offset: directory_offset(bytes, 0x18, 8)?,
        capabilities_offset: directory_offset(bytes, 0x1c, 4)?,
        scsi_request_list_offset: directory_offset(bytes, 0x20, 4)?,
        response_list_offset: directory_offset(bytes, 0x24, 4)?,
        system_request_list_offset: directory_offset(bytes, 0x28, 4)?,
        system_response_list_offset: directory_offset(bytes, 0x2c, 4)?,
        management_request_list_offset: directory_offset(bytes, 0x30, 4)?,
        management_response_list_offset: directory_offset(bytes, 0x34, 4)?,
        pci_interrupt_block_offset: directory_offset(bytes, 0x38, 4)?,
        endpoint_interrupt_block_offset: directory_offset(bytes, 0x3c, 4)?,
        inquiry_identity_offset: directory_offset(bytes, 0x40, 1)?,
    };

    let capabilities_bytes = slice_at(bytes, directory.capabilities_offset, CAPABILITIES_BYTES)?;
    let capabilities = PortCapabilities {
        interface_version: read_u32(capabilities_bytes, 0)?,
        transfer_mode: read_u32(capabilities_bytes, 4)?,
        max_transfer_bytes: read_u32(capabilities_bytes, 8)?,
        max_outstanding_transactions: read_u32(capabilities_bytes, 12)?,
        max_luns: read_u32(capabilities_bytes, 16)?,
        port_flags: read_u32(capabilities_bytes, 20)?,
        max_burst: read_u32(capabilities_bytes, 24)?,
        stall_size: read_u32(capabilities_bytes, 28)?,
        max_sgl_entries: read_u32(capabilities_bytes, 32)?,
        host_interface_version: read_u32(capabilities_bytes, 36)?,
    };
    if !(1..=4).contains(&capabilities.interface_version) {
        return Err(ThunderboltError::UnsupportedInterfaceVersion(
            capabilities.interface_version,
        ));
    }
    if capabilities.max_luns > MAX_LUNS as u32 {
        return Err(ThunderboltError::TooManyLogicalUnits(capabilities.max_luns));
    }

    let luns_bytes = slice_at(
        bytes,
        directory.lun_table_offset,
        LUN_RECORD_BYTES
            .checked_mul(MAX_LUNS)
            .ok_or(ThunderboltError::ArithmeticOverflow)?,
    )?;
    let mut luns = Vec::new();
    for index in 0..MAX_LUNS {
        let offset = index
            .checked_mul(LUN_RECORD_BYTES)
            .ok_or(ThunderboltError::ArithmeticOverflow)?;
        if read_u32(luns_bytes, offset)? == 0 {
            continue;
        }
        let block_size = read_u32(luns_bytes, offset + 4)?;
        let logical_block_count = read_u64(luns_bytes, offset + 8)?;
        if block_size == 0 || logical_block_count == 0 {
            return Err(ThunderboltError::InvalidLogicalUnit { index });
        }
        let logical_bytes = u64::from(block_size)
            .checked_mul(logical_block_count)
            .ok_or(ThunderboltError::ArithmeticOverflow)?;
        luns.push(LogicalUnit {
            index,
            block_size,
            logical_block_count,
            logical_bytes,
        });
    }

    Ok(PortSnapshot {
        directory,
        capabilities,
        luns,
    })
}

/// A neutral request-list entry. `dma_address` is serialized only; this module
/// neither allocates nor dereferences it. No wire-alignment rule was recovered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestEntry {
    pub dma_address: u64,
    pub request_length: u32,
    pub transaction_id: u32,
}

/// A copied request burst, parsed under a caller-supplied allocation limit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestBurst {
    pub window: u32,
    pub entries: Vec<RequestEntry>,
}

/// Encodes a neutral request burst. It does not submit the resulting bytes.
pub fn encode_request_burst(
    window: u32,
    entries: &[RequestEntry],
) -> Result<Vec<u8>, ThunderboltError> {
    let count = checked_entry_count(entries.len())?;
    if count > window {
        return Err(ThunderboltError::CountExceedsWindow);
    }
    let length = frame_length(REQUEST_BURST_HEADER_BYTES, REQUEST_BURST_ENTRY_BYTES, count)?;
    let mut encoded = vec![0_u8; length];
    encoded[..4].copy_from_slice(&window.to_le_bytes());
    encoded[4..8].copy_from_slice(&count.to_le_bytes());
    for (index, entry) in entries.iter().enumerate() {
        let offset = REQUEST_BURST_HEADER_BYTES
            .checked_add(
                index
                    .checked_mul(REQUEST_BURST_ENTRY_BYTES)
                    .ok_or(ThunderboltError::ArithmeticOverflow)?,
            )
            .ok_or(ThunderboltError::ArithmeticOverflow)?;
        encoded[offset..offset + 4].copy_from_slice(&(entry.dma_address as u32).to_le_bytes());
        encoded[offset + 4..offset + 8]
            .copy_from_slice(&((entry.dma_address >> 32) as u32).to_le_bytes());
        encoded[offset + 8..offset + 12].copy_from_slice(&entry.request_length.to_le_bytes());
        encoded[offset + 12..offset + 16].copy_from_slice(&entry.transaction_id.to_le_bytes());
    }
    Ok(encoded)
}

/// Parses a copied request burst using a caller-defined entry limit.
pub fn parse_request_burst(
    bytes: &[u8],
    max_entries: u32,
) -> Result<RequestBurst, ThunderboltError> {
    let header = bytes
        .get(..REQUEST_BURST_HEADER_BYTES)
        .ok_or(ThunderboltError::Truncated)?;
    let window = read_u32(header, 0)?;
    let count = read_u32(header, 4)?;
    validate_count(count, window, max_entries)?;
    let expected_length =
        frame_length(REQUEST_BURST_HEADER_BYTES, REQUEST_BURST_ENTRY_BYTES, count)?;
    let frame = bytes
        .get(..expected_length)
        .ok_or(ThunderboltError::Truncated)?;
    let mut entries = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        let offset = REQUEST_BURST_HEADER_BYTES
            .checked_add(
                index
                    .checked_mul(REQUEST_BURST_ENTRY_BYTES)
                    .ok_or(ThunderboltError::ArithmeticOverflow)?,
            )
            .ok_or(ThunderboltError::ArithmeticOverflow)?;
        let dma_address =
            u64::from(read_u32(frame, offset)?) | (u64::from(read_u32(frame, offset + 4)?) << 32);
        entries.push(RequestEntry {
            dma_address,
            request_length: read_u32(frame, offset + 8)?,
            transaction_id: read_u32(frame, offset + 12)?,
        });
    }
    Ok(RequestBurst { window, entries })
}

/// Version-specific completion fields recovered from a completion list entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionFields {
    /// Interface versions one through three expose only queue and transaction.
    Legacy,
    /// Interface version four has a realized byte count and raw response words.
    V4 {
        realized_bytes: u64,
        /// Raw field at entry offset `0x14`; the original driver accepts 1 or
        /// `0x12` as successful SRB responses.
        srb_response: u32,
        /// Raw field at entry offset `0x18`; no success interpretation here.
        scsi_status: u32,
        // The words at entry offsets 0x10 and 0x1c are intentionally not
        // exposed: their semantics were not recovered.
    },
}

/// A neutral completion-list entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletionEntry {
    pub queue_index: u32,
    pub transaction_id: u32,
    pub fields: CompletionFields,
}

/// Expected data needed to reject stale or oversized completions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedCompletion {
    pub queue_index: u32,
    pub transaction_id: u32,
    pub maximum_request_bytes: u64,
}

/// Parses a completion burst without interpreting it as successful I/O.
///
/// `window` must come from the matching request channel. `outstanding_limit`
/// bounds allocation independently of data supplied by the frame.
pub fn parse_completion_burst(
    bytes: &[u8],
    interface_version: u32,
    window: u32,
    outstanding_limit: u32,
) -> Result<Vec<CompletionEntry>, ThunderboltError> {
    let entry_bytes = completion_entry_bytes(interface_version)?;
    let header = bytes
        .get(..COMPLETION_BURST_HEADER_BYTES)
        .ok_or(ThunderboltError::Truncated)?;
    let count = read_u32(header, 4)?;
    if count == u32::MAX {
        return Err(ThunderboltError::AllOnesCount);
    }
    validate_count(count, window, outstanding_limit)?;
    let expected_length = frame_length(COMPLETION_BURST_HEADER_BYTES, entry_bytes, count)?;
    let frame = bytes
        .get(..expected_length)
        .ok_or(ThunderboltError::Truncated)?;
    let mut completions = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        let offset = COMPLETION_BURST_HEADER_BYTES
            .checked_add(
                index
                    .checked_mul(entry_bytes)
                    .ok_or(ThunderboltError::ArithmeticOverflow)?,
            )
            .ok_or(ThunderboltError::ArithmeticOverflow)?;
        let fields = if interface_version == 4 {
            CompletionFields::V4 {
                realized_bytes: read_u64(frame, offset + 8)?,
                srb_response: read_u32(frame, offset + 0x14)?,
                scsi_status: read_u32(frame, offset + 0x18)?,
            }
        } else {
            CompletionFields::Legacy
        };
        completions.push(CompletionEntry {
            queue_index: read_u32(frame, offset)?,
            transaction_id: read_u32(frame, offset + 4)?,
            fields,
        });
    }
    Ok(completions)
}

/// Matches completions to caller-owned outstanding requests.
///
/// A valid count alone never means success. Version-four SRB response words are
/// checked against the two values accepted by the recovered original driver;
/// raw SCSI status remains available to a higher-level policy.
pub fn validate_completions(
    completions: &[CompletionEntry],
    expected: &[ExpectedCompletion],
) -> Result<(), ThunderboltError> {
    for (position, completion) in completions.iter().enumerate() {
        if completions[..position]
            .iter()
            .any(|prior| prior.queue_index == completion.queue_index)
        {
            return Err(ThunderboltError::DuplicateCompletion {
                queue_index: completion.queue_index,
            });
        }
        let Some(request) = expected
            .iter()
            .find(|request| request.queue_index == completion.queue_index)
        else {
            return Err(ThunderboltError::UnexpectedCompletion {
                queue_index: completion.queue_index,
            });
        };
        if request.transaction_id != completion.transaction_id {
            return Err(ThunderboltError::CompletionTransactionMismatch {
                queue_index: completion.queue_index,
            });
        }
        if let CompletionFields::V4 {
            realized_bytes,
            srb_response,
            ..
        } = completion.fields
        {
            if realized_bytes > request.maximum_request_bytes {
                return Err(ThunderboltError::CompletionTooLarge {
                    queue_index: completion.queue_index,
                });
            }
            if srb_response != 1 && srb_response != 0x12 {
                return Err(ThunderboltError::CompletionStatusFailed {
                    queue_index: completion.queue_index,
                    srb_response,
                });
            }
        }
    }
    Ok(())
}

fn directory_offset(
    bytes: &[u8],
    field_offset: usize,
    alignment: usize,
) -> Result<usize, ThunderboltError> {
    let offset = read_u32(bytes, field_offset)? as usize;
    if offset < PORT_DIRECTORY_BYTES {
        return Err(ThunderboltError::OffsetInsideHeader);
    }
    if offset >= bytes.len() {
        return Err(ThunderboltError::OffsetOutOfRange);
    }
    if offset % alignment != 0 {
        return Err(ThunderboltError::OffsetMisaligned);
    }
    Ok(offset)
}

fn slice_at(bytes: &[u8], offset: usize, length: usize) -> Result<&[u8], ThunderboltError> {
    let end = offset
        .checked_add(length)
        .ok_or(ThunderboltError::ArithmeticOverflow)?;
    bytes.get(offset..end).ok_or(ThunderboltError::Truncated)
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ThunderboltError> {
    let raw = slice_at(bytes, offset, 4)?;
    Ok(u32::from_le_bytes(raw.try_into().expect("fixed length")))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ThunderboltError> {
    let raw = slice_at(bytes, offset, 8)?;
    Ok(u64::from_le_bytes(raw.try_into().expect("fixed length")))
}

fn checked_entry_count(count: usize) -> Result<u32, ThunderboltError> {
    let count = u32::try_from(count).map_err(|_| ThunderboltError::CountExceedsLimit)?;
    if count > MAX_FRAME_ENTRIES {
        return Err(ThunderboltError::CountExceedsLimit);
    }
    Ok(count)
}

fn validate_count(count: u32, window: u32, caller_limit: u32) -> Result<(), ThunderboltError> {
    if count > window {
        return Err(ThunderboltError::CountExceedsWindow);
    }
    if count > caller_limit || count > MAX_FRAME_ENTRIES {
        return Err(ThunderboltError::CountExceedsLimit);
    }
    Ok(())
}

fn frame_length(
    header_bytes: usize,
    entry_bytes: usize,
    count: u32,
) -> Result<usize, ThunderboltError> {
    let entries = (count as usize)
        .checked_mul(entry_bytes)
        .ok_or(ThunderboltError::ArithmeticOverflow)?;
    header_bytes
        .checked_add(entries)
        .ok_or(ThunderboltError::ArithmeticOverflow)
}

fn completion_entry_bytes(interface_version: u32) -> Result<usize, ThunderboltError> {
    match interface_version {
        1..=3 => Ok(LEGACY_COMPLETION_ENTRY_BYTES),
        4 => Ok(V4_COMPLETION_ENTRY_BYTES),
        other => Err(ThunderboltError::UnsupportedInterfaceVersion(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAP_OFFSET: usize = 0x80;
    const LUN_OFFSET: usize = 0xc0;
    const LIST_OFFSET: usize = 0x60;

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    // Synthetic fixture, not captured hardware data.
    fn snapshot_fixture() -> Vec<u8> {
        let mut bytes = vec![0_u8; 0x380];
        bytes[..PORT_SIGNATURE.len()].copy_from_slice(PORT_SIGNATURE);
        bytes[10..16].copy_from_slice(b"paddin");
        put_u32(&mut bytes, 0x18, LUN_OFFSET as u32);
        put_u32(&mut bytes, 0x1c, CAP_OFFSET as u32);
        put_u32(&mut bytes, 0x20, LIST_OFFSET as u32);
        put_u32(&mut bytes, 0x24, (LIST_OFFSET + 8) as u32);
        put_u32(&mut bytes, 0x28, (LIST_OFFSET + 16) as u32);
        put_u32(&mut bytes, 0x2c, (LIST_OFFSET + 24) as u32);
        put_u32(&mut bytes, 0x30, (LIST_OFFSET + 32) as u32);
        put_u32(&mut bytes, 0x34, (LIST_OFFSET + 40) as u32);
        put_u32(&mut bytes, 0x38, (LIST_OFFSET + 32) as u32);
        put_u32(&mut bytes, 0x3c, (LIST_OFFSET + 36) as u32);
        put_u32(&mut bytes, 0x40, (LIST_OFFSET + 40) as u32);
        put_u32(&mut bytes, CAP_OFFSET, 4);
        put_u32(&mut bytes, CAP_OFFSET + 16, MAX_LUNS as u32);
        put_u32(&mut bytes, LUN_OFFSET, 1);
        put_u32(&mut bytes, LUN_OFFSET + 4, 4096);
        put_u64(&mut bytes, LUN_OFFSET + 8, 8);
        bytes
    }

    #[test]
    fn snapshot_accepts_c_string_signature_with_nonzero_padding() {
        let snapshot = parse_port_snapshot(&snapshot_fixture()).unwrap();
        assert_eq!(snapshot.luns.len(), 1);
        assert_eq!(snapshot.luns[0].logical_bytes, 32_768);
        assert_eq!(snapshot.virtual_byte_ceiling(), Ok(32_768));
        assert_eq!(
            snapshot.directory.management_request_list_offset,
            LIST_OFFSET + 32
        );
        assert_eq!(
            snapshot.directory.management_response_list_offset,
            LIST_OFFSET + 40
        );
    }

    #[test]
    fn snapshot_rejects_signature_without_nul() {
        let mut bytes = snapshot_fixture();
        bytes[9] = b'x';
        assert_eq!(
            parse_port_snapshot(&bytes),
            Err(ThunderboltError::InvalidPortSignature)
        );
    }

    #[test]
    fn snapshot_rejects_truncated_header_out_of_range_targets_and_lun_table() {
        assert_eq!(
            parse_port_snapshot(&[0; PORT_DIRECTORY_BYTES - 1]),
            Err(ThunderboltError::Truncated)
        );

        let mut capabilities = snapshot_fixture();
        let late_capabilities_offset = 0x2f0;
        put_u32(&mut capabilities, 0x1c, late_capabilities_offset);
        capabilities.truncate(late_capabilities_offset as usize + CAPABILITIES_BYTES - 1);
        assert_eq!(
            parse_port_snapshot(&capabilities),
            Err(ThunderboltError::Truncated)
        );

        let mut luns = snapshot_fixture();
        luns.truncate(LUN_OFFSET + MAX_LUNS * LUN_RECORD_BYTES - 1);
        assert_eq!(parse_port_snapshot(&luns), Err(ThunderboltError::Truncated));
    }

    #[test]
    fn snapshot_rejects_offsets_inside_header_misaligned_and_out_of_range() {
        let mut inside = snapshot_fixture();
        put_u32(&mut inside, 0x18, 0x40);
        assert_eq!(
            parse_port_snapshot(&inside),
            Err(ThunderboltError::OffsetInsideHeader)
        );

        let mut misaligned = snapshot_fixture();
        put_u32(&mut misaligned, 0x30, (LIST_OFFSET + 2) as u32);
        assert_eq!(
            parse_port_snapshot(&misaligned),
            Err(ThunderboltError::OffsetMisaligned)
        );

        let mut outside = snapshot_fixture();
        let outside_offset = outside.len() as u32;
        put_u32(&mut outside, 0x20, outside_offset);
        assert_eq!(
            parse_port_snapshot(&outside),
            Err(ThunderboltError::OffsetOutOfRange)
        );
    }

    #[test]
    fn snapshot_rejects_unsupported_version_and_lun_count() {
        let mut version = snapshot_fixture();
        put_u32(&mut version, CAP_OFFSET, 5);
        assert_eq!(
            parse_port_snapshot(&version),
            Err(ThunderboltError::UnsupportedInterfaceVersion(5))
        );

        let mut count = snapshot_fixture();
        put_u32(&mut count, CAP_OFFSET + 16, 33);
        assert_eq!(
            parse_port_snapshot(&count),
            Err(ThunderboltError::TooManyLogicalUnits(33))
        );
    }

    #[test]
    fn snapshot_skips_disabled_luns_and_rejects_invalid_enabled_luns() {
        let mut disabled = snapshot_fixture();
        let second = LUN_OFFSET + LUN_RECORD_BYTES;
        put_u32(&mut disabled, second + 4, 0);
        put_u64(&mut disabled, second + 8, 0);
        assert_eq!(parse_port_snapshot(&disabled).unwrap().luns.len(), 1);

        let mut zero_size = snapshot_fixture();
        put_u32(&mut zero_size, LUN_OFFSET + 4, 0);
        assert_eq!(
            parse_port_snapshot(&zero_size),
            Err(ThunderboltError::InvalidLogicalUnit { index: 0 })
        );

        let mut zero_count = snapshot_fixture();
        put_u64(&mut zero_count, LUN_OFFSET + 8, 0);
        assert_eq!(
            parse_port_snapshot(&zero_count),
            Err(ThunderboltError::InvalidLogicalUnit { index: 0 })
        );
    }

    #[test]
    fn snapshot_rejects_logical_byte_overflow() {
        let mut bytes = snapshot_fixture();
        put_u32(&mut bytes, LUN_OFFSET + 4, u32::MAX);
        put_u64(&mut bytes, LUN_OFFSET + 8, u64::MAX);
        assert_eq!(
            parse_port_snapshot(&bytes),
            Err(ThunderboltError::ArithmeticOverflow)
        );
    }

    #[test]
    fn request_burst_has_exact_bytes_and_keeps_source_unchanged() {
        let entries = [RequestEntry {
            dma_address: 0x1122_3344_5566_7788,
            request_length: 0xaabb_ccdd,
            transaction_id: 0x0102_0304,
        }];
        let source_before = entries;
        let encoded = encode_request_burst(2, &entries).unwrap();
        assert_eq!(
            encoded,
            vec![
                2, 0, 0, 0, 1, 0, 0, 0, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0xdd, 0xcc,
                0xbb, 0xaa, 4, 3, 2, 1,
            ]
        );
        assert_eq!(entries, source_before);
        assert_eq!(parse_request_burst(&encoded, 2).unwrap().entries, entries);
    }

    #[test]
    fn request_burst_rejects_bad_counts_and_every_truncated_boundary() {
        let entry = RequestEntry {
            dma_address: 0,
            request_length: 1,
            transaction_id: 2,
        };
        assert_eq!(
            encode_request_burst(0, &[entry]),
            Err(ThunderboltError::CountExceedsWindow)
        );
        let encoded = encode_request_burst(1, &[entry]).unwrap();
        for length in 0..encoded.len() {
            assert_eq!(
                parse_request_burst(&encoded[..length], 1),
                Err(ThunderboltError::Truncated)
            );
        }
        let mut bad_window = encoded.clone();
        put_u32(&mut bad_window, 0, 0);
        assert_eq!(
            parse_request_burst(&bad_window, 1),
            Err(ThunderboltError::CountExceedsWindow)
        );
        assert_eq!(
            parse_request_burst(&encoded, 0),
            Err(ThunderboltError::CountExceedsLimit)
        );
    }

    fn v4_completion(
        queue_index: u32,
        transaction_id: u32,
        realized: u64,
        response: u32,
    ) -> Vec<u8> {
        let mut bytes = vec![0_u8; COMPLETION_BURST_HEADER_BYTES + V4_COMPLETION_ENTRY_BYTES];
        put_u32(&mut bytes, 4, 1);
        put_u32(&mut bytes, 8, queue_index);
        put_u32(&mut bytes, 12, transaction_id);
        put_u64(&mut bytes, 16, realized);
        put_u32(&mut bytes, 8 + 0x14, response);
        put_u32(&mut bytes, 8 + 0x18, 0x02);
        bytes
    }

    #[test]
    fn completion_distinguishes_legacy_and_v4_raw_fields() {
        let mut legacy = vec![0_u8; COMPLETION_BURST_HEADER_BYTES + LEGACY_COMPLETION_ENTRY_BYTES];
        put_u32(&mut legacy, 4, 1);
        put_u32(&mut legacy, 8, 7);
        put_u32(&mut legacy, 12, 9);
        assert_eq!(
            parse_completion_burst(&legacy, 3, 1, 1).unwrap()[0].fields,
            CompletionFields::Legacy
        );

        let parsed = parse_completion_burst(&v4_completion(7, 9, 99, 0x12), 4, 1, 1).unwrap();
        assert_eq!(
            parsed[0].fields,
            CompletionFields::V4 {
                realized_bytes: 99,
                srb_response: 0x12,
                scsi_status: 0x02,
            }
        );
    }

    #[test]
    fn completion_rejects_versions_counts_and_every_truncated_boundary() {
        assert_eq!(
            parse_completion_burst(&[], 0, 1, 1),
            Err(ThunderboltError::UnsupportedInterfaceVersion(0))
        );
        let encoded = v4_completion(1, 2, 3, 1);
        for length in 0..encoded.len() {
            assert_eq!(
                parse_completion_burst(&encoded[..length], 4, 1, 1),
                Err(ThunderboltError::Truncated)
            );
        }
        let mut all_ones = encoded.clone();
        put_u32(&mut all_ones, 4, u32::MAX);
        assert_eq!(
            parse_completion_burst(&all_ones, 4, u32::MAX, u32::MAX),
            Err(ThunderboltError::AllOnesCount)
        );
        assert_eq!(
            parse_completion_burst(&encoded, 4, 0, 1),
            Err(ThunderboltError::CountExceedsWindow)
        );
        assert_eq!(
            parse_completion_burst(&encoded, 4, 1, 0),
            Err(ThunderboltError::CountExceedsLimit)
        );
    }

    #[test]
    fn completion_validation_rejects_stale_bad_and_oversized_results() {
        let expected = [ExpectedCompletion {
            queue_index: 7,
            transaction_id: 9,
            maximum_request_bytes: 100,
        }];
        let accepted = parse_completion_burst(&v4_completion(7, 9, 100, 1), 4, 1, 1).unwrap();
        assert_eq!(validate_completions(&accepted, &expected), Ok(()));

        let wrong_index = parse_completion_burst(&v4_completion(8, 9, 1, 1), 4, 1, 1).unwrap();
        assert_eq!(
            validate_completions(&wrong_index, &expected),
            Err(ThunderboltError::UnexpectedCompletion { queue_index: 8 })
        );
        let wrong_tx = parse_completion_burst(&v4_completion(7, 8, 1, 1), 4, 1, 1).unwrap();
        assert_eq!(
            validate_completions(&wrong_tx, &expected),
            Err(ThunderboltError::CompletionTransactionMismatch { queue_index: 7 })
        );
        let oversized = parse_completion_burst(&v4_completion(7, 9, 101, 1), 4, 1, 1).unwrap();
        assert_eq!(
            validate_completions(&oversized, &expected),
            Err(ThunderboltError::CompletionTooLarge { queue_index: 7 })
        );
        let failed = parse_completion_burst(&v4_completion(7, 9, 1, 2), 4, 1, 1).unwrap();
        assert_eq!(
            validate_completions(&failed, &expected),
            Err(ThunderboltError::CompletionStatusFailed {
                queue_index: 7,
                srb_response: 2,
            })
        );
    }
}
