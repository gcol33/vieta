//! Operator identities and the canonical-shape laws that belong to them (D36).
//!
//! A term head is a resolved operator, identified by a module path and a name.
//! The laws that determine an operator's stored shape are part of that identity
//! and are fixed once: either by an explicit declaration, or by the first use of
//! the operator as a head, which fixes the empty signature. Definitions,
//! matching policy, and notation are world state and live elsewhere.

use crate::id::ExprId;

/// A module path, which together with a name identifies an operator.
///
/// A different module is a different key and therefore a different operator,
/// which is what lets a name be rebound to fresh laws without disturbing terms
/// already built under the old ones.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct ModuleId(pub u32);

impl ModuleId {
    /// The kernel's own module, and the module an unqualified name resolves in
    /// until there is a module system to say otherwise.
    pub const CORE: ModuleId = ModuleId(0);
}

/// The laws that determine an operator's stored shape.
///
/// Each field is a claim about the operator that holds with no side condition,
/// because Layer A applies it while building a term and has nowhere to record a
/// condition (`docs/layer-a.md` §2). Whoever declares the operator owes that
/// claim.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CanonicalSignature<'s> {
    /// `f(a, f(b, c)) = f(a, b, c)`, so nested arguments are spliced into the
    /// parent, `f(a)` is `a`, and `f()` is the unit when one is declared.
    pub associative: bool,
    /// `f(a, b) = f(b, a)`, so arguments are sorted into the canonical order.
    pub commutative: bool,
    /// `f(a, a) = f(a)`, so duplicate arguments are dropped.
    pub idempotent: bool,
    /// An `e` with `f(a, e) = a`, dropped from argument lists.
    pub unit: Option<ExprId<'s>>,
    /// A `z` with `f(a, z) = z`, which makes the whole application `z`.
    pub zero: Option<ExprId<'s>>,
}

impl<'s> CanonicalSignature<'s> {
    /// No laws, which is what an ordinary symbol has.
    pub const EMPTY: CanonicalSignature<'s> = CanonicalSignature {
        associative: false,
        commutative: false,
        idempotent: false,
        unit: None,
        zero: None,
    };

    /// Whether this signature claims nothing, and so leaves construction to
    /// intern the application as written.
    pub fn is_empty(&self) -> bool {
        *self == CanonicalSignature::EMPTY
    }

    pub(crate) fn into_raw(self) -> RawSignature {
        RawSignature {
            associative: self.associative,
            commutative: self.commutative,
            idempotent: self.idempotent,
            unit: self.unit.map(ExprId::bits),
            zero: self.zero.map(ExprId::bits),
        }
    }
}

/// A signature with its element references stored as raw id words, which is how
/// the symbol table holds one.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct RawSignature {
    pub(crate) associative: bool,
    pub(crate) commutative: bool,
    pub(crate) idempotent: bool,
    pub(crate) unit: Option<u32>,
    pub(crate) zero: Option<u32>,
}

impl RawSignature {
    pub(crate) const EMPTY: RawSignature = RawSignature {
        associative: false,
        commutative: false,
        idempotent: false,
        unit: None,
        zero: None,
    };

    pub(crate) fn is_empty(self) -> bool {
        self == RawSignature::EMPTY
    }

    pub(crate) fn into_signature<'s>(self) -> CanonicalSignature<'s> {
        CanonicalSignature {
            associative: self.associative,
            commutative: self.commutative,
            idempotent: self.idempotent,
            unit: self.unit.map(ExprId::from_raw),
            zero: self.zero.map(ExprId::from_raw),
        }
    }
}

/// A declaration that contradicts the signature an operator already has.
///
/// Reached two ways, and they are the same case: declaring different laws twice
/// for one operator, and declaring laws for an operator that terms have already
/// been built with, whose first use fixed the empty signature. Declaring the
/// same laws again is not an error, which is what makes reloading a module a
/// no-op.
///
/// The two answers available are to declare a different operator, or to bind
/// the printed name to a new one in another module. A unit or zero headed by the
/// operator being declared is excluded by the same rule, since building that
/// term is what fixes the operator's signature.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SignatureConflict;

impl core::fmt::Display for SignatureConflict {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("the operator already has different canonical-shape laws")
    }
}

impl std::error::Error for SignatureConflict {}

#[cfg(test)]
mod tests {
    use super::{CanonicalSignature, ModuleId, RawSignature};

    #[test]
    fn the_empty_signature_is_the_default() {
        assert!(CanonicalSignature::EMPTY.is_empty());
        assert_eq!(CanonicalSignature::default(), CanonicalSignature::EMPTY);
        assert!(RawSignature::default().is_empty());
    }

    #[test]
    fn signatures_survive_the_round_trip_to_raw() {
        let signature = CanonicalSignature {
            associative: true,
            commutative: true,
            ..CanonicalSignature::EMPTY
        };
        assert_eq!(signature.into_raw().into_signature(), signature);
    }

    #[test]
    fn the_kernel_module_is_zero() {
        assert_eq!(ModuleId::CORE, ModuleId(0));
        assert_eq!(ModuleId::default(), ModuleId::CORE);
    }
}
