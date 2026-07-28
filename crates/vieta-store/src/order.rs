//! The canonical order on terms (`docs/layer-a.md` §5).
//!
//! Sorting a commutative operator's arguments needs a total order, and it has to
//! be determined by content rather than by id. An id-ordered sort would break
//! three ways: a compacting collector renumbers ids (D8), so a stored argument
//! list would stop being sorted and the next construction of the same term would
//! intern a second node; ids differ between processes, so a store segment and
//! the store receiving it would disagree about canonical form (D23); and
//! canonical printed output would depend on the order symbols happened to be
//! interned in, which forecloses an implementation-independent conformance
//! suite.
//!
//! The order is not required to be mathematically conventional. The canonical
//! printer prints what it says and the pretty printer is free to present a sum
//! however it reads best, which is what D12's split already bought.

use core::cmp::Ordering;

use crate::node::{View, is_number, tag};
use crate::num::Num;

/// Numbers sort before symbols, which sort before applications.
fn kind(id: u32) -> u8 {
    if is_number(id) {
        0
    } else if tag(id) == crate::id::Tag::Symbol {
        1
    } else {
        2
    }
}

/// Compare two terms in the canonical order.
///
/// The comparison opens on id equality, which hash-consing makes the answer for
/// every shared subterm, so the recursive descent only runs where two terms
/// genuinely differ and stops at the first difference.
pub(crate) fn canonical_cmp(view: View<'_>, a: u32, b: u32) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    let (left, right) = (kind(a), kind(b));
    if left != right {
        return left.cmp(&right);
    }
    match left {
        0 => number(a).cmp(&number(b)),
        1 => {
            let (name_a, module_a) = symbol(view, a);
            let (name_b, module_b) = symbol(view, b);
            name_a.cmp(name_b).then_with(|| module_a.cmp(&module_b))
        }
        _ => {
            let node_a = view.node(a).expect("kind said application");
            let node_b = view.node(b).expect("kind said application");
            canonical_cmp(view, node_a.head, node_b.head)
                .then_with(|| node_a.arity.cmp(&node_b.arity))
                .then_with(|| {
                    let args_a = view.args_of(node_a);
                    let args_b = view.args_of(node_b);
                    for (&x, &y) in args_a.iter().zip(args_b) {
                        let ordering = canonical_cmp(view, x, y);
                        if ordering != Ordering::Equal {
                            return ordering;
                        }
                    }
                    Ordering::Equal
                })
        }
    }
}

fn number(id: u32) -> Num {
    Num::from_id(id).expect("kind said number")
}

fn symbol(view: View<'_>, id: u32) -> (&str, u32) {
    view.symbol(id).expect("kind said symbol")
}
