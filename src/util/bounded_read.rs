//! Reading length-prefixed data without trusting the length.
//!
//! Every wire format in this crate is length-prefixed with a `var_int` that the
//! sender controls. Sizing an allocation from that value before the bytes have
//! arrived lets a tiny message demand an enormous allocation:
//!
//! ```text
//! let len = var_int::read(reader)?;
//! let mut buf = vec![0; len as usize];   // allocate first
//! reader.read_exact(&mut buf)?;          // find out it was a lie second
//! ```
//!
//! A 27-byte transaction declaring a 2^48-byte locking script produces
//! `memory allocation of 281474976710656 bytes failed`. **Allocation failure in
//! Rust aborts rather than panicking** — it does not unwind, so `catch_unwind`
//! and any panic hook the consuming application installed are bypassed and the
//! process dies. There is nothing a consumer can do about it from the outside.
//!
//! The helpers here make the allocation track the bytes that actually arrive.

use std::io::Read;

use crate::util::ChainGangError;

/// Upper bound on a speculative `Vec::with_capacity` for an element count.
///
/// Element counts are not read into a buffer, so they cannot use
/// [`read_exact_vec`]; the loop that follows them reads from the reader and
/// fails when the data runs out. Reserving up to this many entries keeps the
/// fast path for realistic messages while making a hostile count cost nothing.
pub const MAX_PREALLOC_ELEMENTS: usize = 1024;

/// Read exactly `len` bytes, growing the buffer as they arrive.
///
/// `Read::take` bounds the read and `read_to_end` grows geometrically from
/// empty, so a declared length that the reader cannot satisfy costs only the
/// bytes that were really there. A short read is an error rather than a
/// silently truncated value.
pub fn read_exact_vec(
    reader: &mut dyn Read,
    len: u64,
    field: &str,
) -> Result<Vec<u8>, ChainGangError> {
    let mut buf = Vec::new();
    let read = reader.take(len).read_to_end(&mut buf)?;
    if read as u64 != len {
        return Err(ChainGangError::BadData(format!(
            "{field} declares {len} bytes but only {read} were available"
        )));
    }
    Ok(buf)
}

/// Capacity to reserve for a declared element count, bounded by
/// [`MAX_PREALLOC_ELEMENTS`].
pub fn bounded_capacity(declared: u64) -> usize {
    usize::try_from(declared)
        .unwrap_or(MAX_PREALLOC_ELEMENTS)
        .min(MAX_PREALLOC_ELEMENTS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn a_hostile_length_does_not_allocate_it() {
        // 2^48 declared, three bytes available. Before this helper the same
        // input aborted the process.
        let mut reader = Cursor::new(vec![1u8, 2, 3]);
        let result = read_exact_vec(&mut reader, 1 << 48, "locking script");
        assert!(matches!(result, Err(ChainGangError::BadData(_))));
    }

    #[test]
    fn u64_max_does_not_allocate_it() {
        let mut reader = Cursor::new(Vec::new());
        assert!(read_exact_vec(&mut reader, u64::MAX, "test").is_err());
    }

    #[test]
    fn an_honest_length_reads_exactly() {
        let mut reader = Cursor::new(vec![1u8, 2, 3, 4, 5]);
        let buf = read_exact_vec(&mut reader, 3, "test").expect("reads");
        assert_eq!(buf, vec![1, 2, 3]);
        // The remainder is left for the next field.
        let rest = read_exact_vec(&mut reader, 2, "test").expect("reads");
        assert_eq!(rest, vec![4, 5]);
    }

    #[test]
    fn a_zero_length_is_an_empty_vec_not_an_error() {
        let mut reader = Cursor::new(vec![9u8]);
        assert_eq!(
            read_exact_vec(&mut reader, 0, "test").expect("reads"),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn a_short_read_is_an_error_not_a_truncated_value() {
        let mut reader = Cursor::new(vec![1u8, 2]);
        assert!(read_exact_vec(&mut reader, 5, "test").is_err());
    }

    #[test]
    fn capacity_is_bounded() {
        assert_eq!(bounded_capacity(0), 0);
        assert_eq!(bounded_capacity(10), 10);
        assert_eq!(bounded_capacity(u64::MAX), MAX_PREALLOC_ELEMENTS);
        assert_eq!(
            bounded_capacity(MAX_PREALLOC_ELEMENTS as u64 + 1),
            MAX_PREALLOC_ELEMENTS
        );
    }
}
