//! The interned expression store: Vieta's term representation and the id space
//! everything else holds handles into.
//!
//! An expression is either an atom carried inline in its id (a small integer, a
//! small rational, a symbol) or an interned application of a head to arguments.
//! Applications are normalized by Layer A and then hash-consed, so equal
//! subterms are stored once and structural equality is a `u32` comparison.
//!
//! ```
//! use vieta_store::Store;
//!
//! let store = Store::new();
//! let x = store.symbol("x");
//! let two = store.int(2).expect("2 is small");
//!
//! assert_eq!(store.app(store.plus(), &[x, two])?, store.app(store.plus(), &[two, x])?);
//! # Ok::<(), vieta_store::Cancelled>(())
//! ```
//!
//! That is Layer A, not interning. `Plus` is commutative, so its arguments are
//! sorted into a canonical order before the node is built, and the two spellings
//! reach the same id. An operator that has declared nothing keeps what it was
//! given:
//!
//! ```
//! # use vieta_store::Store;
//! # let store = Store::new();
//! # let x = store.symbol("x");
//! # let two = store.int(2).expect("2 is small");
//! let f = store.symbol("f");
//!
//! assert_ne!(store.app(f, &[x, two])?, store.app(f, &[two, x])?);
//! # Ok::<(), vieta_store::Cancelled>(())
//! ```
//!
//! Which laws an operator has is part of its identity and is fixed once (D36),
//! so a term can never acquire a different shape than the one it was interned
//! with. `docs/layer-a.md` is the specification: what Layer A normalizes, the
//! canonical order, and where its completeness deliberately stops. Semantic
//! equivalence beyond that is a separate context-scoped layer and never a
//! mutation of store identity (D9).
//!
//! Two invariants are enforced rather than documented. Every application is
//! interned, so `ExprId` equality is structural equality. And an `ExprId`
//! borrows the store that produced it, so holding one across
//! [`Store::safepoint`] fails to compile, which is D8.

mod arith;
mod cancel;
mod hash;
mod id;
mod node;
mod normalize;
mod num;
mod operator;
mod order;
mod probe;
mod store;
mod symbol;

pub use cancel::{CancelToken, Cancelled};
pub use id::{
    ExprId, MAX_PAYLOAD, PAYLOAD_BITS, SMALL_INT_MAX, SMALL_INT_MIN, SMALL_RAT_DEN_MAX,
    SMALL_RAT_NUM_MAX, SMALL_RAT_NUM_MIN, TAG_BITS, Tag,
};
pub use operator::{CanonicalSignature, ModuleId, SignatureConflict};
pub use store::{Store, StoreStats};
