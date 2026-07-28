//! Tokens, including the trivia that a lossless tree keeps as leaves.

use crate::source::Span;

/// What a leaf of the tree is.
///
/// Trivia are ordinary kinds here rather than a separate channel, because a
/// lossless tree holds them as leaves in source order (D37). Whether an API
/// presents a comment as leading or trailing is a view over that order and not
/// a property of the tree.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TokenKind {
    /// A run of whitespace, preserved exactly, including tabs and CRLF.
    Whitespace,
    /// A `//` comment up to but not including the line break.
    LineComment,
    /// A `/* */` comment, which reaches the end of the source when unclosed.
    BlockComment,
    /// A byte-order mark at the start of the source.
    Bom,
    /// An identifier, or a keyword that is not one of the reserved words.
    Ident,
    /// `fn`
    KwFn,
    /// `let`
    KwLet,
    /// `in`
    KwIn,
    /// `term`
    KwTerm,
    /// A run of digits, with `_` allowed between them.
    Int,
    /// Digits, a `.`, and digits.
    Real,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `^`
    Caret,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `,`
    Comma,
    /// `=`
    Equals,
    /// `=>`
    FatArrow,
    /// A character the lexer does not recognize, one character at a time.
    Unknown,
}

impl TokenKind {
    /// Whether this kind carries no meaning for elaboration.
    pub const fn is_trivia(self) -> bool {
        matches!(
            self,
            TokenKind::Whitespace
                | TokenKind::LineComment
                | TokenKind::BlockComment
                | TokenKind::Bom
        )
    }

    /// How to name this kind in a diagnostic.
    pub const fn describe(self) -> &'static str {
        match self {
            TokenKind::KwFn => "`fn`",
            TokenKind::KwLet => "`let`",
            TokenKind::KwIn => "`in`",
            TokenKind::KwTerm => "`term`",
            TokenKind::Plus => "`+`",
            TokenKind::Minus => "`-`",
            TokenKind::Star => "`*`",
            TokenKind::Slash => "`/`",
            TokenKind::Caret => "`^`",
            TokenKind::LParen => "`(`",
            TokenKind::RParen => "`)`",
            TokenKind::LBrace => "`{`",
            TokenKind::RBrace => "`}`",
            TokenKind::Comma => "`,`",
            TokenKind::Equals => "`=`",
            TokenKind::FatArrow => "`=>`",
            TokenKind::Ident => "an identifier",
            TokenKind::Int | TokenKind::Real => "a number",
            _ => "a token",
        }
    }
}

/// A leaf of the tree.
///
/// An empty span means the leaf was inserted by error recovery. It names where
/// something was expected and prints as nothing, so recovery never fabricates
/// bytes the source did not contain (D37).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Token {
    /// What the leaf is.
    pub kind: TokenKind,
    /// Where it sits in the source.
    pub span: Span,
}

impl Token {
    /// Whether error recovery inserted this leaf.
    pub const fn is_synthetic(self) -> bool {
        self.span.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{Token, TokenKind};
    use crate::source::Span;

    #[test]
    fn trivia_kinds_are_the_ones_elaboration_ignores() {
        for kind in [
            TokenKind::Whitespace,
            TokenKind::LineComment,
            TokenKind::BlockComment,
            TokenKind::Bom,
        ] {
            assert!(kind.is_trivia(), "{kind:?}");
        }
        for kind in [TokenKind::Ident, TokenKind::Plus, TokenKind::Unknown] {
            assert!(!kind.is_trivia(), "{kind:?}");
        }
    }

    #[test]
    fn an_empty_span_is_what_makes_a_leaf_synthetic() {
        let real = Token { kind: TokenKind::RParen, span: Span::new(3, 4) };
        let inserted = Token { kind: TokenKind::RParen, span: Span::empty_at(4) };
        assert!(!real.is_synthetic());
        assert!(inserted.is_synthetic());
    }
}
