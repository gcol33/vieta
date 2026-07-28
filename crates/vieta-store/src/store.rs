//! The interned term store, and Layer A normalization at construction.

use core::cell::RefCell;

use crate::arith::{self, Outcome};
use crate::cancel::{CancelToken, Cancelled};
use crate::hash::{mix, seed};
use crate::id::{
    ExprId, MAX_PAYLOAD, Tag, decode_small_int, decode_small_rat, encode_small_int,
    encode_small_rat, gcd,
};
use crate::node::{Node, View, payload, tag};
use crate::normalize;
use crate::operator::{CanonicalSignature, ModuleId, RawSignature, SignatureConflict};
use crate::probe::{Probe, ProbeTable};
use crate::symbol::SymbolTable;

/// Arguments up to this arity are staged on the stack while interning.
const INLINE_ARGS: usize = 8;

#[derive(Default)]
struct Inner {
    nodes: Vec<Node>,
    args: Vec<u32>,
    table: ProbeTable,
    symbols: SymbolTable,
}

/// The operator identities kernel arithmetic is attached to, and the exponent
/// `x^1 -> x` tests for.
#[derive(Clone, Copy)]
pub(crate) struct Kernel {
    pub(crate) plus: u32,
    pub(crate) times: u32,
    pub(crate) power: u32,
    pub(crate) one: u32,
}

/// A hash-consed store of expressions.
///
/// Every application built through [`Store::app`] is normalized by Layer A and
/// then interned, so equality of the returned [`ExprId`] is equality modulo the
/// canonical-shape laws of the operators involved. Ids borrow the store, which
/// is what keeps them from outliving a [`safepoint`](Store::safepoint).
pub struct Store {
    inner: RefCell<Inner>,
    kernel: Kernel,
    cancel: RefCell<Option<CancelToken>>,
}

/// A measurement of what the store is holding.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StoreStats {
    /// Interned applications.
    pub nodes: usize,
    /// Distinct symbols.
    pub symbols: usize,
    /// Words in the flat argument array.
    pub arg_words: usize,
    /// Slots in the intern table, occupied or not.
    pub table_slots: usize,
    /// Bytes of heap the store has reserved, which is what resident memory
    /// pays. Growth doubles, so this overshoots what the data needs.
    pub reserved_bytes: usize,
    /// Bytes of heap the store's data occupies, which is what the layout costs.
    pub used_bytes: usize,
}

impl StoreStats {
    fn per_node(&self, bytes: usize) -> f64 {
        if self.nodes == 0 {
            0.0
        } else {
            bytes as f64 / self.nodes as f64
        }
    }

    /// Reserved heap divided by interned nodes.
    pub fn reserved_bytes_per_node(&self) -> f64 {
        self.per_node(self.reserved_bytes)
    }

    /// Occupied heap divided by interned nodes, which is the layout number
    /// §1.9 asks for.
    pub fn used_bytes_per_node(&self) -> f64 {
        self.per_node(self.used_bytes)
    }
}

fn hash_key(head: u32, args: &[u32]) -> u32 {
    let mut hash = mix(seed(), head);
    hash = mix(hash, args.len() as u32);
    for &arg in args {
        hash = mix(hash, arg);
    }
    hash
}

fn node_args(argbuf: &[u32], node: Node) -> &[u32] {
    let start = node.arg_offset as usize;
    &argbuf[start..start + node.arity as usize]
}

fn node_eq(nodes: &[Node], argbuf: &[u32], entry: u32, head: u32, args: &[u32]) -> bool {
    let node = nodes[entry as usize];
    node.head == head && node.arity as usize == args.len() && node_args(argbuf, node) == args
}

fn small_int(value: i64) -> u32 {
    let payload = encode_small_int(value).expect("the kernel's constants are small");
    ExprId::from_parts(Tag::SmallInt, payload).bits()
}

impl Inner {
    fn view(&self) -> View<'_> {
        View { nodes: &self.nodes, args: &self.args, symbols: &self.symbols }
    }

    fn intern(&mut self, head: u32, args: &[u32]) -> u32 {
        let hash = hash_key(head, args);
        let probe = {
            let nodes = &self.nodes;
            let argbuf = &self.args;
            self.table
                .probe(hash, |entry| node_eq(nodes, argbuf, entry, head, args))
        };
        let entry = match probe {
            Probe::Found(entry) => entry,
            Probe::Vacant(slot) => {
                let arg_offset = self.args.len() as u32;
                self.args.extend_from_slice(args);
                let entry = self.nodes.len() as u32;
                assert!(entry <= MAX_PAYLOAD, "node table exhausted the id space");
                self.nodes.push(Node { head, arity: args.len() as u32, arg_offset });
                self.table.occupy(slot, entry);
                if self.table.needs_grow() {
                    let nodes = &self.nodes;
                    let argbuf = &self.args;
                    self.table.grow(|entry| {
                        let node = nodes[entry as usize];
                        hash_key(node.head, node_args(argbuf, node))
                    });
                }
                entry
            }
        };
        ExprId::from_parts(Tag::Node, entry).bits()
    }

    fn intern_symbol(&mut self, module: ModuleId, name: &str) -> u32 {
        let entry = self.symbols.intern(module.0, name);
        assert!(entry <= MAX_PAYLOAD, "symbol table exhausted the id space");
        ExprId::from_parts(Tag::Symbol, entry).bits()
    }
}

impl Default for Store {
    fn default() -> Self {
        Store::new()
    }
}

impl Store {
    /// A store holding the kernel's operators and nothing else.
    pub fn new() -> Self {
        let mut inner = Inner::default();
        let zero = small_int(0);
        let one = small_int(1);
        let kernel = Kernel {
            plus: kernel_operator(
                &mut inner,
                "Plus",
                RawSignature {
                    associative: true,
                    commutative: true,
                    idempotent: false,
                    unit: Some(zero),
                    zero: None,
                },
            ),
            times: kernel_operator(
                &mut inner,
                "Times",
                RawSignature {
                    associative: true,
                    commutative: true,
                    idempotent: false,
                    unit: Some(one),
                    zero: Some(zero),
                },
            ),
            power: kernel_operator(&mut inner, "Power", RawSignature::EMPTY),
            one,
        };
        Store { inner: RefCell::new(inner), kernel, cancel: RefCell::new(None) }
    }

    /// Install a cancellation token, returning the one it replaces.
    ///
    /// While a cancelled token is installed, [`app`](Store::app) refuses to
    /// build anything, which stops a rewrite loop between terms rather than
    /// waiting for it to return.
    pub fn set_cancel(&self, token: Option<CancelToken>) -> Option<CancelToken> {
        self.cancel.replace(token)
    }

    /// The symbol with this name in the kernel's module, interning it if it is
    /// new.
    pub fn symbol(&self, name: &str) -> ExprId<'_> {
        self.symbol_in(ModuleId::CORE, name)
    }

    /// The symbol with this module and name, interning it if it is new.
    ///
    /// A module and a name together identify an operator (D36), so the same
    /// name in two modules is two operators with independent laws.
    pub fn symbol_in(&self, module: ModuleId, name: &str) -> ExprId<'_> {
        ExprId::from_raw(self.inner.borrow_mut().intern_symbol(module, name))
    }

    /// Declare an operator's canonical-shape laws.
    ///
    /// The laws are part of the operator's identity and are fixed once (D36).
    /// Declaring the same laws again is a no-op, which is what makes reloading
    /// a module harmless. Declaring different laws, including declaring any
    /// laws for an operator that terms have already been built with, is a
    /// [`SignatureConflict`].
    pub fn declare(
        &self,
        module: ModuleId,
        name: &str,
        signature: CanonicalSignature<'_>,
    ) -> Result<ExprId<'_>, SignatureConflict> {
        let mut borrow = self.inner.borrow_mut();
        let id = borrow.intern_symbol(module, name);
        borrow.symbols.fix(payload(id), signature.into_raw())?;
        Ok(ExprId::from_raw(id))
    }

    /// The canonical-shape laws fixed for an operator, or `None` when the id is
    /// not a symbol or its laws are still open.
    pub fn signature(&self, id: ExprId<'_>) -> Option<CanonicalSignature<'_>> {
        if tag(id.bits()) != Tag::Symbol {
            return None;
        }
        self.inner
            .borrow()
            .symbols
            .signature(id.payload())
            .map(RawSignature::into_signature)
    }

    /// The kernel's addition.
    pub fn plus(&self) -> ExprId<'_> {
        ExprId::from_raw(self.kernel.plus)
    }

    /// The kernel's multiplication.
    pub fn times(&self) -> ExprId<'_> {
        ExprId::from_raw(self.kernel.times)
    }

    /// The kernel's exponentiation.
    pub fn power(&self) -> ExprId<'_> {
        ExprId::from_raw(self.kernel.power)
    }

    /// An exact integer, or `None` when it needs the side table that arrives
    /// with M1.
    pub fn int(&self, value: i64) -> Option<ExprId<'_>> {
        encode_small_int(value).map(|payload| ExprId::from_parts(Tag::SmallInt, payload))
    }

    /// An exact rational, reduced to lowest terms with a positive denominator.
    /// A denominator of one yields an integer. `None` when the reduced value
    /// needs the side table that arrives with M1, or when the denominator is
    /// zero.
    pub fn rat(&self, numerator: i64, denominator: i64) -> Option<ExprId<'_>> {
        if denominator == 0 {
            return None;
        }
        let (numerator, denominator) = if denominator < 0 {
            (numerator.checked_neg()?, denominator.checked_neg()?)
        } else {
            (numerator, denominator)
        };
        if numerator == 0 {
            return self.int(0);
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator as u64);
        let reduced_num = numerator / divisor as i64;
        let reduced_den = denominator as u64 / divisor;
        if reduced_den == 1 {
            return self.int(reduced_num);
        }
        encode_small_rat(reduced_num, reduced_den)
            .map(|payload| ExprId::from_parts(Tag::SmallRat, payload))
    }

    /// The application of `head` to `args`, normalized by Layer A and interned.
    ///
    /// The head is an ordinary id, so a computed head costs no more than a
    /// symbolic one. Using a symbol as a head fixes its canonical signature,
    /// which is why a declaration afterwards conflicts (D36).
    ///
    /// Fails only when the store's cancellation token is set. Layer A is total,
    /// so a term it cannot normalize is interned as written.
    pub fn app(&self, head: ExprId<'_>, args: &[ExprId<'_>]) -> Result<ExprId<'_>, Cancelled> {
        let mut inline = [0u32; INLINE_ARGS];
        let mut spill = Vec::new();
        let bits: &[u32] = if args.len() <= INLINE_ARGS {
            for (slot, arg) in inline.iter_mut().zip(args) {
                *slot = arg.bits();
            }
            &inline[..args.len()]
        } else {
            spill.extend(args.iter().map(|arg| arg.bits()));
            &spill
        };
        self.app_raw(head.bits(), bits).map(ExprId::from_raw)
    }

    pub(crate) fn app_raw(&self, head: u32, args: &[u32]) -> Result<u32, Cancelled> {
        self.check_cancel()?;
        let signature = self.fix_head_signature(head);
        let kernel_head = self.is_kernel_head(head);
        if signature.is_empty() && !kernel_head {
            return Ok(self.inner.borrow_mut().intern(head, args));
        }

        let mut list = args.to_vec();
        if signature.associative {
            let borrow = self.inner.borrow();
            normalize::flatten(borrow.view(), head, &mut list);
        }
        if let Some(id) = self.absorbed(&signature, &list) {
            return Ok(id);
        }
        self.drop_units(&signature, &mut list);
        if signature.commutative {
            self.sort(&mut list);
        }

        if kernel_head {
            match arith::normalize(self, head, list)? {
                Outcome::Done(id) => return Ok(id),
                Outcome::Args(next) => list = next,
            }
            if let Some(id) = self.absorbed(&signature, &list) {
                return Ok(id);
            }
            self.drop_units(&signature, &mut list);
            if signature.commutative {
                self.sort(&mut list);
            }
        }

        if signature.idempotent {
            normalize::dedupe(&mut list);
        }
        if signature.associative {
            if list.len() == 1 {
                return Ok(list[0]);
            }
            if list.is_empty() {
                if let Some(unit) = signature.unit {
                    return Ok(unit);
                }
            }
        }
        Ok(self.inner.borrow_mut().intern(head, &list))
    }

    pub(crate) fn kernel(&self) -> Kernel {
        self.kernel
    }

    pub(crate) fn with_view<R>(&self, f: impl FnOnce(View<'_>) -> R) -> R {
        let borrow = self.inner.borrow();
        f(borrow.view())
    }

    fn check_cancel(&self) -> Result<(), Cancelled> {
        match self.cancel.borrow().as_ref() {
            Some(token) if token.is_cancelled() => Err(Cancelled),
            _ => Ok(()),
        }
    }

    fn is_kernel_head(&self, head: u32) -> bool {
        head == self.kernel.plus || head == self.kernel.times || head == self.kernel.power
    }

    /// The signature of a head, fixing the empty one when this is the first use
    /// of a symbol as a head.
    fn fix_head_signature(&self, head: u32) -> RawSignature {
        if tag(head) != Tag::Symbol {
            return RawSignature::EMPTY;
        }
        self.inner.borrow_mut().symbols.fix_empty(payload(head))
    }

    /// The annihilator, when one of the arguments is it.
    fn absorbed(&self, signature: &RawSignature, list: &[u32]) -> Option<u32> {
        let zero = signature.zero?;
        list.contains(&zero).then_some(zero)
    }

    fn drop_units(&self, signature: &RawSignature, list: &mut Vec<u32>) {
        if let Some(unit) = signature.unit {
            list.retain(|&arg| arg != unit);
        }
    }

    fn sort(&self, list: &mut [u32]) {
        let borrow = self.inner.borrow();
        normalize::sort_canonical(borrow.view(), list);
    }

    /// Whether this id denotes something with no arguments.
    pub fn is_atom(&self, id: ExprId<'_>) -> bool {
        id.tag() != Tag::Node
    }

    /// The head of an application, or `None` for an atom.
    pub fn head(&self, id: ExprId<'_>) -> Option<ExprId<'_>> {
        if id.tag() != Tag::Node {
            return None;
        }
        Some(ExprId::from_raw(
            self.inner.borrow().nodes[id.payload() as usize].head,
        ))
    }

    /// How many arguments an application has. Atoms have none.
    pub fn arity(&self, id: ExprId<'_>) -> u32 {
        if id.tag() != Tag::Node {
            return 0;
        }
        self.inner.borrow().nodes[id.payload() as usize].arity
    }

    /// One argument of an application, or `None` when the index is past the
    /// arity.
    pub fn arg(&self, id: ExprId<'_>, index: u32) -> Option<ExprId<'_>> {
        if id.tag() != Tag::Node {
            return None;
        }
        let borrow = self.inner.borrow();
        let node = borrow.nodes[id.payload() as usize];
        if index >= node.arity {
            return None;
        }
        Some(ExprId::from_raw(
            borrow.args[node.arg_offset as usize + index as usize],
        ))
    }

    /// Every argument of an application, in order.
    pub fn collect_args(&self, id: ExprId<'_>) -> Vec<ExprId<'_>> {
        if id.tag() != Tag::Node {
            return Vec::new();
        }
        let borrow = self.inner.borrow();
        let node = borrow.nodes[id.payload() as usize];
        node_args(&borrow.args, node)
            .iter()
            .map(|&bits| ExprId::from_raw(bits))
            .collect()
    }

    /// The integer this id denotes, or `None` if it denotes something else.
    pub fn as_int(&self, id: ExprId<'_>) -> Option<i64> {
        match id.tag() {
            Tag::SmallInt => Some(decode_small_int(id.payload())),
            _ => None,
        }
    }

    /// The rational this id denotes as a numerator and positive denominator, or
    /// `None` if it denotes something else. Integers are not rationals here;
    /// they carry the `SmallInt` tag and answer [`as_int`](Store::as_int).
    pub fn as_rat(&self, id: ExprId<'_>) -> Option<(i64, u64)> {
        match id.tag() {
            Tag::SmallRat => Some(decode_small_rat(id.payload())),
            _ => None,
        }
    }

    /// Call `f` with the name of a symbol, or return `None` for anything else.
    ///
    /// The name is passed to a callback rather than returned because it lives in
    /// the store's text arena.
    pub fn with_symbol_name<R>(&self, id: ExprId<'_>, f: impl FnOnce(&str) -> R) -> Option<R> {
        if id.tag() != Tag::Symbol {
            return None;
        }
        let borrow = self.inner.borrow();
        borrow.symbols.name(id.payload()).map(f)
    }

    /// Every interned application, in construction order.
    ///
    /// Atoms are not included: they have no node and are reachable only from the
    /// applications that mention them.
    pub fn node_ids(&self) -> impl Iterator<Item = ExprId<'_>> {
        let count = self.inner.borrow().nodes.len() as u32;
        (0..count).map(|entry| ExprId::from_parts(Tag::Node, entry))
    }

    /// What the store is currently holding.
    pub fn stats(&self) -> StoreStats {
        let borrow = self.inner.borrow();
        let reserved_bytes = borrow.nodes.capacity() * size_of::<Node>()
            + borrow.args.capacity() * size_of::<u32>()
            + borrow.table.heap_bytes()
            + borrow.symbols.reserved_bytes();
        let used_bytes = borrow.nodes.len() * size_of::<Node>()
            + borrow.args.len() * size_of::<u32>()
            + borrow.table.heap_bytes()
            + borrow.symbols.used_bytes();
        StoreStats {
            nodes: borrow.nodes.len(),
            symbols: borrow.symbols.len(),
            arg_words: borrow.args.len(),
            table_slots: borrow.table.slot_count(),
            reserved_bytes,
            used_bytes,
        }
    }

    /// A point at which ids may be renumbered (D8).
    ///
    /// Reclamation is not implemented yet, so the body is empty and the
    /// signature is the content: taking `&mut self` means no [`ExprId`] handed
    /// out by this store can still be alive, because every id borrows it. D8's
    /// rule is a compile error rather than a convention.
    ///
    /// Ids that end before the safepoint are fine:
    ///
    /// ```
    /// let mut store = vieta_store::Store::new();
    /// let bits = {
    ///     let x = store.symbol("x");
    ///     x.bits()
    /// };
    /// store.safepoint();
    /// assert_ne!(bits, 0);
    /// ```
    ///
    /// An id held across one does not compile:
    ///
    /// ```compile_fail
    /// let mut store = vieta_store::Store::new();
    /// let x = store.symbol("x");
    /// store.safepoint();
    /// let _ = x.bits();
    /// ```
    pub fn safepoint(&mut self) {}
}

fn kernel_operator(inner: &mut Inner, name: &str, signature: RawSignature) -> u32 {
    let id = inner.intern_symbol(ModuleId::CORE, name);
    inner
        .symbols
        .fix(payload(id), signature)
        .expect("the kernel declares each of its operators once");
    id
}

#[cfg(test)]
mod tests {
    use super::Store;
    use crate::cancel::{CancelToken, Cancelled};
    use crate::id::{SMALL_INT_MAX, SMALL_INT_MIN, Tag};
    use crate::operator::{CanonicalSignature, ModuleId};

    #[test]
    fn equal_applications_are_one_id() {
        let store = Store::new();
        let f = store.symbol("f");
        let a = store.symbol("a");
        let b = store.symbol("b");
        assert_eq!(store.app(f, &[a, b]), store.app(f, &[a, b]));
    }

    #[test]
    fn argument_order_distinguishes_applications() {
        let store = Store::new();
        let f = store.symbol("f");
        let a = store.symbol("a");
        let b = store.symbol("b");
        assert_ne!(store.app(f, &[a, b]), store.app(f, &[b, a]));
    }

    #[test]
    fn arity_distinguishes_applications() {
        let store = Store::new();
        let f = store.symbol("f");
        let a = store.symbol("a");
        assert_ne!(store.app(f, &[]), store.app(f, &[a]));
        assert_ne!(store.app(f, &[a]), store.app(f, &[a, a]));
    }

    #[test]
    fn heads_are_ordinary_ids() {
        let store = Store::new();
        let d = store.symbol("Derivative");
        let f = store.symbol("f");
        let one = store.int(1).expect("in range");
        let outer = store.app(d, &[one]).expect("not cancelled");
        let applied = store.app(outer, &[f]).expect("not cancelled");
        assert_eq!(store.head(applied), Some(outer));
        assert_eq!(store.head(outer), Some(d));
        assert_eq!(store.head(d), None);
    }

    #[test]
    fn structure_reads_back() {
        let store = Store::new();
        let f = store.symbol("f");
        let a = store.symbol("a");
        let b = store.symbol("b");
        let term = store.app(f, &[a, b]).expect("not cancelled");
        assert_eq!(store.arity(term), 2);
        assert_eq!(store.arg(term, 0), Some(a));
        assert_eq!(store.arg(term, 1), Some(b));
        assert_eq!(store.arg(term, 2), None);
        assert_eq!(store.collect_args(term), vec![a, b]);
        assert!(!store.is_atom(term));
        assert!(store.is_atom(a));
    }

    #[test]
    fn large_arities_intern_like_small_ones() {
        let store = Store::new();
        let f = store.symbol("f");
        let args: Vec<_> = (0..64).map(|i| store.int(i).expect("in range")).collect();
        let first = store.app(f, &args).expect("not cancelled");
        let second = store.app(f, &args).expect("not cancelled");
        assert_eq!(first, second);
        assert_eq!(store.arity(first), 64);
        assert_eq!(store.arg(first, 63), Some(args[63]));
        assert_eq!(store.stats().nodes, 1);
    }

    #[test]
    fn shared_subterms_are_stored_once() {
        let store = Store::new();
        let f = store.symbol("f");
        let g = store.symbol("g");
        let a = store.symbol("a");
        let shared = store.app(g, &[a]).expect("not cancelled");
        store.app(f, &[shared, shared]).expect("not cancelled");
        store.app(f, &[shared, shared]).expect("not cancelled");
        assert_eq!(store.stats().nodes, 2);
    }

    #[test]
    fn symbols_are_interned_by_name() {
        let store = Store::new();
        let first = store.symbol("Integrate");
        let second = store.symbol("Integrate");
        assert_eq!(first, second);
        assert_ne!(first, store.symbol("Integrale"));
        assert_eq!(
            store.with_symbol_name(first, |name| name.to_owned()),
            Some("Integrate".to_owned())
        );
        assert_eq!(store.stats().symbols, 5, "three kernel operators and two here");
    }

    #[test]
    fn the_same_name_in_two_modules_is_two_operators() {
        let store = Store::new();
        let core = store.symbol("Plus");
        let mine = store.symbol_in(ModuleId(1), "Plus");
        assert_ne!(core, mine);
        assert_eq!(core, store.plus());
        assert_eq!(store.signature(mine), None);
    }

    #[test]
    fn numbers_do_not_touch_the_node_table() {
        let store = Store::new();
        let a = store.int(7).expect("in range");
        let b = store.int(7).expect("in range");
        assert_eq!(a, b);
        assert_eq!(a.tag(), Tag::SmallInt);
        assert_eq!(store.as_int(a), Some(7));
        assert_eq!(store.stats().nodes, 0);
    }

    #[test]
    fn integers_outside_the_payload_await_the_side_table() {
        let store = Store::new();
        assert!(store.int(SMALL_INT_MAX).is_some());
        assert!(store.int(SMALL_INT_MIN).is_some());
        assert!(store.int(SMALL_INT_MAX + 1).is_none());
        assert!(store.int(SMALL_INT_MIN - 1).is_none());
    }

    #[test]
    fn rationals_are_reduced_before_they_are_tagged() {
        let store = Store::new();
        let half = store.rat(1, 2).expect("in range");
        assert_eq!(store.rat(2, 4), Some(half));
        assert_eq!(store.rat(-2, -4), Some(half));
        assert_eq!(store.as_rat(half), Some((1, 2)));
        assert_eq!(store.as_int(half), None);
    }

    #[test]
    fn rationals_with_unit_denominators_are_integers() {
        let store = Store::new();
        let two = store.int(2).expect("in range");
        assert_eq!(store.rat(4, 2), Some(two));
        assert_eq!(store.rat(-4, -2), Some(two));
        assert_eq!(store.rat(0, 5), store.int(0));
        assert_eq!(store.rat(1, 0), None);
    }

    #[test]
    fn negative_denominators_move_the_sign_to_the_numerator() {
        let store = Store::new();
        let expected = store.rat(-1, 3);
        assert_eq!(store.rat(1, -3), expected);
        assert_eq!(store.as_rat(expected.expect("in range")), Some((-1, 3)));
    }

    #[test]
    fn deep_sharing_keeps_the_node_count_linear() {
        let store = Store::new();
        let f = store.symbol("f");
        let mut term = store.symbol("a");
        for _ in 0..1_000 {
            term = store.app(f, &[term, term]).expect("not cancelled");
        }
        assert_eq!(store.stats().nodes, 1_000);
        assert_eq!(store.arity(term), 2);
    }

    #[test]
    fn a_cancelled_token_stops_construction() {
        let store = Store::new();
        let f = store.symbol("f");
        let a = store.symbol("a");
        let token = CancelToken::new();
        store.set_cancel(Some(token.clone()));
        assert!(store.app(f, &[a]).is_ok());
        token.cancel();
        assert_eq!(store.app(f, &[a]), Err(Cancelled));
        store.set_cancel(None);
        assert!(store.app(f, &[a]).is_ok());
    }

    #[test]
    fn declaring_the_same_laws_twice_is_allowed() {
        let store = Store::new();
        let associative =
            CanonicalSignature { associative: true, ..CanonicalSignature::EMPTY };
        let first = store.declare(ModuleId::CORE, "f", associative).expect("open");
        let second = store.declare(ModuleId::CORE, "f", associative).expect("identical");
        assert_eq!(first, second);
        assert_eq!(store.signature(first), Some(associative));
    }

    #[test]
    fn declaring_different_laws_conflicts() {
        let store = Store::new();
        store
            .declare(
                ModuleId::CORE,
                "f",
                CanonicalSignature { associative: true, ..CanonicalSignature::EMPTY },
            )
            .expect("open");
        assert!(
            store
                .declare(
                    ModuleId::CORE,
                    "f",
                    CanonicalSignature { commutative: true, ..CanonicalSignature::EMPTY },
                )
                .is_err()
        );
    }

    #[test]
    fn using_a_head_forecloses_declaring_laws_for_it() {
        let store = Store::new();
        let f = store.symbol("f");
        let a = store.symbol("a");
        store.app(f, &[a]).expect("not cancelled");
        assert_eq!(store.signature(f), Some(CanonicalSignature::EMPTY));
        assert!(
            store
                .declare(
                    ModuleId::CORE,
                    "f",
                    CanonicalSignature { associative: true, ..CanonicalSignature::EMPTY },
                )
                .is_err(),
            "terms headed by f already exist"
        );
    }

    #[test]
    fn a_symbol_that_is_only_an_argument_keeps_its_laws_open() {
        let store = Store::new();
        let f = store.symbol("f");
        let g = store.symbol("g");
        store.app(f, &[g]).expect("not cancelled");
        assert_eq!(store.signature(g), None);
        assert!(
            store
                .declare(
                    ModuleId::CORE,
                    "g",
                    CanonicalSignature { associative: true, ..CanonicalSignature::EMPTY },
                )
                .is_ok()
        );
    }
}
