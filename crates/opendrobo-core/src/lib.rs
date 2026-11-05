//! Portable, offline decoder for the OpenDrobo capacity page.
//!
//! The decoder accepts only page `0x7a`, subpage `0x02`, and complete 32- or
//! 56-byte bodies. It reads no more than 60 bytes, ignores the uninterpreted
//! PT extension, and requires explicit scope confirmations before returning
//! capacity data.
//!
//! C ABI status codes are stable: 0 success; 1 byte units; 2 device binding;
//! 3 single-volume scope; 4 future; 5 stale; 6 timestamp overflow; 7
//! truncated; 8 unsupported page; 9 unsupported body; 10 zero protected
//! total; 11 inconsistent protected counts; 12 unprotected total below
//! protected total; 13 null input; 14 null output; 15 unknown flags; 16
//! contained panic.

use core::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub mod beyondraid;
pub mod thunderbolt;

const PAGE_CODE: u8 = 0x7a;
const SUBPAGE_CODE: u8 = 0x02;
const HEADER_BYTES: usize = 4;
const LEGACY_BODY_BYTES: usize = 32;
const EXTENDED_BODY_BYTES: usize = 56;
const MAX_PAGE_BYTES: usize = HEADER_BYTES + EXTENDED_BODY_BYTES;
const MAX_AGE_MS: i64 = 30_000;

const CONFIRM_BYTE_UNITS: u32 = 1;
const CONFIRM_SAME_DEVICE: u32 = 2;
const CONFIRM_SINGLE_VOLUME: u32 = 4;
const KNOWN_CONFIRMATIONS: u32 = CONFIRM_BYTE_UNITS | CONFIRM_SAME_DEVICE | CONFIRM_SINGLE_VOLUME;

/// Explicit caller-supplied checks that establish the scope of capacity data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapacityConfirmation {
    pub byte_units: bool,
    pub same_device: bool,
    pub single_volume: bool,
}

impl CapacityConfirmation {
    fn validate(self) -> Result<(), CapacityError> {
        if !self.byte_units {
            return Err(CapacityError::ByteUnitsUnconfirmed);
        }
        if !self.same_device {
            return Err(CapacityError::DeviceUnconfirmed);
        }
        if !self.single_volume {
            return Err(CapacityError::SingleVolumeUnconfirmed);
        }
        Ok(())
    }
}

/// Capacity accounting copied by value across the C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CapacityRecord {
    pub protected_free_bytes: u64,
    pub protected_used_bytes: u64,
    pub protected_total_bytes: u64,
    pub unprotected_total_bytes: u64,
    pub captured_at_ms: i64,
}

impl CapacityRecord {
    /// Rejects invalid age first, then invalid capacity counts.
    pub fn validate_at(&self, now_ms: i64) -> Result<(), CapacityError> {
        validate_age(self.captured_at_ms, now_ms)?;
        validate_counts(
            self.protected_free_bytes,
            self.protected_used_bytes,
            self.protected_total_bytes,
            self.unprotected_total_bytes,
        )
    }
}

/// All decoder and validation failures, with stable C ABI status codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapacityError {
    ByteUnitsUnconfirmed,
    DeviceUnconfirmed,
    SingleVolumeUnconfirmed,
    FutureObservation,
    StaleObservation,
    TimestampOverflow,
    Truncated,
    UnsupportedPage,
    UnsupportedBodyLength,
    ZeroProtectedTotal,
    InconsistentProtectedCounts,
    UnprotectedTotalBelowProtectedTotal,
    NullInput,
    NullOutput,
    UnknownConfirmationFlags,
    PanicContained,
}

impl CapacityError {
    /// Stable nonzero C ABI error code.
    pub const fn code(self) -> i32 {
        match self {
            Self::ByteUnitsUnconfirmed => 1,
            Self::DeviceUnconfirmed => 2,
            Self::SingleVolumeUnconfirmed => 3,
            Self::FutureObservation => 4,
            Self::StaleObservation => 5,
            Self::TimestampOverflow => 6,
            Self::Truncated => 7,
            Self::UnsupportedPage => 8,
            Self::UnsupportedBodyLength => 9,
            Self::ZeroProtectedTotal => 10,
            Self::InconsistentProtectedCounts => 11,
            Self::UnprotectedTotalBelowProtectedTotal => 12,
            Self::NullInput => 13,
            Self::NullOutput => 14,
            Self::UnknownConfirmationFlags => 15,
            Self::PanicContained => 16,
        }
    }
}

impl fmt::Display for CapacityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ByteUnitsUnconfirmed => "capacity byte units are unconfirmed",
            Self::DeviceUnconfirmed => "capacity record/device binding is unconfirmed",
            Self::SingleVolumeUnconfirmed => "capacity record has ambiguous volume scope",
            Self::FutureObservation => "capacity observation is in the future",
            Self::StaleObservation => "capacity observation is stale",
            Self::TimestampOverflow => "capacity timestamp arithmetic overflowed",
            Self::Truncated => "capacity page is truncated",
            Self::UnsupportedPage => "capacity page or subpage is unsupported",
            Self::UnsupportedBodyLength => "capacity body length is unsupported",
            Self::ZeroProtectedTotal => "protected capacity total is zero",
            Self::InconsistentProtectedCounts => "protected capacity counts are inconsistent",
            Self::UnprotectedTotalBelowProtectedTotal => {
                "unprotected capacity total is below protected capacity total"
            }
            Self::NullInput => "capacity input pointer is null",
            Self::NullOutput => "capacity output pointer is null",
            Self::UnknownConfirmationFlags => "capacity confirmation flags contain unknown bits",
            Self::PanicContained => "capacity decoder panic was contained",
        };
        f.write_str(message)
    }
}

impl std::error::Error for CapacityError {}

/// Decodes one fully declared capacity page.
///
/// Allocation padding after a complete 36- or 60-byte page is accepted. The
/// optional PT extension is deliberately ignored.
pub fn decode_capacity(
    page: &[u8],
    confirmation: CapacityConfirmation,
    captured_at_ms: i64,
    now_ms: i64,
) -> Result<CapacityRecord, CapacityError> {
    confirmation.validate()?;
    validate_age(captured_at_ms, now_ms)?;

    let page = &page[..page.len().min(MAX_PAGE_BYTES)];
    if page.len() < HEADER_BYTES {
        return Err(CapacityError::Truncated);
    }
    if page[0] != PAGE_CODE || page[1] != SUBPAGE_CODE {
        return Err(CapacityError::UnsupportedPage);
    }

    let body_length = u16::from_be_bytes([page[2], page[3]]) as usize;
    if body_length != LEGACY_BODY_BYTES && body_length != EXTENDED_BODY_BYTES {
        return Err(CapacityError::UnsupportedBodyLength);
    }
    let declared_length = HEADER_BYTES + body_length;
    if page.len() < declared_length {
        return Err(CapacityError::Truncated);
    }

    let protected_free_bytes = read_be_u64(page, 4)?;
    let protected_used_bytes = read_be_u64(page, 12)?;
    let protected_total_bytes = read_be_u64(page, 20)?;
    let unprotected_total_bytes = read_be_u64(page, 28)?;

    validate_counts(
        protected_free_bytes,
        protected_used_bytes,
        protected_total_bytes,
        unprotected_total_bytes,
    )?;

    Ok(CapacityRecord {
        protected_free_bytes,
        protected_used_bytes,
        protected_total_bytes,
        unprotected_total_bytes,
        captured_at_ms,
    })
}

fn validate_age(captured_at_ms: i64, now_ms: i64) -> Result<(), CapacityError> {
    let age_ms = now_ms
        .checked_sub(captured_at_ms)
        .ok_or(CapacityError::TimestampOverflow)?;
    if age_ms < 0 {
        return Err(CapacityError::FutureObservation);
    }
    if age_ms > MAX_AGE_MS {
        return Err(CapacityError::StaleObservation);
    }
    Ok(())
}

fn validate_counts(
    protected_free_bytes: u64,
    protected_used_bytes: u64,
    protected_total_bytes: u64,
    unprotected_total_bytes: u64,
) -> Result<(), CapacityError> {
    if protected_total_bytes == 0 {
        return Err(CapacityError::ZeroProtectedTotal);
    }
    if protected_free_bytes > protected_total_bytes
        || protected_used_bytes > protected_total_bytes
        || protected_total_bytes - protected_free_bytes != protected_used_bytes
    {
        return Err(CapacityError::InconsistentProtectedCounts);
    }
    if unprotected_total_bytes < protected_total_bytes {
        return Err(CapacityError::UnprotectedTotalBelowProtectedTotal);
    }
    Ok(())
}

fn read_be_u64(page: &[u8], offset: usize) -> Result<u64, CapacityError> {
    let end = offset.checked_add(8).ok_or(CapacityError::Truncated)?;
    let bytes = page.get(offset..end).ok_or(CapacityError::Truncated)?;
    Ok(u64::from_be_bytes(
        bytes.try_into().expect("u64 slice has eight bytes"),
    ))
}

fn confirmation_from_flags(flags: u32) -> Result<CapacityConfirmation, CapacityError> {
    if flags & !KNOWN_CONFIRMATIONS != 0 {
        return Err(CapacityError::UnknownConfirmationFlags);
    }
    Ok(CapacityConfirmation {
        byte_units: flags & CONFIRM_BYTE_UNITS != 0,
        same_device: flags & CONFIRM_SAME_DEVICE != 0,
        single_volume: flags & CONFIRM_SINGLE_VOLUME != 0,
    })
}

/// C ABI decoder. `page` and `output` must be non-null; the decoder copies no
/// more than `min(length, 60)` page bytes and retains neither pointer.
///
/// `page` may overlap `output`: input bytes are copied before output is
/// cleared. On every decode error with a valid output pointer, `output` is
/// zeroed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn od_capacity_decode(
    page: *const u8,
    length: usize,
    confirmations: u32,
    captured_at_ms: i64,
    now_ms: i64,
    output: *mut CapacityRecord,
) -> i32 {
    if output.is_null() {
        return CapacityError::NullOutput.code();
    }
    if page.is_null() {
        // Safety: the C ABI contract requires a valid, writable output record.
        unsafe { output.write(CapacityRecord::default()) };
        return CapacityError::NullInput.code();
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let bounded_length = length.min(MAX_PAGE_BYTES);
        let mut copied_page = [0_u8; MAX_PAGE_BYTES];
        // Safety: the C ABI contract requires `page` valid for bounded_length bytes.
        let source = unsafe { std::slice::from_raw_parts(page, bounded_length) };
        copied_page[..bounded_length].copy_from_slice(source);
        // Safety: checked non-null above; contract requires a writable record.
        unsafe { output.write(CapacityRecord::default()) };
        let confirmation = confirmation_from_flags(confirmations)?;
        decode_capacity(
            &copied_page[..bounded_length],
            confirmation,
            captured_at_ms,
            now_ms,
        )
    }));

    match result {
        Ok(Ok(record)) => {
            // Safety: checked non-null above; contract requires a writable record.
            unsafe { output.write(record) };
            0
        }
        Ok(Err(error)) => error.code(),
        Err(_) => {
            // Safety: checked non-null above; contract requires a writable record.
            unsafe { output.write(CapacityRecord::default()) };
            CapacityError::PanicContained.code()
        }
    }
}

/// C ABI freshness validator. `capacity` must be non-null and valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn od_capacity_validate(capacity: *const CapacityRecord, now_ms: i64) -> i32 {
    if capacity.is_null() {
        return CapacityError::NullInput.code();
    }
    match catch_unwind(AssertUnwindSafe(|| {
        // Safety: the C ABI contract requires a valid capacity pointer.
        unsafe { (*capacity).validate_at(now_ms) }
    })) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => error.code(),
        Err(_) => CapacityError::PanicContained.code(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, offset_of, size_of};

    const CONFIRMED: CapacityConfirmation = CapacityConfirmation {
        byte_units: true,
        same_device: true,
        single_volume: true,
    };
    const CAPTURED_AT: i64 = 10_000;
    const FRESH_NOW: i64 = CAPTURED_AT + MAX_AGE_MS;

    fn page(
        body_length: usize,
        page_code: u8,
        subpage_code: u8,
        free: u64,
        used: u64,
        total: u64,
        unprotected: u64,
    ) -> Vec<u8> {
        let mut result = vec![
            page_code,
            subpage_code,
            (body_length >> 8) as u8,
            body_length as u8,
        ];
        for value in [free, used, total, unprotected] {
            result.extend(value.to_be_bytes());
        }
        if body_length == EXTENDED_BODY_BYTES {
            result.extend([0xa5; 24]);
        }
        result
    }

    fn valid_page() -> Vec<u8> {
        page(
            LEGACY_BODY_BYTES,
            PAGE_CODE,
            SUBPAGE_CODE,
            7_000_000_000_000,
            5_000_000_000_000,
            12_000_000_000_000,
            12_500_000_000_000,
        )
    }

    fn rejects(page: &[u8]) -> CapacityError {
        decode_capacity(page, CONFIRMED, CAPTURED_AT, FRESH_NOW).unwrap_err()
    }

    #[test]
    fn decodes_actual_protected_capacity_not_logical_ceiling() {
        let record = decode_capacity(&valid_page(), CONFIRMED, CAPTURED_AT, FRESH_NOW).unwrap();
        assert_eq!(record.protected_total_bytes, 12_000_000_000_000);
        assert_ne!(record.protected_total_bytes, 64_000_000_000_000);
        assert_eq!(record.protected_used_bytes, 5_000_000_000_000);
        assert_eq!(record.protected_free_bytes, 7_000_000_000_000);
    }

    #[test]
    fn every_legacy_header_and_body_truncation_rejects() {
        let valid = valid_page();
        for length in 0..MAX_PAGE_BYTES - EXTENDED_BODY_BYTES + LEGACY_BODY_BYTES {
            assert_eq!(
                rejects(&valid[..length]),
                CapacityError::Truncated,
                "length {length}"
            );
        }
    }

    #[test]
    fn every_extended_body_truncation_rejects() {
        let valid = page(EXTENDED_BODY_BYTES, PAGE_CODE, SUBPAGE_CODE, 7, 5, 12, 12);
        for length in 0..MAX_PAGE_BYTES {
            assert_eq!(
                rejects(&valid[..length]),
                CapacityError::Truncated,
                "length {length}"
            );
        }
    }

    #[test]
    fn extended_page_and_padding_accept() {
        let mut extended = page(EXTENDED_BODY_BYTES, PAGE_CODE, SUBPAGE_CODE, 7, 5, 12, 12);
        extended.extend([0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(
            decode_capacity(&extended, CONFIRMED, CAPTURED_AT, FRESH_NOW)
                .unwrap()
                .protected_total_bytes,
            12
        );
    }

    #[test]
    fn wrong_page_fields_and_body_lengths_reject() {
        assert_eq!(
            rejects(&page(LEGACY_BODY_BYTES, 0x79, SUBPAGE_CODE, 7, 5, 12, 12)),
            CapacityError::UnsupportedPage
        );
        assert_eq!(
            rejects(&page(LEGACY_BODY_BYTES, PAGE_CODE, 0x03, 7, 5, 12, 12)),
            CapacityError::UnsupportedPage
        );
        assert_eq!(
            rejects(&page(28, PAGE_CODE, SUBPAGE_CODE, 7, 5, 12, 12)),
            CapacityError::UnsupportedBodyLength
        );
        assert_eq!(
            rejects(&page(60, PAGE_CODE, SUBPAGE_CODE, 7, 5, 12, 12)),
            CapacityError::UnsupportedBodyLength
        );
    }

    #[test]
    fn big_endian_and_u64_edge_values_are_correct() {
        let record = decode_capacity(
            &page(
                LEGACY_BODY_BYTES,
                PAGE_CODE,
                SUBPAGE_CODE,
                u64::MAX,
                0,
                u64::MAX,
                u64::MAX,
            ),
            CONFIRMED,
            CAPTURED_AT,
            FRESH_NOW,
        )
        .unwrap();
        assert_eq!(record.protected_free_bytes, u64::MAX);
        assert_eq!(record.protected_used_bytes, 0);
        assert_eq!(
            rejects(&page(
                LEGACY_BODY_BYTES,
                PAGE_CODE,
                SUBPAGE_CODE,
                u64::MAX,
                1,
                u64::MAX,
                u64::MAX
            )),
            CapacityError::InconsistentProtectedCounts
        );
    }

    #[test]
    fn protected_and_unprotected_invariants_reject() {
        assert_eq!(
            rejects(&page(
                LEGACY_BODY_BYTES,
                PAGE_CODE,
                SUBPAGE_CODE,
                0,
                0,
                0,
                0
            )),
            CapacityError::ZeroProtectedTotal
        );
        assert_eq!(
            rejects(&page(
                LEGACY_BODY_BYTES,
                PAGE_CODE,
                SUBPAGE_CODE,
                8,
                5,
                12,
                12
            )),
            CapacityError::InconsistentProtectedCounts
        );
        assert_eq!(
            rejects(&page(
                LEGACY_BODY_BYTES,
                PAGE_CODE,
                SUBPAGE_CODE,
                13,
                0,
                12,
                12
            )),
            CapacityError::InconsistentProtectedCounts
        );
        assert_eq!(
            rejects(&page(
                LEGACY_BODY_BYTES,
                PAGE_CODE,
                SUBPAGE_CODE,
                7,
                5,
                12,
                11
            )),
            CapacityError::UnprotectedTotalBelowProtectedTotal
        );
    }

    #[test]
    fn confirmation_failures_are_specific() {
        assert_eq!(
            decode_capacity(
                &valid_page(),
                CapacityConfirmation {
                    byte_units: false,
                    ..CONFIRMED
                },
                CAPTURED_AT,
                FRESH_NOW
            ),
            Err(CapacityError::ByteUnitsUnconfirmed)
        );
        assert_eq!(
            decode_capacity(
                &valid_page(),
                CapacityConfirmation {
                    same_device: false,
                    ..CONFIRMED
                },
                CAPTURED_AT,
                FRESH_NOW
            ),
            Err(CapacityError::DeviceUnconfirmed)
        );
        assert_eq!(
            decode_capacity(
                &valid_page(),
                CapacityConfirmation {
                    single_volume: false,
                    ..CONFIRMED
                },
                CAPTURED_AT,
                FRESH_NOW
            ),
            Err(CapacityError::SingleVolumeUnconfirmed)
        );
    }

    #[test]
    fn parse_and_later_validation_enforce_timestamp_bounds_and_overflow() {
        assert_eq!(
            decode_capacity(&valid_page(), CONFIRMED, CAPTURED_AT, CAPTURED_AT - 1),
            Err(CapacityError::FutureObservation)
        );
        assert_eq!(
            decode_capacity(
                &valid_page(),
                CONFIRMED,
                CAPTURED_AT,
                CAPTURED_AT + MAX_AGE_MS + 1
            ),
            Err(CapacityError::StaleObservation)
        );
        assert_eq!(
            decode_capacity(&valid_page(), CONFIRMED, i64::MIN, i64::MAX),
            Err(CapacityError::TimestampOverflow)
        );
        let record = decode_capacity(&valid_page(), CONFIRMED, CAPTURED_AT, FRESH_NOW).unwrap();
        assert_eq!(
            record.validate_at(CAPTURED_AT - 1),
            Err(CapacityError::FutureObservation)
        );
        assert_eq!(
            record.validate_at(CAPTURED_AT + MAX_AGE_MS + 1),
            Err(CapacityError::StaleObservation)
        );
        assert_eq!(
            record.validate_at(i64::MAX),
            Err(CapacityError::StaleObservation)
        );
        let overflow_record = CapacityRecord {
            captured_at_ms: i64::MIN,
            ..record
        };
        assert_eq!(
            overflow_record.validate_at(i64::MAX),
            Err(CapacityError::TimestampOverflow)
        );
    }

    #[test]
    fn slice_and_input_over_maximum_are_bounded() {
        let valid = valid_page();
        let mut prefixed = vec![0xff];
        prefixed.extend(&valid);
        let sliced = &prefixed[1..];
        assert_eq!(
            decode_capacity(sliced, CONFIRMED, CAPTURED_AT, FRESH_NOW)
                .unwrap()
                .protected_total_bytes,
            12_000_000_000_000
        );
        let mut padded = valid;
        padded.extend(std::iter::repeat_n(0xee, 100));
        assert!(decode_capacity(&padded, CONFIRMED, CAPTURED_AT, FRESH_NOW).is_ok());
    }

    #[test]
    fn error_codes_and_display_are_stable() {
        assert_eq!(CapacityError::StaleObservation.code(), 5);
        assert_eq!(CapacityError::UnknownConfirmationFlags.code(), 15);
        assert_eq!(CapacityError::PanicContained.code(), 16);
        assert_eq!(
            CapacityError::Truncated.to_string(),
            "capacity page is truncated"
        );
    }

    #[test]
    fn c_abi_rejects_nulls_and_clears_output_on_error() {
        let mut output = CapacityRecord {
            protected_free_bytes: 1,
            protected_used_bytes: 2,
            protected_total_bytes: 3,
            unprotected_total_bytes: 4,
            captured_at_ms: 5,
        };
        let page = valid_page();
        assert_eq!(
            unsafe {
                od_capacity_decode(
                    core::ptr::null(),
                    page.len(),
                    7,
                    CAPTURED_AT,
                    FRESH_NOW,
                    &mut output,
                )
            },
            CapacityError::NullInput.code()
        );
        assert_eq!(output, CapacityRecord::default());
        assert_eq!(
            unsafe {
                od_capacity_decode(
                    page.as_ptr(),
                    page.len(),
                    7 | 8,
                    CAPTURED_AT,
                    FRESH_NOW,
                    &mut output,
                )
            },
            CapacityError::UnknownConfirmationFlags.code()
        );
        assert_eq!(output, CapacityRecord::default());
        assert_eq!(
            unsafe {
                od_capacity_decode(
                    page.as_ptr(),
                    page.len(),
                    7,
                    CAPTURED_AT,
                    FRESH_NOW,
                    core::ptr::null_mut(),
                )
            },
            CapacityError::NullOutput.code()
        );
        assert_eq!(
            unsafe { od_capacity_validate(core::ptr::null(), FRESH_NOW) },
            CapacityError::NullInput.code()
        );
    }

    #[test]
    fn c_abi_decodes_and_validates() {
        let page = valid_page();
        let mut output = CapacityRecord::default();
        assert_eq!(
            unsafe {
                od_capacity_decode(
                    page.as_ptr(),
                    page.len(),
                    7,
                    CAPTURED_AT,
                    FRESH_NOW,
                    &mut output,
                )
            },
            0
        );
        assert_eq!(output.protected_total_bytes, 12_000_000_000_000);
        assert_eq!(unsafe { od_capacity_validate(&output, FRESH_NOW) }, 0);
        assert_eq!(
            unsafe { od_capacity_validate(&output, FRESH_NOW + 1) },
            CapacityError::StaleObservation.code()
        );
    }

    #[test]
    fn c_abi_accepts_overlapping_aligned_input_and_output() {
        let page = valid_page();
        assert_eq!(page.len(), 36);
        let mut storage = [0_u64; 8];
        let base = storage.as_mut_ptr().cast::<u8>();
        let output = unsafe { base.add(8).cast::<CapacityRecord>() };

        unsafe { std::ptr::copy_nonoverlapping(page.as_ptr(), base, page.len()) };
        assert_eq!(
            unsafe { od_capacity_decode(base, page.len(), 7, CAPTURED_AT, FRESH_NOW, output) },
            0
        );
        assert_eq!(
            unsafe { output.read() }.protected_total_bytes,
            12_000_000_000_000
        );

        unsafe { std::ptr::copy_nonoverlapping(page.as_ptr(), base, page.len()) };
        unsafe { base.write(0x79) };
        assert_eq!(
            unsafe { od_capacity_decode(base, page.len(), 7, CAPTURED_AT, FRESH_NOW, output) },
            CapacityError::UnsupportedPage.code()
        );
        assert_eq!(unsafe { output.read() }, CapacityRecord::default());
    }

    #[test]
    fn validation_checks_age_then_all_numeric_invariants() {
        let zeroed = CapacityRecord::default();
        assert_eq!(
            zeroed.validate_at(0),
            Err(CapacityError::ZeroProtectedTotal)
        );
        assert_eq!(
            unsafe { od_capacity_validate(&zeroed, 0) },
            CapacityError::ZeroProtectedTotal.code()
        );

        let valid = decode_capacity(&valid_page(), CONFIRMED, CAPTURED_AT, FRESH_NOW).unwrap();
        let inconsistent = CapacityRecord {
            protected_free_bytes: 8,
            protected_used_bytes: 5,
            protected_total_bytes: 12,
            unprotected_total_bytes: 12,
            captured_at_ms: CAPTURED_AT,
        };
        assert_eq!(
            inconsistent.validate_at(CAPTURED_AT),
            Err(CapacityError::InconsistentProtectedCounts)
        );
        let below_protected = CapacityRecord {
            unprotected_total_bytes: valid.protected_total_bytes - 1,
            ..valid
        };
        assert_eq!(
            below_protected.validate_at(FRESH_NOW),
            Err(CapacityError::UnprotectedTotalBelowProtectedTotal)
        );
        let overflow_zeroed = CapacityRecord {
            captured_at_ms: i64::MIN,
            ..zeroed
        };
        assert_eq!(
            overflow_zeroed.validate_at(i64::MAX),
            Err(CapacityError::TimestampOverflow)
        );
    }

    #[test]
    fn c_abi_layout_matches_header_fields() {
        assert_eq!(size_of::<CapacityRecord>(), 40);
        assert_eq!(align_of::<CapacityRecord>(), align_of::<u64>());
        assert_eq!(offset_of!(CapacityRecord, protected_free_bytes), 0);
        assert_eq!(offset_of!(CapacityRecord, protected_used_bytes), 8);
        assert_eq!(offset_of!(CapacityRecord, protected_total_bytes), 16);
        assert_eq!(offset_of!(CapacityRecord, unprotected_total_bytes), 24);
        assert_eq!(offset_of!(CapacityRecord, captured_at_ms), 32);
    }
}
