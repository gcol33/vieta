//! Source text and the spans that point into it.

use core::fmt;

/// A byte range in the source text.
///
/// An empty span is a position rather than a range, which is what a token
/// inserted by error recovery carries (D37): it names where something was
/// expected and contributes nothing when the tree is printed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Span {
    /// Byte offset of the first byte.
    pub start: u32,
    /// Byte offset one past the last byte.
    pub end: u32,
}

impl Span {
    /// A span covering `start..end`.
    pub const fn new(start: u32, end: u32) -> Span {
        Span { start, end }
    }

    /// The empty span at an offset.
    pub const fn empty_at(offset: u32) -> Span {
        Span { start: offset, end: offset }
    }

    /// How many bytes the span covers.
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    /// Whether the span covers no bytes, which is what makes a leaf synthetic.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// The smallest span covering both.
    pub fn cover(self, other: Span) -> Span {
        Span { start: self.start.min(other.start), end: self.end.max(other.end) }
    }
}

/// Source that is known to be valid UTF-8.
///
/// The check happens once, before lexing, so every span lands on a character
/// boundary and a mis-encoded file produces one diagnostic rather than a
/// cascade. Losslessness is about malformed syntax, which the tree carries in
/// full (D37).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SourceText {
    text: String,
}

impl SourceText {
    /// Read source bytes, or report where they stop being UTF-8.
    pub fn new(bytes: &[u8]) -> Result<SourceText, NotUtf8> {
        match core::str::from_utf8(bytes) {
            Ok(text) => Ok(SourceText { text: text.to_owned() }),
            Err(error) => Err(NotUtf8 { offset: error.valid_up_to() }),
        }
    }

    /// The whole source.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The text a span covers, or `None` when the span is out of range or lands
    /// off a character boundary.
    pub fn slice(&self, span: Span) -> Option<&str> {
        self.text.get(span.start as usize..span.end as usize)
    }

    /// How many bytes the source holds.
    pub fn len(&self) -> u32 {
        self.text.len() as u32
    }

    /// Whether the source is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl From<String> for SourceText {
    fn from(text: String) -> SourceText {
        SourceText { text }
    }
}

impl From<&str> for SourceText {
    fn from(text: &str) -> SourceText {
        SourceText { text: text.to_owned() }
    }
}

/// Source bytes that are not valid UTF-8.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NotUtf8 {
    /// Byte offset of the first byte that is not part of a valid sequence.
    pub offset: usize,
}

impl fmt::Display for NotUtf8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "source is not valid UTF-8 at byte {}", self.offset)
    }
}

impl std::error::Error for NotUtf8 {}

#[cfg(test)]
mod tests {
    use super::{NotUtf8, SourceText, Span};

    #[test]
    fn spans_cover_and_measure() {
        let span = Span::new(2, 5);
        assert_eq!(span.len(), 3);
        assert!(!span.is_empty());
        assert!(Span::empty_at(7).is_empty());
        assert_eq!(span.cover(Span::new(9, 11)), Span::new(2, 11));
    }

    #[test]
    fn valid_utf8_is_accepted_whole() {
        let source = SourceText::new("x + \u{03b1}".as_bytes()).expect("valid");
        assert_eq!(source.as_str(), "x + \u{03b1}");
        assert_eq!(source.len(), 6);
        assert_eq!(source.slice(Span::new(0, 1)), Some("x"));
    }

    #[test]
    fn invalid_utf8_reports_where_it_stops() {
        let result = SourceText::new(&[b'x', b' ', 0xff, b'y']);
        assert_eq!(result, Err(NotUtf8 { offset: 2 }));
    }

    #[test]
    fn a_span_off_a_character_boundary_yields_nothing() {
        let source = SourceText::from("\u{03b1}");
        assert_eq!(source.slice(Span::new(0, 1)), None);
        assert_eq!(source.slice(Span::new(0, 2)), Some("\u{03b1}"));
    }
}
