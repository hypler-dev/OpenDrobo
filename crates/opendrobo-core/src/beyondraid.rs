//! Read-only structural inspection for a recovered RMR header shape.
//!
//! This module recognizes only a candidate header and verifies its recovered
//! checksum rule. It does not identify a disk, validate an array, reconstruct
//! BeyondRAID data, or establish that caller words identify any particular
//! physical or logical object. Firmware validation includes additional checks
//! that are intentionally not modeled here.

use core::fmt;

/// The recovered RMR header and checksum window length.
pub const RMR_WINDOW_BYTES: usize = 0x2000;

const MAGIC_WORD_0: u32 = 0x726d_7268;
const MAGIC_WORD_4: u32 = 0x4246_432d;
const REVISION_OFFSET: usize = 0x08;
const DECLARED_LENGTH_OFFSET: usize = 0x0c;
const STORED_CRC_OFFSET: usize = 0x10;
const CALLER_WORD_14_OFFSET: usize = 0x14;
const CALLER_WORD_18_OFFSET: usize = 0x18;
const RAW_WORD_1C_OFFSET: usize = 0x1c;
const GEOMETRY_SHIFT_OFFSET: usize = 0x20;
const PACK_ID_OFFSET: usize = 0x88;
const PACK_ID_BYTES: usize = 8;

/// A structurally recognized RMR header with a matching checksum.
///
/// This is a candidate only. It is not a valid-disk or valid-array result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RmrHeaderCandidate {
    pub revision: u32,
    pub caller_word_14: u32,
    pub caller_word_18: u32,
    pub raw_word_1c: u32,
    pub geometry_shift: u32,
    pub pack_id: [u8; PACK_ID_BYTES],
}

impl RmrHeaderCandidate {
    /// Checks caller-supplied identity material against this candidate.
    ///
    /// The comparison follows recovered validator branches, but cannot prove
    /// that these values bind the header to a particular disk pack or array.
    /// Revision two requires an explicit expected pack identifier. Revision
    /// one checks only the two caller words because its pack field is required
    /// to be zero during candidate parsing.
    pub fn check_identity(
        &self,
        expected_word_14: u32,
        expected_word_18: u32,
        expected_pack_id: Option<[u8; PACK_ID_BYTES]>,
    ) -> Result<(), RmrError> {
        if self.caller_word_14 != expected_word_14 {
            return Err(RmrError::CallerWord14Mismatch);
        }
        if self.caller_word_18 != expected_word_18 {
            return Err(RmrError::CallerWord18Mismatch);
        }
        if self.revision == 2 {
            let expected_pack_id = expected_pack_id.ok_or(RmrError::RevisionTwoPackIdRequired)?;
            if self.pack_id != expected_pack_id {
                return Err(RmrError::RevisionTwoPackIdMismatch);
            }
        }
        Ok(())
    }
}

/// Failures from bounded RMR candidate inspection or identity comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RmrError {
    Truncated,
    UnsupportedMagic,
    UnsupportedRevision,
    UnsupportedDeclaredLength,
    ChecksumMismatch,
    CallerWord18Unaligned,
    UnsupportedGeometryShift,
    RevisionOneState,
    RevisionOnePackId,
    CallerWord14Mismatch,
    CallerWord18Mismatch,
    RevisionTwoPackIdRequired,
    RevisionTwoPackIdMismatch,
}

impl fmt::Display for RmrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Truncated => "RMR candidate is shorter than the 8192-byte checksum window",
            Self::UnsupportedMagic => "RMR candidate magic is unsupported",
            Self::UnsupportedRevision => "RMR candidate revision is unsupported",
            Self::UnsupportedDeclaredLength => "RMR candidate declared length is unsupported",
            Self::ChecksumMismatch => "RMR candidate checksum does not match",
            Self::CallerWord18Unaligned => {
                "RMR candidate caller word at 0x18 is not aligned to 512 bytes"
            }
            Self::UnsupportedGeometryShift => "RMR candidate geometry shift is unsupported",
            Self::RevisionOneState => "revision-one RMR candidate has an unsupported state word",
            Self::RevisionOnePackId => "revision-one RMR candidate has a nonzero pack identifier",
            Self::CallerWord14Mismatch => "RMR candidate caller word at 0x14 does not match",
            Self::CallerWord18Mismatch => "RMR candidate caller word at 0x18 does not match",
            Self::RevisionTwoPackIdRequired => {
                "revision-two RMR candidate requires an expected pack identifier"
            }
            Self::RevisionTwoPackIdMismatch => {
                "revision-two RMR candidate pack identifier does not match"
            }
        };
        f.write_str(message)
    }
}

impl std::error::Error for RmrError {}

/// Computes the recovered reflected CRC using a zero initial value and no
/// final XOR. This function checks every byte supplied by the caller.
pub fn rmr_crc(bytes: &[u8]) -> u32 {
    crc_update(0, bytes)
}

/// Inspects the first 8192 bytes as a recovered RMR header candidate.
///
/// Allocation padding after the 8192-byte window is accepted and ignored. The
/// stored checksum field is virtually zeroed during calculation; input bytes
/// are never modified.
pub fn parse_rmr_candidate(bytes: &[u8]) -> Result<RmrHeaderCandidate, RmrError> {
    let window = bytes.get(..RMR_WINDOW_BYTES).ok_or(RmrError::Truncated)?;

    if read_le_u32(window, 0)? != MAGIC_WORD_0 || read_le_u32(window, 4)? != MAGIC_WORD_4 {
        return Err(RmrError::UnsupportedMagic);
    }

    let revision = read_le_u32(window, REVISION_OFFSET)?;
    if revision != 1 && revision != 2 {
        return Err(RmrError::UnsupportedRevision);
    }
    if read_le_u32(window, DECLARED_LENGTH_OFFSET)? != RMR_WINDOW_BYTES as u32 {
        return Err(RmrError::UnsupportedDeclaredLength);
    }

    let stored_crc = read_le_u32(window, STORED_CRC_OFFSET)?;
    if rmr_window_crc(window) != stored_crc {
        return Err(RmrError::ChecksumMismatch);
    }

    let caller_word_18 = read_le_u32(window, CALLER_WORD_18_OFFSET)?;
    if caller_word_18 & 0x1ff != 0 {
        return Err(RmrError::CallerWord18Unaligned);
    }

    let geometry_shift = read_le_u32(window, GEOMETRY_SHIFT_OFFSET)?;
    if !(12..=14).contains(&geometry_shift) {
        return Err(RmrError::UnsupportedGeometryShift);
    }

    let pack_id = read_pack_id(window)?;
    let raw_word_1c = read_le_u32(window, RAW_WORD_1C_OFFSET)?;
    if revision == 1 {
        if raw_word_1c != 1 {
            return Err(RmrError::RevisionOneState);
        }
        if pack_id != [0; PACK_ID_BYTES] {
            return Err(RmrError::RevisionOnePackId);
        }
    }

    Ok(RmrHeaderCandidate {
        revision,
        caller_word_14: read_le_u32(window, CALLER_WORD_14_OFFSET)?,
        caller_word_18,
        raw_word_1c,
        geometry_shift,
        pack_id,
    })
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Result<u32, RmrError> {
    let end = offset.checked_add(4).ok_or(RmrError::Truncated)?;
    let bytes = bytes.get(offset..end).ok_or(RmrError::Truncated)?;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("u32 slice has four bytes"),
    ))
}

fn read_pack_id(window: &[u8]) -> Result<[u8; PACK_ID_BYTES], RmrError> {
    let end = PACK_ID_OFFSET
        .checked_add(PACK_ID_BYTES)
        .ok_or(RmrError::Truncated)?;
    let bytes = window.get(PACK_ID_OFFSET..end).ok_or(RmrError::Truncated)?;
    Ok(bytes
        .try_into()
        .expect("pack identifier slice has eight bytes"))
}

fn rmr_window_crc(window: &[u8]) -> u32 {
    debug_assert_eq!(window.len(), RMR_WINDOW_BYTES);
    let before_stored_crc = &window[..STORED_CRC_OFFSET];
    let after_stored_crc = &window[STORED_CRC_OFFSET + 4..];
    let crc = crc_update(0, before_stored_crc);
    let crc = crc_update(crc, &[0; 4]);
    crc_update(crc, after_stored_crc)
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

#[cfg(test)]
mod tests {
    use super::*;

    const CALLER_14: u32 = 0x1122_3344;
    const CALLER_18: u32 = 0x5566_7800;
    const PACK_ID: [u8; PACK_ID_BYTES] = *b"testpack";

    fn synthetic_candidate(revision: u32) -> [u8; RMR_WINDOW_BYTES] {
        let mut bytes = [0u8; RMR_WINDOW_BYTES];
        put_le_u32(&mut bytes, 0, MAGIC_WORD_0);
        put_le_u32(&mut bytes, 4, MAGIC_WORD_4);
        put_le_u32(&mut bytes, REVISION_OFFSET, revision);
        put_le_u32(&mut bytes, DECLARED_LENGTH_OFFSET, RMR_WINDOW_BYTES as u32);
        put_le_u32(&mut bytes, CALLER_WORD_14_OFFSET, CALLER_14);
        put_le_u32(&mut bytes, CALLER_WORD_18_OFFSET, CALLER_18);
        put_le_u32(&mut bytes, GEOMETRY_SHIFT_OFFSET, 12);
        if revision == 1 {
            put_le_u32(&mut bytes, RAW_WORD_1C_OFFSET, 1);
        } else {
            bytes[PACK_ID_OFFSET..PACK_ID_OFFSET + PACK_ID_BYTES].copy_from_slice(&PACK_ID);
        }
        let crc = rmr_window_crc(&bytes);
        put_le_u32(&mut bytes, STORED_CRC_OFFSET, crc);
        bytes
    }

    fn put_le_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn refresh_crc(bytes: &mut [u8; RMR_WINDOW_BYTES]) {
        let crc = rmr_window_crc(bytes);
        put_le_u32(bytes, STORED_CRC_OFFSET, crc);
    }

    #[test]
    fn recovered_crc_known_vector_uses_zero_seed_and_no_final_xor() {
        assert_eq!(rmr_crc(b"123456789"), 0x2dfd_2d88);
    }

    #[test]
    fn lookup_table_crc_matches_the_bitwise_reference() {
        let synthetic = synthetic_candidate(2);
        for bytes in [b"123456789".as_slice(), &synthetic[..]] {
            assert_eq!(table_crc(bytes), rmr_crc(bytes));
        }
    }

    #[test]
    fn parses_synthetic_revision_one_candidate() {
        let bytes = synthetic_candidate(1);
        assert_eq!(
            parse_rmr_candidate(&bytes),
            Ok(RmrHeaderCandidate {
                revision: 1,
                caller_word_14: CALLER_14,
                caller_word_18: CALLER_18,
                raw_word_1c: 1,
                geometry_shift: 12,
                pack_id: [0; PACK_ID_BYTES],
            })
        );
    }

    #[test]
    fn parses_synthetic_revision_two_candidate() {
        let bytes = synthetic_candidate(2);
        let candidate = parse_rmr_candidate(&bytes).expect("synthetic candidate parses");
        assert_eq!(candidate.revision, 2);
        assert_eq!(candidate.pack_id, PACK_ID);
    }

    #[test]
    fn every_shorter_input_is_truncated_without_panicking() {
        let bytes = synthetic_candidate(1);
        for length in 0..RMR_WINDOW_BYTES {
            assert_eq!(
                parse_rmr_candidate(&bytes[..length]),
                Err(RmrError::Truncated)
            );
        }
    }

    #[test]
    fn accepts_padding_and_only_reads_the_checksum_window() {
        let bytes = synthetic_candidate(1);
        let expected = parse_rmr_candidate(&bytes).expect("synthetic candidate parses");
        let mut padded = bytes.to_vec();
        padded.extend_from_slice(&[0xff; 64]);
        assert_eq!(parse_rmr_candidate(&padded), Ok(expected));
    }

    #[test]
    fn parsing_does_not_mutate_the_source() {
        let bytes = synthetic_candidate(2);
        let before = bytes;
        let _ = parse_rmr_candidate(&bytes).expect("synthetic candidate parses");
        assert_eq!(bytes, before);
    }

    #[test]
    fn checksum_detects_flips_in_each_candidate_region() {
        let bytes = synthetic_candidate(2);
        for offset in [
            0,
            4,
            REVISION_OFFSET,
            DECLARED_LENGTH_OFFSET,
            STORED_CRC_OFFSET,
            CALLER_WORD_14_OFFSET,
            CALLER_WORD_18_OFFSET,
            RAW_WORD_1C_OFFSET,
            GEOMETRY_SHIFT_OFFSET,
            PACK_ID_OFFSET,
            0x200,
            RMR_WINDOW_BYTES - 1,
        ] {
            let mut changed = bytes;
            changed[offset] ^= 0x80;
            assert!(parse_rmr_candidate(&changed).is_err(), "offset {offset:#x}");
        }
    }

    #[test]
    fn rejects_bad_magic_revision_declared_length_and_geometry_shift() {
        let mut bytes = synthetic_candidate(1);
        put_le_u32(&mut bytes, 0, 0);
        refresh_crc(&mut bytes);
        assert_eq!(parse_rmr_candidate(&bytes), Err(RmrError::UnsupportedMagic));

        let mut bytes = synthetic_candidate(1);
        put_le_u32(&mut bytes, REVISION_OFFSET, 3);
        refresh_crc(&mut bytes);
        assert_eq!(
            parse_rmr_candidate(&bytes),
            Err(RmrError::UnsupportedRevision)
        );

        let mut bytes = synthetic_candidate(1);
        put_le_u32(&mut bytes, DECLARED_LENGTH_OFFSET, 0);
        refresh_crc(&mut bytes);
        assert_eq!(
            parse_rmr_candidate(&bytes),
            Err(RmrError::UnsupportedDeclaredLength)
        );

        let mut bytes = synthetic_candidate(1);
        put_le_u32(&mut bytes, GEOMETRY_SHIFT_OFFSET, 11);
        refresh_crc(&mut bytes);
        assert_eq!(
            parse_rmr_candidate(&bytes),
            Err(RmrError::UnsupportedGeometryShift)
        );
    }

    #[test]
    fn rejects_unaligned_caller_word_18_after_checksum_verification() {
        for low_bit in [1, 0x100] {
            let mut bytes = synthetic_candidate(1);
            put_le_u32(&mut bytes, CALLER_WORD_18_OFFSET, CALLER_18 | low_bit);
            refresh_crc(&mut bytes);
            assert_eq!(
                parse_rmr_candidate(&bytes),
                Err(RmrError::CallerWord18Unaligned)
            );
        }
    }

    #[test]
    fn rejects_revision_one_state_and_pack_identifier() {
        let mut bytes = synthetic_candidate(1);
        put_le_u32(&mut bytes, RAW_WORD_1C_OFFSET, 2);
        refresh_crc(&mut bytes);
        assert_eq!(parse_rmr_candidate(&bytes), Err(RmrError::RevisionOneState));

        let mut bytes = synthetic_candidate(1);
        bytes[PACK_ID_OFFSET] = 1;
        refresh_crc(&mut bytes);
        assert_eq!(
            parse_rmr_candidate(&bytes),
            Err(RmrError::RevisionOnePackId)
        );
    }

    #[test]
    fn identity_checks_caller_words_and_revision_two_pack_identifier() {
        let revision_one = parse_rmr_candidate(&synthetic_candidate(1)).unwrap();
        assert_eq!(
            revision_one.check_identity(CALLER_14, CALLER_18, None),
            Ok(())
        );
        assert_eq!(
            revision_one.check_identity(CALLER_14.wrapping_add(1), CALLER_18, None),
            Err(RmrError::CallerWord14Mismatch)
        );
        assert_eq!(
            revision_one.check_identity(CALLER_14, CALLER_18.wrapping_add(1), Some(PACK_ID)),
            Err(RmrError::CallerWord18Mismatch)
        );

        let revision_two = parse_rmr_candidate(&synthetic_candidate(2)).unwrap();
        assert_eq!(
            revision_two.check_identity(CALLER_14, CALLER_18, None),
            Err(RmrError::RevisionTwoPackIdRequired)
        );
        assert_eq!(
            revision_two.check_identity(CALLER_14, CALLER_18, Some(*b"wrong-id")),
            Err(RmrError::RevisionTwoPackIdMismatch)
        );
        assert_eq!(
            revision_two.check_identity(CALLER_14, CALLER_18, Some(PACK_ID)),
            Ok(())
        );
    }

    const fn build_crc_table() -> [u32; 256] {
        let mut table = [0; 256];
        let mut index = 0;
        while index < table.len() {
            let mut crc = index as u32;
            let mut bit = 0;
            while bit < 8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
                bit += 1;
            }
            table[index] = crc;
            index += 1;
        }
        table
    }

    const CRC_TABLE: [u32; 256] = build_crc_table();

    fn table_crc(bytes: &[u8]) -> u32 {
        let mut crc = 0;
        for &byte in bytes {
            let index = ((crc ^ u32::from(byte)) & 0xff) as usize;
            crc = (crc >> 8) ^ CRC_TABLE[index];
        }
        crc
    }
}
