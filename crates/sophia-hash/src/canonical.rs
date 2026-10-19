//! Canonical tag-length-value framing. See `canonical.md`.
//!
//! A field is a little-endian u32 tag length, the tag bytes, a little-endian
//! u32 payload length, then the payload. A payload may itself be a sequence
//! of fields. This is the framing of Moonkale's WP-66 normaliser v1, shared so
//! that the two systems' canonical streams are built the same way (their ids
//! still differ: the streams start with different prefixes).

/// Builds canonical bytes.
#[derive(Default, Debug, Clone)]
pub struct Writer {
    buf: Vec<u8>,
}

/// A position returned by [`Writer::open`], closed by [`Writer::close`].
#[must_use]
#[derive(Debug)]
pub struct Open(usize);

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(n: usize) -> Self {
        Writer { buf: Vec::with_capacity(n) }
    }

    /// Writes a complete field.
    pub fn field(&mut self, tag: &str, payload: &[u8]) {
        self.tag(tag);
        self.buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        self.buf.extend_from_slice(payload);
    }

    pub fn u32(&mut self, tag: &str, v: u32) {
        self.field(tag, &v.to_le_bytes());
    }

    pub fn u64(&mut self, tag: &str, v: u64) {
        self.field(tag, &v.to_le_bytes());
    }

    pub fn str(&mut self, tag: &str, v: &str) {
        self.field(tag, v.as_bytes());
    }

    /// Starts a field whose payload is written by further calls, up to the
    /// matching [`Writer::close`].
    pub fn open(&mut self, tag: &str) -> Open {
        self.tag(tag);
        let at = self.buf.len();
        self.buf.extend_from_slice(&[0; 4]);
        Open(at)
    }

    pub fn close(&mut self, open: Open) {
        let len = (self.buf.len() - open.0 - 4) as u32;
        self.buf[open.0..open.0 + 4].copy_from_slice(&len.to_le_bytes());
    }

    /// Appends bytes that are already canonical fields.
    pub fn raw(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn finish(self) -> Vec<u8> {
        self.buf
    }

    fn tag(&mut self, tag: &str) {
        self.buf.extend_from_slice(&(tag.len() as u32).to_le_bytes());
        self.buf.extend_from_slice(tag.as_bytes());
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReadError {
    #[error("truncated field at byte {0}")]
    Truncated(usize),
    #[error("tag is not UTF-8 at byte {0}")]
    BadTag(usize),
    #[error("expected tag {expected:?}, found {found:?}")]
    UnexpectedTag { expected: String, found: String },
    #[error("expected a {expected}-byte payload for {tag:?}, found {found}")]
    BadLength { tag: String, expected: usize, found: usize },
    #[error("no more fields")]
    End,
}

/// Reads a sequence of fields.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    buf: &'a [u8],
    at: usize,
    base: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Reader { buf, at: 0, base: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.at >= self.buf.len()
    }

    /// The next field's tag without consuming it.
    pub fn peek_tag(&self) -> Result<&'a str, ReadError> {
        self.clone().next().map(|(t, _)| t)
    }

    /// The next field as (tag, payload).
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<(&'a str, &'a [u8]), ReadError> {
        if self.is_empty() {
            return Err(ReadError::End);
        }
        let start = self.base + self.at;
        let tag_len = self.u32_raw().ok_or(ReadError::Truncated(start))? as usize;
        let tag = self.take(tag_len).ok_or(ReadError::Truncated(start))?;
        let tag = std::str::from_utf8(tag).map_err(|_| ReadError::BadTag(start))?;
        let len = self.u32_raw().ok_or(ReadError::Truncated(start))? as usize;
        let payload = self.take(len).ok_or(ReadError::Truncated(start))?;
        Ok((tag, payload))
    }

    /// The next field, which must have tag `tag`.
    pub fn expect(&mut self, tag: &str) -> Result<&'a [u8], ReadError> {
        let (t, p) = self.next()?;
        if t != tag {
            return Err(ReadError::UnexpectedTag { expected: tag.into(), found: t.into() });
        }
        Ok(p)
    }

    /// A reader over a field's payload, which must have tag `tag`.
    pub fn enter(&mut self, tag: &str) -> Result<Reader<'a>, ReadError> {
        let offset = self.base + self.at;
        let p = self.expect(tag)?;
        Ok(Reader { buf: p, at: 0, base: offset })
    }

    pub fn expect_u32(&mut self, tag: &str) -> Result<u32, ReadError> {
        let p = self.expect(tag)?;
        let a: [u8; 4] = p.try_into().map_err(|_| ReadError::BadLength {
            tag: tag.into(),
            expected: 4,
            found: p.len(),
        })?;
        Ok(u32::from_le_bytes(a))
    }

    pub fn expect_str(&mut self, tag: &str) -> Result<&'a str, ReadError> {
        let at = self.base + self.at;
        let p = self.expect(tag)?;
        std::str::from_utf8(p).map_err(|_| ReadError::BadTag(at))
    }

    fn u32_raw(&mut self) -> Option<u32> {
        let b = self.take(4)?;
        Some(u32::from_le_bytes(b.try_into().ok()?))
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(n)?;
        let s = self.buf.get(self.at..end)?;
        self.at = end;
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_layout_is_tag_length_value() {
        let mut w = Writer::new();
        w.field("ab", b"xyz");
        assert_eq!(w.finish(), [2, 0, 0, 0, b'a', b'b', 3, 0, 0, 0, b'x', b'y', b'z']);
    }

    #[test]
    fn nested_fields_round_trip() {
        let mut w = Writer::new();
        let o = w.open("outer");
        w.u32("n", 7);
        w.str("s", "hi");
        w.close(o);
        w.field("tail", &[]);
        let bytes = w.finish();

        let mut r = Reader::new(&bytes);
        let mut inner = r.enter("outer").unwrap();
        assert_eq!(inner.expect_u32("n").unwrap(), 7);
        assert_eq!(inner.expect_str("s").unwrap(), "hi");
        assert!(inner.is_empty());
        assert_eq!(r.expect("tail").unwrap(), b"");
        assert!(r.is_empty());
    }

    #[test]
    fn truncation_is_an_error_not_a_panic() {
        let mut w = Writer::new();
        w.field("tag", b"payload");
        let bytes = w.finish();
        for cut in 0..bytes.len() {
            let mut r = Reader::new(&bytes[..cut]);
            if cut == 0 {
                assert_eq!(r.next(), Err(ReadError::End));
            } else {
                assert!(matches!(r.next(), Err(ReadError::Truncated(_))));
            }
        }
    }
}
