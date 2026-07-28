//! Interned application nodes, and read-only views over the structure.
//!
//! A [`View`] is what the normalization passes read while the store's interior
//! is borrowed. It carries no way to build a term, which is what keeps a pass
//! that needs to construct one from holding the borrow while it does.

use crate::id::{ExprId, Tag};
use crate::symbol::SymbolTable;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Node {
    pub(crate) head: u32,
    pub(crate) arity: u32,
    pub(crate) arg_offset: u32,
}

/// Read-only access to interned structure and symbol names.
#[derive(Clone, Copy)]
pub(crate) struct View<'a> {
    pub(crate) nodes: &'a [Node],
    pub(crate) args: &'a [u32],
    pub(crate) symbols: &'a SymbolTable,
}

impl<'a> View<'a> {
    /// The node an id denotes, or `None` when the id is an atom.
    pub(crate) fn node(self, id: u32) -> Option<Node> {
        if tag(id) == Tag::Node {
            Some(self.nodes[payload(id) as usize])
        } else {
            None
        }
    }

    pub(crate) fn args_of(self, node: Node) -> &'a [u32] {
        let start = node.arg_offset as usize;
        &self.args[start..start + node.arity as usize]
    }

    /// The arguments of an application, or an empty slice for an atom.
    pub(crate) fn args_at(self, id: u32) -> &'a [u32] {
        match self.node(id) {
            Some(node) => self.args_of(node),
            None => &[],
        }
    }

    /// The head of an application, or `None` for an atom.
    pub(crate) fn head_of(self, id: u32) -> Option<u32> {
        self.node(id).map(|node| node.head)
    }

    /// Whether this id is an application of that head.
    pub(crate) fn is_headed_by(self, id: u32, head: u32) -> bool {
        self.head_of(id) == Some(head)
    }

    /// The name and module of a symbol, or `None` for anything else.
    pub(crate) fn symbol(self, id: u32) -> Option<(&'a str, u32)> {
        if tag(id) != Tag::Symbol {
            return None;
        }
        let entry = payload(id);
        match (self.symbols.name(entry), self.symbols.module(entry)) {
            (Some(name), Some(module)) => Some((name, module)),
            _ => None,
        }
    }
}

pub(crate) fn tag(id: u32) -> Tag {
    ExprId::from_raw(id).tag()
}

pub(crate) fn payload(id: u32) -> u32 {
    ExprId::from_raw(id).payload()
}

/// Whether an id denotes an exact number.
pub(crate) fn is_number(id: u32) -> bool {
    matches!(
        tag(id),
        Tag::SmallInt | Tag::SmallRat | Tag::BigInt | Tag::BigRat
    )
}
