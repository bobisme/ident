use core::fmt::Write;
use std::{
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};

const STR_LEN: usize = 13;
const RND_BITS: usize = 36;
const RND_MASK: u64 = (1 << RND_BITS) - 1;

static CHARS: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not encode id {0}: {1}")]
    Encode(u64, std::fmt::Error),
    #[error("IdStr full: tried to write {byte} @ {idx}")]
    IdStrFull { byte: u8, idx: usize },
}

#[derive(Clone, Debug, Default)]
pub struct IdStr {
    data: [u8; STR_LEN],
    idx: usize,
}

impl IdStr {
    pub fn new(x: u64) -> Self {
        let mut idstr = Self::default();
        encode(x, &mut idstr)
            .map_err(|e| Error::Encode(x, e))
            .map_err(|e| {
                eprintln!("ERROR: {}", e);
                e
            })
            .unwrap();
        idstr
    }

    pub fn write_char(&mut self, c: impl Into<u8>) -> Result<(), Error> {
        let byte = c.into();
        if self.idx >= STR_LEN {
            return Err(Error::IdStrFull {
                byte,
                idx: self.idx,
            });
        }
        self.data[self.idx] = byte;
        self.idx += 1;
        Ok(())
    }
}

impl Write for IdStr {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for c in s.as_bytes() {
            self.write_char(*c).unwrap();
        }
        Ok(())
    }
}

/// Id is a STR_LEN-char representation of a 64-bit number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Id([u8; STR_LEN]);

impl Id {
    pub fn new() -> Self {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let rnd = fastrand::u64(..);
        let x = (time << RND_BITS) | (rnd & RND_MASK);
        let idstr = IdStr::new(x);
        Self(idstr.data)
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in self.0 {
            f.write_char(c as char)?;
        }
        Ok(())
    }
}

impl From<u64> for Id {
    fn from(x: u64) -> Self {
        let idstr = IdStr::new(x);
        Self(idstr.data)
    }
}

impl From<&str> for Id {
    fn from(x: &str) -> Self {
        let mut inner = [0u8; STR_LEN];
        for (i, c) in x.as_bytes().iter().enumerate() {
            inner[i] = *c;
        }
        Self(inner)
    }
}

// This is taken from the crockford crate but modified to _not_ strip leading
// zeroes and to use a lowercase char set.
pub fn encode<T: Write>(mut n: u64, buf: &mut T) -> Result<(), std::fmt::Error> {
    // Used for the initial shift.
    const QUAD_SHIFT: usize = 60;

    // Used for all subsequent shifts.
    const FIVE_SHIFT: usize = 59;
    const FIVE_RESET: usize = 5;

    // After we clear the four most significant bits, the four least significant bits will be
    // replaced with 0001. We can then know to stop once the four most significant bits are,
    // likewise, 0001.
    const STOP_BIT: u64 = 1 << QUAD_SHIFT;

    if n == 0 {
        buf.write_char('0')?;
        return Ok(());
    }

    let mut idx = 0;

    // From now until we reach the stop bit, take the five most significant bits and then shift
    // left by five bits.
    while n != STOP_BIT && idx < STR_LEN {
        buf.write_char(CHARS[(n >> FIVE_SHIFT) as usize] as char)?;
        match idx {
            5 => {
                idx += 2;
                buf.write_char('.')?;
            }
            _ => idx += 1,
        };
        n <<= FIVE_RESET;
    }

    Ok(())
}

pub trait Identifiable {
    fn id(&self) -> Id;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn is_ok() {
        let id = Id::from(0xdeadbeefbeefdead);
        assert_eq!("vtpvxv.xyxzfa", id.to_string());
    }
}
