//! Surface syntax: the semantically shaped form elaboration reads (D37).
//!
//! Grouping is gone here, because parentheses are a fact about the source and
//! not about the program: `(x + y)` and `x + y` are one Syntax and two
//! different concrete trees. Trivia is gone for the same reason. What survives
//! is names, structure, binding forms, and an origin on every node, because
//! diagnostics, hygiene, and derivations all need to say where something came
//! from and none of them can recover it later.

use crate::cst::{Cst, ElementRef, NodeKind, NodeRef};
use crate::source::Span;
use crate::token::TokenKind;

/// Where a node came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Origin {
    /// Written in the source at this span.
    Source(Span),
    /// Supplied by error recovery. The span is where it was expected.
    Recovered(Span),
    /// Produced by expanding a macro at this call site. Reserved for D29.
    Expanded(Span),
}

impl Origin {
    /// The source position, whatever the reason for it.
    pub fn span(self) -> Span {
        match self {
            Origin::Source(span) | Origin::Recovered(span) | Origin::Expanded(span) => span,
        }
    }
}

/// An infix operator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinaryOp {
    /// `+`
    Add,
    /// `-`
    Subtract,
    /// `*`
    Multiply,
    /// `/`
    Divide,
    /// `^`
    Power,
}

/// A prefix operator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnaryOp {
    /// `-`
    Negate,
}

/// A surface expression.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Syntax {
    /// A use of a name.
    Name {
        /// The name as written.
        text: Box<str>,
        /// Where it came from.
        origin: Origin,
    },
    /// A number, kept as written until elaboration reads it.
    Number {
        /// The literal as written, including separators and trailing zeros.
        text: Box<str>,
        /// Where it came from.
        origin: Origin,
    },
    /// A prefix operator applied to an operand.
    Unary {
        /// Which operator.
        op: UnaryOp,
        /// What it applies to.
        operand: Box<Syntax>,
        /// Where it came from.
        origin: Origin,
    },
    /// An infix operator applied to two operands.
    Binary {
        /// Which operator.
        op: BinaryOp,
        /// The left operand.
        left: Box<Syntax>,
        /// The right operand.
        right: Box<Syntax>,
        /// Where it came from.
        origin: Origin,
    },
    /// A callee applied to arguments.
    Call {
        /// What is being called.
        callee: Box<Syntax>,
        /// The arguments, in order.
        arguments: Vec<Syntax>,
        /// Where it came from.
        origin: Origin,
    },
    /// `fn(x) => body`
    Lambda {
        /// The bound name, as written.
        parameter: Box<str>,
        /// The body, in whose scope the parameter is bound.
        body: Box<Syntax>,
        /// Where it came from.
        origin: Origin,
    },
    /// `let x = value in body`
    Let {
        /// The bound name, as written.
        name: Box<str>,
        /// What the name is bound to.
        value: Box<Syntax>,
        /// The body, in whose scope the name is bound.
        body: Box<Syntax>,
        /// Where it came from.
        origin: Origin,
    },
    /// `term { expression }`
    Quote {
        /// The quoted expression, which elaboration reads symbolically.
        body: Box<Syntax>,
        /// Where it came from.
        origin: Origin,
    },
    /// Input that did not parse.
    Error {
        /// Where it came from.
        origin: Origin,
    },
}

impl Syntax {
    /// Where this node came from.
    pub fn origin(&self) -> Origin {
        match self {
            Syntax::Name { origin, .. }
            | Syntax::Number { origin, .. }
            | Syntax::Unary { origin, .. }
            | Syntax::Binary { origin, .. }
            | Syntax::Call { origin, .. }
            | Syntax::Lambda { origin, .. }
            | Syntax::Let { origin, .. }
            | Syntax::Quote { origin, .. }
            | Syntax::Error { origin } => *origin,
        }
    }

    /// The same expression with a different origin, which is what lowering
    /// uses when a node it absorbs carries provenance the survivor must keep.
    pub fn with_origin(self, origin: Origin) -> Syntax {
        match self {
            Syntax::Name { text, .. } => Syntax::Name { text, origin },
            Syntax::Number { text, .. } => Syntax::Number { text, origin },
            Syntax::Unary { op, operand, .. } => Syntax::Unary { op, operand, origin },
            Syntax::Binary { op, left, right, .. } => {
                Syntax::Binary { op, left, right, origin }
            }
            Syntax::Call { callee, arguments, .. } => {
                Syntax::Call { callee, arguments, origin }
            }
            Syntax::Lambda { parameter, body, .. } => {
                Syntax::Lambda { parameter, body, origin }
            }
            Syntax::Let { name, value, body, .. } => Syntax::Let { name, value, body, origin },
            Syntax::Quote { body, .. } => Syntax::Quote { body, origin },
            Syntax::Error { .. } => Syntax::Error { origin },
        }
    }

    /// Whether two expressions are the same program, ignoring where each node
    /// came from. `(x + y)` and `x + y` answer yes.
    pub fn same_shape(&self, other: &Syntax) -> bool {
        match (self, other) {
            (Syntax::Name { text: a, .. }, Syntax::Name { text: b, .. }) => a == b,
            (Syntax::Number { text: a, .. }, Syntax::Number { text: b, .. }) => a == b,
            (
                Syntax::Unary { op: a, operand: x, .. },
                Syntax::Unary { op: b, operand: y, .. },
            ) => a == b && x.same_shape(y),
            (
                Syntax::Binary { op: a, left: al, right: ar, .. },
                Syntax::Binary { op: b, left: bl, right: br, .. },
            ) => a == b && al.same_shape(bl) && ar.same_shape(br),
            (
                Syntax::Call { callee: a, arguments: ax, .. },
                Syntax::Call { callee: b, arguments: bx, .. },
            ) => {
                a.same_shape(b)
                    && ax.len() == bx.len()
                    && ax.iter().zip(bx).all(|(x, y)| x.same_shape(y))
            }
            (
                Syntax::Lambda { parameter: a, body: x, .. },
                Syntax::Lambda { parameter: b, body: y, .. },
            ) => a == b && x.same_shape(y),
            (
                Syntax::Let { name: a, value: av, body: ab, .. },
                Syntax::Let { name: b, value: bv, body: bb, .. },
            ) => a == b && av.same_shape(bv) && ab.same_shape(bb),
            (Syntax::Quote { body: a, .. }, Syntax::Quote { body: b, .. }) => a.same_shape(b),
            (Syntax::Error { .. }, Syntax::Error { .. }) => true,
            _ => false,
        }
    }
}

/// Lower a concrete tree to surface syntax.
pub fn lower(cst: &Cst) -> Syntax {
    match cst.root().child_nodes().next() {
        Some(node) => lower_node(node),
        None => Syntax::Error { origin: Origin::Recovered(cst.root().span()) },
    }
}

fn lower_node(node: NodeRef<'_>) -> Syntax {
    let origin = origin_of(node);
    match node.kind() {
        NodeKind::Literal => Syntax::Number { text: first_text(node), origin },
        NodeKind::NameRef => Syntax::Name { text: first_text(node), origin },
        NodeKind::ParenExpr => match node.child_nodes().next() {
            // The grouping disappears here, so anything the parser had to
            // supply for it moves to the expression that survives.
            Some(inner) => {
                let lowered = lower_node(inner);
                if supplied(node) {
                    let span = lowered.origin().span();
                    lowered.with_origin(Origin::Recovered(span))
                } else {
                    lowered
                }
            }
            None => Syntax::Error { origin },
        },
        NodeKind::UnaryExpr => Syntax::Unary {
            op: UnaryOp::Negate,
            operand: Box::new(child_or_error(node, 0, origin)),
            origin,
        },
        NodeKind::BinaryExpr => {
            match node.child_tokens().find_map(|token| binary_op(token.kind())) {
                Some(op) => Syntax::Binary {
                    op,
                    left: Box::new(child_or_error(node, 0, origin)),
                    right: Box::new(child_or_error(node, 1, origin)),
                    origin,
                },
                None => Syntax::Error { origin },
            }
        }
        NodeKind::CallExpr => {
            let mut nodes = node.child_nodes();
            let callee = nodes.next().map_or(Syntax::Error { origin }, lower_node);
            let list = nodes.find(|child| child.kind() == NodeKind::ArgList);
            let arguments = list
                .map(|list| list.child_nodes().map(lower_node).collect())
                .unwrap_or_default();
            // The argument list disappears here, so its provenance moves to the
            // call that absorbs it.
            let origin = match list {
                Some(list) if supplied(list) => Origin::Recovered(node.span()),
                _ => origin,
            };
            Syntax::Call { callee: Box::new(callee), arguments, origin }
        }
        NodeKind::Lambda => Syntax::Lambda {
            parameter: bound_name(node),
            body: Box::new(child_or_error(node, 0, origin)),
            origin,
        },
        NodeKind::Let => Syntax::Let {
            name: bound_name(node),
            value: Box::new(child_or_error(node, 0, origin)),
            body: Box::new(child_or_error(node, 1, origin)),
            origin,
        },
        NodeKind::Quote => Syntax::Quote {
            body: Box::new(child_or_error(node, 0, origin)),
            origin,
        },
        NodeKind::Root | NodeKind::ArgList | NodeKind::Error => Syntax::Error { origin },
    }
}

/// Whether the parser had to insert a leaf into this node to finish it.
fn supplied(node: NodeRef<'_>) -> bool {
    node.children()
        .any(|child| matches!(child, ElementRef::Token(token) if token.is_synthetic()))
}

/// A node is recovered when the parser had to supply part of it, which is what
/// a diagnostic needs to distinguish from what the author wrote.
fn origin_of(node: NodeRef<'_>) -> Origin {
    if supplied(node) || node.kind() == NodeKind::Error {
        Origin::Recovered(node.span())
    } else {
        Origin::Source(node.span())
    }
}

fn first_text(node: NodeRef<'_>) -> Box<str> {
    node.child_tokens().next().map_or_else(|| "".into(), |token| token.text().into())
}

fn bound_name(node: NodeRef<'_>) -> Box<str> {
    node.child_tokens()
        .find(|token| token.kind() == TokenKind::Ident)
        .map_or_else(|| "".into(), |token| token.text().into())
}

fn child_or_error(node: NodeRef<'_>, index: usize, origin: Origin) -> Syntax {
    node.child_nodes()
        .nth(index)
        .map_or(Syntax::Error { origin: Origin::Recovered(origin.span()) }, lower_node)
}

fn binary_op(kind: TokenKind) -> Option<BinaryOp> {
    match kind {
        TokenKind::Plus => Some(BinaryOp::Add),
        TokenKind::Minus => Some(BinaryOp::Subtract),
        TokenKind::Star => Some(BinaryOp::Multiply),
        TokenKind::Slash => Some(BinaryOp::Divide),
        TokenKind::Caret => Some(BinaryOp::Power),
        _ => None,
    }
}
