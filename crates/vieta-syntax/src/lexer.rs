//! The lexer, which is total: every step consumes at least one character or
//! reaches the end of the source (D37).
//!
//! The tokens it produces tile the source exactly, which is the invariant the
//! byte-exact round trip rests on.

use crate::source::Span;
use crate::token::{Token, TokenKind};

const BOM: char = '\u{feff}';

pub(crate) fn lex(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut rest = text;
    let mut offset = 0u32;

    while let Some(first) = rest.chars().next() {
        let width = scan(first, rest);
        let kind = classify(first, &rest[..width], offset);
        tokens.push(Token { kind, span: Span::new(offset, offset + width as u32) });
        rest = &rest[width..];
        offset += width as u32;
    }
    tokens
}

/// How many bytes the next token covers. Never zero, which is what makes the
/// loop above terminate.
fn scan(first: char, rest: &str) -> usize {
    if first.is_whitespace() {
        return run(rest, char::is_whitespace);
    }
    if is_ident_start(first) {
        return run(rest, is_ident_continue);
    }
    if first.is_ascii_digit() {
        return number(rest);
    }
    if rest.starts_with("//") {
        return rest.find(['\n', '\r']).unwrap_or(rest.len());
    }
    if rest.starts_with("/*") {
        return match rest[2..].find("*/") {
            Some(end) => 2 + end + 2,
            None => rest.len(),
        };
    }
    if rest.starts_with("=>") {
        return 2;
    }
    first.len_utf8()
}

fn classify(first: char, text: &str, offset: u32) -> TokenKind {
    if first == BOM && offset == 0 {
        return TokenKind::Bom;
    }
    if first.is_whitespace() {
        return TokenKind::Whitespace;
    }
    if is_ident_start(first) {
        return match text {
            "fn" => TokenKind::KwFn,
            "let" => TokenKind::KwLet,
            "in" => TokenKind::KwIn,
            "term" => TokenKind::KwTerm,
            _ => TokenKind::Ident,
        };
    }
    if first.is_ascii_digit() {
        return if text.contains('.') { TokenKind::Real } else { TokenKind::Int };
    }
    if text.starts_with("//") {
        return TokenKind::LineComment;
    }
    if text.starts_with("/*") {
        return TokenKind::BlockComment;
    }
    match text {
        "=>" => TokenKind::FatArrow,
        "+" => TokenKind::Plus,
        "-" => TokenKind::Minus,
        "*" => TokenKind::Star,
        "/" => TokenKind::Slash,
        "^" => TokenKind::Caret,
        "(" => TokenKind::LParen,
        ")" => TokenKind::RParen,
        "{" => TokenKind::LBrace,
        "}" => TokenKind::RBrace,
        "," => TokenKind::Comma,
        "=" => TokenKind::Equals,
        _ => TokenKind::Unknown,
    }
}

fn run(text: &str, accept: impl Fn(char) -> bool) -> usize {
    text.char_indices()
        .find(|&(_, ch)| !accept(ch))
        .map_or(text.len(), |(index, _)| index)
}

/// Digits, optionally `_`-separated, and at most one fractional part. A `.` not
/// followed by a digit ends the number, so `1.` lexes as `1` and an unknown `.`.
fn number(text: &str) -> usize {
    let mut end = digits(text, 0);
    if text[end..].starts_with('.') {
        let after = digits(text, end + 1);
        if after > end + 1 {
            end = after;
        }
    }
    end
}

fn digits(text: &str, from: usize) -> usize {
    let mut end = from;
    for (index, ch) in text[from..].char_indices() {
        if ch.is_ascii_digit() || ch == '_' {
            end = from + index + ch.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || (ch.is_alphabetic() && ch != BOM)
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || (ch.is_alphanumeric() && ch != BOM)
}

#[cfg(test)]
mod tests {
    use super::lex;
    use crate::token::TokenKind;

    fn kinds(text: &str) -> Vec<TokenKind> {
        lex(text).into_iter().map(|token| token.kind).collect()
    }

    /// The property the round trip rests on.
    fn tiles(text: &str) -> bool {
        let mut offset = 0;
        for token in lex(text) {
            if token.span.start != offset {
                return false;
            }
            offset = token.span.end;
        }
        offset as usize == text.len()
    }

    #[test]
    fn tokens_tile_the_source() {
        for text in [
            "",
            "x",
            "x + 2",
            "  \t\r\n ",
            "f(x, y) ^ 2",
            "let a = 1 in a",
            "fn(x) => x * x",
            "term { x + 2 }",
            "\u{feff}x",
            "@$\u{00a7}",
            "/* unclosed",
            "1.50 + 0.5",
            "x // trailing",
            "\u{03b1} + \u{03b2}",
        ] {
            assert!(tiles(text), "{text:?}");
        }
    }

    #[test]
    fn every_token_consumes_at_least_one_character() {
        for text in ["@", ".", "\u{feff}", "/", "=", "\u{1f600}"] {
            let tokens = lex(text);
            assert!(!tokens.is_empty(), "{text:?}");
            assert!(tokens.iter().all(|token| token.span.len() > 0), "{text:?}");
        }
    }

    #[test]
    fn words_and_keywords_separate() {
        assert_eq!(kinds("fn"), vec![TokenKind::KwFn]);
        assert_eq!(kinds("let"), vec![TokenKind::KwLet]);
        assert_eq!(kinds("in"), vec![TokenKind::KwIn]);
        assert_eq!(kinds("term"), vec![TokenKind::KwTerm]);
        assert_eq!(kinds("fnord"), vec![TokenKind::Ident]);
        assert_eq!(kinds("terminal"), vec![TokenKind::Ident]);
        assert_eq!(kinds("\u{03b1}_1"), vec![TokenKind::Ident]);
    }

    #[test]
    fn numbers_keep_their_spelling() {
        assert_eq!(kinds("1_000"), vec![TokenKind::Int]);
        assert_eq!(kinds("1.50"), vec![TokenKind::Real]);
        assert_eq!(kinds("1."), vec![TokenKind::Int, TokenKind::Unknown]);
    }

    #[test]
    fn comments_are_trivia_and_reach_where_they_should() {
        assert_eq!(
            kinds("x // c\ny"),
            vec![
                TokenKind::Ident,
                TokenKind::Whitespace,
                TokenKind::LineComment,
                TokenKind::Whitespace,
                TokenKind::Ident,
            ]
        );
        assert_eq!(kinds("/* a */"), vec![TokenKind::BlockComment]);
        assert_eq!(kinds("/* a"), vec![TokenKind::BlockComment]);
    }

    #[test]
    fn a_byte_order_mark_is_trivia_only_at_the_start() {
        assert_eq!(kinds("\u{feff}x"), vec![TokenKind::Bom, TokenKind::Ident]);
        assert_eq!(kinds("x\u{feff}"), vec![TokenKind::Ident, TokenKind::Unknown]);
    }

    #[test]
    fn the_fat_arrow_does_not_split() {
        assert_eq!(kinds("=>"), vec![TokenKind::FatArrow]);
        assert_eq!(kinds("="), vec![TokenKind::Equals]);
    }
}
