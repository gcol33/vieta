//! The passes Layer A runs from a declared canonical signature
//! (`docs/layer-a.md` §6).
//!
//! Each one reads interned structure and rewrites an argument list in place.
//! None of them builds a term, so none of them needs the store, which is what
//! lets them run while its interior is borrowed.

use crate::node::View;
use crate::order::canonical_cmp;

/// Splice arguments headed by the same operator into the parent.
///
/// One level is enough: the arguments are already normal, so an argument headed
/// by this operator has already had its own arguments spliced.
pub(crate) fn flatten(view: View<'_>, head: u32, list: &mut Vec<u32>) {
    if !list.iter().any(|&arg| view.is_headed_by(arg, head)) {
        return;
    }
    let mut flat = Vec::with_capacity(list.len() + 4);
    for &arg in list.iter() {
        if view.is_headed_by(arg, head) {
            flat.extend_from_slice(view.args_at(arg));
        } else {
            flat.push(arg);
        }
    }
    *list = flat;
}

/// Sort arguments into the canonical order.
pub(crate) fn sort_canonical(view: View<'_>, list: &mut [u32]) {
    list.sort_unstable_by(|&a, &b| canonical_cmp(view, a, b));
}

/// Drop duplicate arguments.
///
/// Adjacent duplicates only, which is the whole multiset once the list has been
/// sorted, and adjacent occurrences alone when the operator is not commutative.
pub(crate) fn dedupe(list: &mut Vec<u32>) {
    list.dedup();
}
