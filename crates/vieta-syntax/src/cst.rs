//! The lossless concrete syntax tree (D37).
//!
//! Every token the lexer produced is a leaf, trivia included, in source order.
//! Printing is therefore the concatenation of the leaves' source text, and the
//! non-synthetic leaves tile the source exactly, which makes the byte-exact
//! round trip a consequence of an invariant rather than a hope.

use crate::source::{SourceText, Span};
use crate::token::{Token, TokenKind};

/// What an interior node of the tree is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NodeKind {
    /// The whole source.
    Root,
    /// A number.
    Literal,
    /// A use of a name.
    NameRef,
    /// A parenthesized expression, which Syntax drops and the CST keeps.
    ParenExpr,
    /// A prefix operator applied to one operand.
    UnaryExpr,
    /// An infix operator applied to two operands.
    BinaryExpr,
    /// A callee applied to an argument list.
    CallExpr,
    /// The parenthesized arguments of a call.
    ArgList,
    /// `fn(x) => body`
    Lambda,
    /// `let x = value in body`
    Let,
    /// Input the parser could not place, kept so that it still prints.
    Error,
}

/// Something a parse could not make sense of.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SyntaxError {
    /// Where the problem is. Empty when something was expected and absent.
    pub span: Span,
    /// What went wrong, in one phrase.
    pub message: String,
}

#[derive(Clone, Copy)]
enum Child {
    Node(u32),
    Token(u32),
}

#[derive(Clone, Copy)]
struct NodeData {
    kind: NodeKind,
    first_child: u32,
    child_count: u32,
    span: Span,
}

/// A parsed source file, with nothing from it discarded.
#[derive(Clone)]
pub struct Cst {
    source: SourceText,
    tokens: Vec<Token>,
    nodes: Vec<NodeData>,
    children: Vec<Child>,
    root: u32,
    errors: Vec<SyntaxError>,
}

impl Cst {
    /// The source this was parsed from.
    pub fn source(&self) -> &SourceText {
        &self.source
    }

    /// The source text, reconstructed from the tree.
    ///
    /// Equal to the original source for every input the parser accepts,
    /// including malformed ones, because synthetic leaves carry empty spans.
    pub fn print(&self) -> String {
        let mut out = String::with_capacity(self.source.len() as usize);
        for token in &self.tokens {
            if let Some(text) = self.source.slice(token.span) {
                out.push_str(text);
            }
        }
        out
    }

    /// The outermost node.
    pub fn root(&self) -> NodeRef<'_> {
        NodeRef { cst: self, index: self.root }
    }

    /// Everything that went wrong, in source order.
    pub fn errors(&self) -> &[SyntaxError] {
        &self.errors
    }

    /// The leaves, in source order, trivia included.
    pub fn leaves(&self) -> impl Iterator<Item = TokenRef<'_>> {
        (0..self.tokens.len() as u32).map(|index| TokenRef { cst: self, index })
    }
}

/// An interior node.
#[derive(Clone, Copy)]
pub struct NodeRef<'c> {
    cst: &'c Cst,
    index: u32,
}

impl<'c> NodeRef<'c> {
    /// What this node is.
    pub fn kind(self) -> NodeKind {
        self.data().kind
    }

    /// The source this node covers, ignoring the synthetic leaves inside it.
    pub fn span(self) -> Span {
        self.data().span
    }

    /// The children, in source order.
    pub fn children(self) -> impl Iterator<Item = ElementRef<'c>> {
        let data = self.data();
        let start = data.first_child as usize;
        let end = start + data.child_count as usize;
        let cst = self.cst;
        cst.children[start..end].iter().map(move |&child| match child {
            Child::Node(index) => ElementRef::Node(NodeRef { cst, index }),
            Child::Token(index) => ElementRef::Token(TokenRef { cst, index }),
        })
    }

    /// The child nodes, skipping leaves.
    pub fn child_nodes(self) -> impl Iterator<Item = NodeRef<'c>> {
        self.children().filter_map(|child| match child {
            ElementRef::Node(node) => Some(node),
            ElementRef::Token(_) => None,
        })
    }

    /// The child leaves that carry meaning, skipping trivia.
    pub fn child_tokens(self) -> impl Iterator<Item = TokenRef<'c>> {
        self.children().filter_map(|child| match child {
            ElementRef::Token(token) if !token.kind().is_trivia() => Some(token),
            _ => None,
        })
    }

    fn data(self) -> NodeData {
        self.cst.nodes[self.index as usize]
    }
}

/// A leaf.
#[derive(Clone, Copy)]
pub struct TokenRef<'c> {
    cst: &'c Cst,
    index: u32,
}

impl<'c> TokenRef<'c> {
    /// What this leaf is.
    pub fn kind(self) -> TokenKind {
        self.token().kind
    }

    /// Where it sits in the source.
    pub fn span(self) -> Span {
        self.token().span
    }

    /// Its source text, empty when error recovery inserted it.
    pub fn text(self) -> &'c str {
        self.cst.source.slice(self.token().span).unwrap_or("")
    }

    /// Whether error recovery inserted it.
    pub fn is_synthetic(self) -> bool {
        self.token().is_synthetic()
    }

    fn token(self) -> Token {
        self.cst.tokens[self.index as usize]
    }
}

/// A child of a node.
#[derive(Clone, Copy)]
pub enum ElementRef<'c> {
    /// An interior node.
    Node(NodeRef<'c>),
    /// A leaf.
    Token(TokenRef<'c>),
}

pub(crate) struct Builder {
    tokens: Vec<Token>,
    nodes: Vec<NodeData>,
    children: Vec<Child>,
    scratch: Vec<Child>,
    open: Vec<(NodeKind, usize)>,
}

impl Builder {
    pub(crate) fn new() -> Builder {
        Builder {
            tokens: Vec::new(),
            nodes: Vec::new(),
            children: Vec::new(),
            scratch: Vec::new(),
            open: Vec::new(),
        }
    }

    /// A position to which a node can be opened later, once the parser knows
    /// that what it has already built is the left operand of something.
    pub(crate) fn checkpoint(&self) -> usize {
        self.scratch.len()
    }

    pub(crate) fn start(&mut self, kind: NodeKind) {
        let at = self.scratch.len();
        self.open.push((kind, at));
    }

    pub(crate) fn start_at(&mut self, checkpoint: usize, kind: NodeKind) {
        self.open.push((kind, checkpoint));
    }

    pub(crate) fn token(&mut self, token: Token) {
        let index = self.tokens.len() as u32;
        self.tokens.push(token);
        self.scratch.push(Child::Token(index));
    }

    pub(crate) fn finish(&mut self) {
        let (kind, start) = self.open.pop().expect("a node is open");
        let taken: Vec<Child> = self.scratch.drain(start..).collect();
        let span = self.cover(&taken);
        let first_child = self.children.len() as u32;
        let child_count = taken.len() as u32;
        self.children.extend(taken);
        let index = self.nodes.len() as u32;
        self.nodes.push(NodeData { kind, first_child, child_count, span });
        self.scratch.push(Child::Node(index));
    }

    pub(crate) fn build(mut self, source: SourceText, errors: Vec<SyntaxError>) -> Cst {
        assert!(self.open.is_empty(), "every node was closed");
        let root = match self.scratch.pop() {
            Some(Child::Node(index)) => index,
            _ => unreachable!("the parser closes a root node"),
        };
        self.scratch.clear();
        Cst { source, tokens: self.tokens, nodes: self.nodes, children: self.children, root, errors }
    }

    /// The smallest span covering the non-synthetic children, so that a node
    /// built entirely out of recovery does not claim source it never had.
    fn cover(&self, taken: &[Child]) -> Span {
        let spans = taken.iter().map(|&child| match child {
            Child::Node(index) => self.nodes[index as usize].span,
            Child::Token(index) => self.tokens[index as usize].span,
        });
        let mut covered: Option<Span> = None;
        let mut first: Option<Span> = None;
        for span in spans {
            first.get_or_insert(span);
            if !span.is_empty() {
                covered = Some(match covered {
                    Some(sofar) => sofar.cover(span),
                    None => span,
                });
            }
        }
        covered.or(first).unwrap_or(Span::empty_at(0))
    }
}
