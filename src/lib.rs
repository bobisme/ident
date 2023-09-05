#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::missing_inline_in_public_items
)]
//! 100-bit ID stored in a u128.
//! 64-bits of randomness every 31.25 milliseconds.
//! 36-bits for time component with an epoch of 2020-01-01
//! should last until 2088-01-14T22:14:07

use std::{
    fmt::Display,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const STR_LEN: usize = 22;
const SEP_IDX: [usize; 2] = [6, 15];
const RND_BITS: usize = 64;
const RND_MASK: u128 = (1 << RND_BITS) - 1;
const ID_BITS: usize = 100;
const ID_MASK: u128 = (1 << ID_BITS) - 1;
/// Seconds since Unix epoch for 2020-01-01T00:00:00Z.
const SECOND_EPOCH: u128 = 1_577_836_800;
const TIME_SHIFT: usize = 5;

const CHARS_STR: &str = "0123456789abcdefghjkmnpqrstvwxyz";
const CHARS: &[u8] = CHARS_STR.as_bytes();

const DECODE_MAP: [i8; 256] = {
    let mut arr = [-1i8; 256];
    let mut i = 0;
    while i < CHARS.len() {
        arr[CHARS[i] as usize] = i as i8;
        i += 1;
    }
    arr
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not encode id {0}: {1}")]
    Encode(u64, std::fmt::Error),
    #[error("IdStr full: tried to write {byte} @ {idx}")]
    IdStrFull { byte: u8, idx: usize },
    #[error("decoding error: invalid digit: {0}")]
    InvalidDigit(u8),
    #[error("decoding error: string must be exactly 13 characters, got {0}")]
    InvalidStrLen(usize),
}

/// Id is a STR_LEN-char representation of a 64-bit number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id(u128);

const fn timestamp_from_unix_dur(dur: Duration) -> u128 {
    ((dur.as_millis() - (SECOND_EPOCH * 1_000)) << TIME_SHIFT) / 1_000
}

impl Id {
    /// Creates a new [`Id`].
    ///
    /// # Panics
    ///
    /// Panics if now is somehow earlier than the unix epoch.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        #[allow(clippy::cast_possible_truncation)]
        let unix_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let time = timestamp_from_unix_dur(unix_time);
        let rnd = fastrand::u128(..);
        let x = (time << RND_BITS) | (rnd & RND_MASK);
        Self(x)
    }

    #[must_use]
    #[inline]
    pub const fn from_u128(x: u128) -> Self {
        Self(x & ((1 << ID_BITS) - 1))
    }
}

impl Default for Id {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for Id {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(decode(s.as_bytes())?))
    }
}

impl Display for Id {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let arr = encode_array(self.0);
        let s = unsafe { std::str::from_utf8_unchecked(&arr[..]) };
        f.write_str(s)
    }
}

impl From<u128> for Id {
    #[inline]
    fn from(x: u128) -> Self {
        Self(x)
    }
}

#[allow(clippy::cast_possible_truncation)]
const MOST_SIGNIFICANT_POSITION: u128 = 32 << (5 * (STR_LEN - SEP_IDX.len() - 2));

const fn decode(input: &[u8]) -> Result<u128, Error> {
    let mut input = input;
    if input.len() != STR_LEN {
        return Err(Error::InvalidStrLen(input.len()));
    }
    let mut place = MOST_SIGNIFICANT_POSITION;
    let mut n: u128 = 0;
    // NOTE: no `for` loops in const.
    // I checked out the assembly for `while` vs `for` and it's
    // exactly the same. --bob
    while let [byte, rest @ ..] = input {
        input = rest;
        if *byte == b'-' {
            continue;
        }
        let digit = match map_byte(*byte) {
            Ok(digit) => digit,
            Err(e) => return Err(e),
        };
        n += place.wrapping_mul(digit as u128);
        place >>= 5;
    }
    Ok(n)
}

const fn map_byte(byte: u8) -> Result<u8, Error> {
    let mapped = DECODE_MAP[byte as usize];
    if mapped == -1 || mapped == -2 {
        return Err(Error::InvalidDigit(byte));
    }
    #[allow(clippy::cast_sign_loss)]
    Ok(mapped as u8)
}

const fn to_100bit(n: u128) -> u128 {
    n & ID_MASK
}

const fn encode_array(n: u128) -> [u8; STR_LEN] {
    let mut n = to_100bit(n);
    let mut buf = *b"000000-00000000-000000";

    if n == 0 {
        return buf;
    }

    let mut idx = 0;

    while idx < STR_LEN {
        if !(idx == SEP_IDX[0] || idx == SEP_IDX[1]) {
            buf[idx] = CHARS[((n >> (ID_BITS - 5)) & 0b11111) as usize];
            n <<= 5;
        }
        idx += 1;
    }

    buf
}

pub trait Identifiable {
    fn id(&self) -> Id;
}

// Compile-time tests.
const _: () = {
    use konst::{const_eq_for, result};
    assert!(
        to_100bit(0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF) == 0xF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF
    );
    // encoding
    assert!(const_eq_for!(slice; encode_array(0b1111 << 60), *b"000000-0f000000-000000"));
    assert!(const_eq_for!(slice; encode_array(0b1111), *b"000000-00000000-00000f"));
    assert!(const_eq_for!(slice; encode_array(0xFFFF_FFFF_FFFF_FFFF), *b"000000-0fzzzzzz-zzzzzz"));
    assert!(const_eq_for!(slice; encode_array((1 << 100) - 1), *b"zzzzzz-zzzzzzzz-zzzzzz"));
    assert!(const_eq_for!(slice; encode_array(1 << 100), *b"000000-00000000-000000"));

    // // decoding
    assert!(result::unwrap_or!(decode(b"000000-0f000000-000000"), 0) == 0b1111 << 60);
    assert!(result::unwrap_or!(decode(b"000000-00000000-00000f"), 0) == 0b1111);
    assert!(result::unwrap_or!(decode(b"000000-0fzzzzzz-zzzzzz"), 0) == 0xFFFF_FFFF_FFFF_FFFF);
    assert!(result::unwrap_or!(decode(b"z00000-00000000-000000"), 0) == 0b11111 << 95);
    assert!(result::unwrap_or!(decode(b"zzzzzz-zzzzzzzz-zzzzzz"), 0) == (1 << 100) - 1);
};

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_str() {
        let id = Id::from_str("000000-0f000000-000000").unwrap();
        assert_eq!(id.0, 0b01111 << 60);
        let id = Id::from_str("000000-0f000000-00000f").unwrap();
        assert_eq!(id.0, 0b01111 << 60 | 0b01111);
        let id = Id::from_str("000000-00000000-00000f").unwrap();
        assert_eq!(id.0, 0b01111);
        let id = Id::from_str("000000-0fzzzzzz-zzzzzz").unwrap();
        assert_eq!(id.0, 0xFFFF_FFFF_FFFF_FFFF);
    }

    #[test]
    fn is_ok() {
        let id = Id::from(0xdead_beef_beef_dead);
        assert_eq!("000000-0dxbdyxy-zezqnd", id.to_string());
        assert_eq!(encode_array(1 << 100), *b"000000-00000000-000000");
        // let id = Id::from(0x10000000000000000000000000);
        // assert_eq!(id.to_string(), "ass");

        let id = Id::from_str("000000-0dxbdyxy-zezqnd").unwrap();
        assert_eq!(id.0, 0xdead_beef_beef_dead);

        let id = Id::new();
        let s = id.to_string();
        assert_eq!(s, Id::from_str(&s).unwrap().to_string());
        assert_eq!(id, Id::from_str(&s).unwrap());
    }

    const PATTERN: &str = konst::string::str_concat!(&[
        "[", CHARS_STR, "]{6}-[", CHARS_STR, "]{8}-[", CHARS_STR, "]{6}"
    ]);

    proptest::proptest! {
        #[test]
        fn doesnt_crash(s in "\\PC*") {
            let _ = Id::from_str(&s);
        }

        #[test]
        fn parses_valid_ids(s in PATTERN) {
            Id::from_str(&s).unwrap();
        }

        #[test]
        fn encodes_u128s(x in 0u128..) {
            let encoded_id = Id::from_u128(x);
            let decoded_id = Id::from_str(&encoded_id.to_string()).unwrap();
            assert!(encoded_id.0 == decoded_id.0);
        }
    }
}

#[cfg(kani)]
#[kani::proof]
fn check_encoding() {
    use kani;
    let x: u128 = kani::any();
    let arr = encode_array(x);
    let decoded = decode(&arr).unwrap();
    assert!(decoded == (x & ((1 << ID_BITS) - 1)));
}
