//! Vieta's syntax layer: a lossless concrete tree, and the surface syntax
//! elaboration reads.
//!
//! Two representations, because exact source preservation and convenient
//! semantic structure pull against each other at every node (D37). The
//! concrete tree keeps every token, comment, and redundant parenthesis, so a
//! formatter, a diagnostic, and a macro can all see what was written. Surface
//! [`Syntax`] drops grouping and trivia, keeps names and binding forms, and
//! carries an [`Origin`] on every node.
//!
//! ```
//! use vieta_syntax::{lower, parse, SourceText};
//!
//! let source = "f(x) + /* why */ 2";
//! let tree = parse(SourceText::from(source));
//!
//! assert_eq!(tree.print(), source);
//! assert!(tree.errors().is_empty());
//! ```
//!
//! Printing is the concatenation of the leaves, and the non-synthetic leaves
//! tile the source exactly, so the round trip holds for malformed input too.
//! Recovery never fabricates bytes:
//!
//! ```
//! # use vieta_syntax::{parse, SourceText};
//! let tree = parse(SourceText::from("f(x"));
//!
//! assert_eq!(tree.print(), "f(x");
//! assert_eq!(tree.errors().len(), 1);
//! ```
//!
//! Grouping is a fact about the source rather than about the program, so it
//! survives in the concrete tree and not above it:
//!
//! ```
//! # use vieta_syntax::{lower, parse, SourceText};
//! let grouped = parse(SourceText::from("(x + y)"));
//! let plain = parse(SourceText::from("x + y"));
//!
//! assert_ne!(grouped.print(), plain.print());
//! assert!(lower(&grouped).same_shape(&lower(&plain)));
//! ```
//!
//! This crate knows nothing about terms or the store. Names are still names
//! here, and resolving them to operator identities is elaboration's job.

mod cst;
mod lexer;
mod parse;
mod source;
mod surface;
mod token;

pub use cst::{Cst, ElementRef, NodeKind, NodeRef, SyntaxError, TokenRef};
pub use parse::parse;
pub use source::{NotUtf8, SourceText, Span};
pub use surface::{BinaryOp, Origin, Syntax, UnaryOp, lower};
pub use token::{Token, TokenKind};
