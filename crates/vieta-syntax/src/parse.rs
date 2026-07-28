//! The parser: total, error-tolerant, and lossless.
//!
//! Every token the lexer produced reaches the tree, in order. A token the
//! grammar wanted and did not find is inserted with an empty span, so it names
//! the problem for a diagnostic and contributes nothing when the tree is
//! printed (D37).

use crate::cst::{Builder, Cst, NodeKind, SyntaxError};
use crate::lexer::lex;
use crate::source::{SourceText, Span};
use crate::token::{Token, TokenKind};

/// How deeply expressions may nest before the parser stops recursing. Input
/// past it still parses and still prints; it lands in error nodes.
const MAX_DEPTH: u32 = 128;

/// Left and right binding power of an infix operator. `^` binds tighter on the
/// right, which is what makes it associate to the right.
const fn infix_binding(kind: TokenKind) -> Option<(u8, u8)> {
    match kind {
        TokenKind::Plus | TokenKind::Minus => Some((1, 2)),
        TokenKind::Star | TokenKind::Slash => Some((3, 4)),
        TokenKind::Caret => Some((6, 5)),
        _ => None,
    }
}

/// Prefix `-` binds tighter than `*` and looser than `^`, so `-x*y` is `(-x)*y`
/// and `-x^2` is `-(x^2)`.
const UNARY_BINDING: u8 = 5;

/// Parse source into a lossless tree.
///
/// Never fails. Malformed input produces error nodes and inserted tokens, and
/// [`Cst::print`] still reproduces the source exactly.
pub fn parse(source: SourceText) -> Cst {
    let tokens = lex(source.as_str());
    let end = source.len();
    let mut parser = Parser {
        tokens: &tokens,
        pos: 0,
        end,
        depth: 0,
        builder: Builder::new(),
        errors: Vec::new(),
    };

    parser.builder.start(NodeKind::Root);
    parser.eat_trivia();
    if parser.peek().is_some() {
        parser.expr(0);
    }
    parser.trailing();
    parser.builder.finish();

    let Parser { builder, errors, .. } = parser;
    builder.build(source, errors)
}

struct Parser<'t> {
    tokens: &'t [Token],
    pos: usize,
    end: u32,
    depth: u32,
    builder: Builder,
    errors: Vec<SyntaxError>,
}

impl Parser<'_> {
    /// The next kind that carries meaning, without moving or emitting.
    fn peek(&self) -> Option<TokenKind> {
        self.tokens[self.pos..]
            .iter()
            .find(|token| !token.kind.is_trivia())
            .map(|token| token.kind)
    }

    /// Emit the trivia sitting at the current position. Where it lands in the
    /// tree does not matter to the printed output, since the leaf order is the
    /// source order either way.
    fn eat_trivia(&mut self) {
        while let Some(token) = self.tokens.get(self.pos) {
            if !token.kind.is_trivia() {
                break;
            }
            self.builder.token(*token);
            self.pos += 1;
        }
    }

    fn bump(&mut self) {
        self.eat_trivia();
        if let Some(token) = self.tokens.get(self.pos) {
            self.builder.token(*token);
            self.pos += 1;
        }
    }

    /// Where the next significant token starts, or the end of the source.
    fn offset(&self) -> u32 {
        self.tokens.get(self.pos).map_or(self.end, |token| token.span.start)
    }

    fn expect(&mut self, kind: TokenKind) {
        if self.peek() == Some(kind) {
            self.bump();
            return;
        }
        self.eat_trivia();
        let span = Span::empty_at(self.offset());
        self.builder.token(Token { kind, span });
        self.errors.push(SyntaxError {
            span,
            message: format!("expected {}", kind.describe()),
        });
    }

    /// Wrap one unusable token in an error node, so that it still prints.
    fn unexpected(&mut self, message: &str) {
        self.eat_trivia();
        let span = self
            .tokens
            .get(self.pos)
            .map_or(Span::empty_at(self.end), |token| token.span);
        self.builder.start(NodeKind::Error);
        if self.pos < self.tokens.len() {
            self.bump();
        }
        self.builder.finish();
        self.errors.push(SyntaxError { span, message: message.to_owned() });
    }

    fn expr(&mut self, min_binding: u8) {
        if self.depth >= MAX_DEPTH {
            self.unexpected("expression nests too deeply");
            return;
        }
        self.depth += 1;
        self.eat_trivia();
        let checkpoint = self.builder.checkpoint();
        self.prefix();
        while let Some(kind) = self.peek() {
            let Some((left, right)) = infix_binding(kind) else {
                break;
            };
            if left < min_binding {
                break;
            }
            self.builder.start_at(checkpoint, NodeKind::BinaryExpr);
            self.bump();
            self.expr(right);
            self.builder.finish();
        }
        self.depth -= 1;
    }

    fn prefix(&mut self) {
        self.eat_trivia();
        match self.peek() {
            Some(TokenKind::Minus) => {
                self.builder.start(NodeKind::UnaryExpr);
                self.bump();
                self.expr(UNARY_BINDING);
                self.builder.finish();
            }
            Some(TokenKind::LParen) => {
                self.builder.start(NodeKind::ParenExpr);
                self.bump();
                self.expr(0);
                self.expect(TokenKind::RParen);
                self.builder.finish();
            }
            Some(TokenKind::Int | TokenKind::Real) => {
                self.builder.start(NodeKind::Literal);
                self.bump();
                self.builder.finish();
            }
            Some(TokenKind::KwFn) => self.lambda(),
            Some(TokenKind::KwLet) => self.binding(),
            Some(TokenKind::KwTerm) => self.quotation(),
            Some(TokenKind::Ident) => {
                let checkpoint = self.builder.checkpoint();
                self.builder.start(NodeKind::NameRef);
                self.bump();
                self.builder.finish();
                while self.peek() == Some(TokenKind::LParen) {
                    self.builder.start_at(checkpoint, NodeKind::CallExpr);
                    self.arguments();
                    self.builder.finish();
                }
            }
            _ => self.unexpected("expected an expression"),
        }
    }

    fn arguments(&mut self) {
        self.builder.start(NodeKind::ArgList);
        self.expect(TokenKind::LParen);
        loop {
            match self.peek() {
                None | Some(TokenKind::RParen) => break,
                _ => {}
            }
            self.expr(0);
            if self.peek() == Some(TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(TokenKind::RParen);
        self.builder.finish();
    }

    fn lambda(&mut self) {
        self.builder.start(NodeKind::Lambda);
        self.bump();
        self.expect(TokenKind::LParen);
        self.expect(TokenKind::Ident);
        self.expect(TokenKind::RParen);
        self.expect(TokenKind::FatArrow);
        self.expr(0);
        self.builder.finish();
    }

    /// `term { e }`. The body uses the same grammar as everything else, so what
    /// a quotation changes is elaboration and not parsing.
    fn quotation(&mut self) {
        self.builder.start(NodeKind::Quote);
        self.bump();
        self.expect(TokenKind::LBrace);
        self.expr(0);
        self.expect(TokenKind::RBrace);
        self.builder.finish();
    }

    fn binding(&mut self) {
        self.builder.start(NodeKind::Let);
        self.bump();
        self.expect(TokenKind::Ident);
        self.expect(TokenKind::Equals);
        self.expr(0);
        self.expect(TokenKind::KwIn);
        self.expr(0);
        self.builder.finish();
    }

    /// Anything left over after the expression, kept so that it prints.
    fn trailing(&mut self) {
        self.eat_trivia();
        let Some(first) = self.tokens.get(self.pos) else {
            return;
        };
        let start = first.span.start;
        self.builder.start(NodeKind::Error);
        while let Some(&token) = self.tokens.get(self.pos) {
            self.builder.token(token);
            self.pos += 1;
        }
        self.builder.finish();
        self.errors.push(SyntaxError {
            span: Span::new(start, self.end),
            message: "unexpected input after the expression".to_owned(),
        });
    }
}
