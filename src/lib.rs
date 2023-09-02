#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::missing_inline_in_public_items
)]

use std::{
    fmt::Display,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

const STR_LEN: usize = 13;
const RND_BITS: usize = 36;
const RND_MASK: u64 = (1 << RND_BITS) - 1;
/// Seconds since Unix epoch for 2020-01-01T00:00:00Z.
const SECOND_EPOCH_MS: u64 = 1_577_836_800_000;

const CHARS: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

const NORMAL_MAPPING: [i8; 256] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1, -1, -1, -1, -1, -1, -1, 10, 11, 12, 13, 14, 15, 16, 17, 1,
    18, 19, 1, 20, 21, 0, 22, 23, 24, 25, 26, -2, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1, -1, 10,
    11, 12, 13, 14, 15, 16, 17, 1, 18, 19, 1, 20, 21, 0, 22, 23, 24, 25, 26, -2, 27, 28, 29, 30,
    31, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];

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
pub struct Id(u64);

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
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            - SECOND_EPOCH_MS;
        let rnd = fastrand::u64(..);
        let x = (time << RND_BITS) | (rnd & RND_MASK);
        Self(x)
    }

    #[must_use]
    #[inline]
    pub const fn from_u64(x: u64) -> Self {
        Self(x)
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

impl From<u64> for Id {
    #[inline]
    fn from(x: u64) -> Self {
        Self(x)
    }
}

const START_PLACE: u64 = 0x20u64.pow(12);

const fn decode(input: &[u8]) -> Result<u64, Error> {
    let mut input = input;
    if input.len() != 13 {
        return Err(Error::InvalidStrLen(input.len()));
    }
    let mut place = START_PLACE;
    let mut n = 0;
    while let [byte, rest @ ..] = input {
        let digit = match normalize(*byte) {
            Ok(digit) => digit,
            err @ Err(_) => return err,
        };
        n += digit.wrapping_mul(place);
        place >>= 5;
        input = rest;
    }
    Ok(n)
}

const fn normalize(byte: u8) -> Result<u64, Error> {
    let mapped = NORMAL_MAPPING[byte as usize];
    if mapped == -1 || mapped == -2 {
        return Err(Error::InvalidDigit(byte));
    }
    #[allow(clippy::cast_sign_loss)]
    Ok(mapped as u64)
}

const fn encode_array(n: u64) -> [u8; STR_LEN] {
    let mut n = n;
    let mut buf = [0u8; STR_LEN];

    if n == 0 {
        return buf;
    }

    let mut idx = 0;

    buf[idx] = CHARS[(n >> 60) as usize];
    idx += 1;
    n <<= 4;

    while idx < STR_LEN {
        buf[idx] = CHARS[(n >> 59) as usize];
        idx += 1;
        n <<= 5;
    }

    buf
}

pub trait Identifiable {
    fn id(&self) -> Id;
}

// Compile-time tests.
#[allow(non_upper_case_globals)]
const _: () = {
    use konst::{const_eq_for, result};
    // encoding
    assert!(const_eq_for!(slice; encode_array(0b1111 << 60), *b"f000000000000"));
    assert!(const_eq_for!(slice; encode_array(0b1111), *b"000000000000f"));
    assert!(const_eq_for!(slice; encode_array(0xFFFF_FFFF_FFFF_FFFF), *b"fzzzzzzzzzzzz"));
    // decoding
    assert!(result::unwrap_or!(decode(b"f000000000000"), 0) == 0b1111 << 60);
    assert!(result::unwrap_or!(decode(b"000000000000f"), 0) == 0b1111);
    assert!(result::unwrap_or!(decode(b"fzzzzzzzzzzzz"), 0) == 0xFFFF_FFFF_FFFF_FFFF);
};

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_to_string() {
        let id = Id(0b1111 << 60);
        let s = id.to_string();
        let mut expected = [CHARS[0]; 13];
        expected[0] = CHARS[15];
        assert_eq!(s, std::str::from_utf8(&expected).unwrap());

        let id = Id(0b1111);
        let s = id.to_string();
        let mut expected = [CHARS[0]; 13];
        expected[12] = CHARS[15];
        assert_eq!(s, std::str::from_utf8(&expected).unwrap());
    }

    #[test]
    fn test_from_str() {
        let id = Id::from_str("f000000000000").unwrap();
        assert_eq!(id.0, 0b01111 << 60);
        let id = Id::from_str("f00000000000f").unwrap();
        assert_eq!(id.0, 0b01111 << 60 | 0b01111);
        let id = Id::from_str("000000000000f").unwrap();
        assert_eq!(id.0, 0b01111);
        let id = Id::from_str("fzzzzzzzzzzzz").unwrap();
        assert_eq!(id.0, 0xFFFF_FFFF_FFFF_FFFF);
    }

    #[test]
    fn is_ok() {
        let id = Id::from(0xdead_beef_beef_dead);
        assert_eq!("dxbdyxyzezqnd", id.to_string());

        let id = Id::from_str("dxbdyxyzezqnd").unwrap();
        assert_eq!(id.0, 0xdead_beef_beef_dead);

        let id = Id::new();
        let s = id.to_string();
        assert_eq!(s, Id::from_str(&s).unwrap().to_string());
        assert_eq!(id, Id::from_str(&s).unwrap());
    }
}

#[cfg(kani)]
#[kani::proof]
fn check_encoding() {
    use kani;
    let x: u64 = kani::any();
    let arr = encode_array(x);
    assert!(decode(&arr).unwrap() == x);
}
