//! A 256-bit BLAKE3 digest. See `digest.md`.

use std::fmt;
use std::str::FromStr;

/// A 256-bit BLAKE3 digest: the identity of a definition, a component, a
/// nominal entity, an annotation or a derived fact.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest(pub [u8; 32]);

impl Digest {
    pub const LEN: usize = 32;

    /// BLAKE3 of `bytes`.
    pub fn of(bytes: &[u8]) -> Self {
        Digest(*blake3::hash(bytes).as_bytes())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        <[u8; 32]>::try_from(bytes).ok().map(Digest)
    }

    /// Full lowercase hex (64 characters).
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.0 {
            s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
            s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
        }
        s
    }

    /// The first `n` hex characters, for display (like git's short hashes).
    pub fn short(&self, n: usize) -> String {
        let mut s = self.to_hex();
        s.truncate(n.min(64));
        s
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.short(12))
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("not a 64-character hex digest")]
pub struct ParseDigestError;

impl FromStr for Digest {
    type Err = ParseDigestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.len() != 64 {
            return Err(ParseDigestError);
        }
        let mut out = [0u8; 32];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let hi = (chunk[0] as char).to_digit(16).ok_or(ParseDigestError)?;
            let lo = (chunk[1] as char).to_digit(16).ok_or(ParseDigestError)?;
            out[i] = (hi * 16 + lo) as u8;
        }
        Ok(Digest(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        let d = Digest::of(b"sophia");
        assert_eq!(d.to_hex().parse::<Digest>().unwrap(), d);
        assert_eq!(format!("#{}", d.to_hex()).parse::<Digest>().unwrap(), d);
        assert_eq!(d.short(8).len(), 8);
    }

    #[test]
    fn rejects_bad_hex() {
        assert!("xyz".parse::<Digest>().is_err());
        assert!("g".repeat(64).parse::<Digest>().is_err());
    }
}
