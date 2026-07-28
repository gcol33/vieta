//! The id space and its tag layout (D7).
//!
//! An [`ExprId`] is one `u32`: a three-bit tag and a twenty-nine-bit payload.
//! Small integers, small rationals, and symbols live entirely in the payload, so
//! arithmetic on small values never touches the intern table.
//!
//! ```text
//! bits 31..29   tag
//! bits 28..0    payload
//!
//! 0  Node       index into the node table
//! 1  SmallInt   29-bit two's complement integer
//! 2  Symbol     index into the symbol table
//! 3  SmallRat   14-bit signed numerator, 15-bit denominator, in lowest terms
//! 4  BigInt     index into the large-integer side table
//! 5  BigRat     index into the large-rational side table
//! ```
//!
//! Tags 6 and 7 are unassigned. The lifetime on `ExprId` ties an id to the
//! borrow of the store that produced it, which is what makes D8's rule (nothing
//! outside the store may hold a raw id across a safepoint) a compile error
//! rather than a convention.

use core::fmt;
use core::marker::PhantomData;

/// Bits reserved for the tag at the top of an id word.
pub const TAG_BITS: u32 = 3;

/// Bits available to a tag's payload.
pub const PAYLOAD_BITS: u32 = u32::BITS - TAG_BITS;

const PAYLOAD_MASK: u32 = (1u32 << PAYLOAD_BITS) - 1;

/// Largest value a payload can hold, and so the largest node or symbol index.
pub const MAX_PAYLOAD: u32 = PAYLOAD_MASK;

/// Smallest integer that fits inline in an id.
pub const SMALL_INT_MIN: i64 = -(1i64 << (PAYLOAD_BITS - 1));

/// Largest integer that fits inline in an id.
pub const SMALL_INT_MAX: i64 = (1i64 << (PAYLOAD_BITS - 1)) - 1;

const RAT_DEN_BITS: u32 = 15;
const RAT_NUM_BITS: u32 = PAYLOAD_BITS - RAT_DEN_BITS;
const RAT_DEN_MASK: u32 = (1u32 << RAT_DEN_BITS) - 1;
const RAT_NUM_MASK: u32 = (1u32 << RAT_NUM_BITS) - 1;

/// Smallest numerator that fits inline in an id.
pub const SMALL_RAT_NUM_MIN: i64 = -(1i64 << (RAT_NUM_BITS - 1));

/// Largest numerator that fits inline in an id.
pub const SMALL_RAT_NUM_MAX: i64 = (1i64 << (RAT_NUM_BITS - 1)) - 1;

/// Largest denominator that fits inline in an id.
pub const SMALL_RAT_DEN_MAX: u64 = RAT_DEN_MASK as u64;

/// What an id's payload denotes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u32)]
pub enum Tag {
    /// An interned application: a head and an argument list.
    Node = 0,
    /// An integer carried inline in the payload.
    SmallInt = 1,
    /// A symbol, by index into the symbol table.
    Symbol = 2,
    /// A rational in lowest terms carried inline in the payload.
    SmallRat = 3,
    /// An integer too large for the payload, by index into a side table.
    BigInt = 4,
    /// A rational too large for the payload, by index into a side table.
    BigRat = 5,
}

impl Tag {
    const fn from_bits(bits: u32) -> Option<Tag> {
        match bits {
            0 => Some(Tag::Node),
            1 => Some(Tag::SmallInt),
            2 => Some(Tag::Symbol),
            3 => Some(Tag::SmallRat),
            4 => Some(Tag::BigInt),
            5 => Some(Tag::BigRat),
            _ => None,
        }
    }

    /// Whether ids with this tag carry their whole value in the payload.
    pub const fn is_inline(self) -> bool {
        matches!(self, Tag::SmallInt | Tag::SmallRat)
    }
}

/// A reference to an expression in a [`Store`](crate::Store).
///
/// Structural equality is equality of the underlying word, because the store
/// interns every application it builds.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExprId<'s> {
    bits: u32,
    store: PhantomData<&'s ()>,
}

impl<'s> ExprId<'s> {
    pub(crate) const fn from_raw(bits: u32) -> Self {
        ExprId { bits, store: PhantomData }
    }

    pub(crate) const fn from_parts(tag: Tag, payload: u32) -> Self {
        debug_assert!(payload <= MAX_PAYLOAD);
        ExprId::from_raw(((tag as u32) << PAYLOAD_BITS) | (payload & PAYLOAD_MASK))
    }

    /// The whole id word, as it is stored in argument arrays and on the wire.
    pub const fn bits(self) -> u32 {
        self.bits
    }

    /// The payload, with the tag stripped.
    pub const fn payload(self) -> u32 {
        self.bits & PAYLOAD_MASK
    }

    /// What this id denotes.
    pub fn tag(self) -> Tag {
        match Tag::from_bits(self.bits >> PAYLOAD_BITS) {
            Some(tag) => tag,
            None => unreachable!("ids are only built by the store, which uses assigned tags"),
        }
    }
}

impl fmt::Debug for ExprId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.tag() {
            Tag::SmallInt => write!(f, "SmallInt({})", decode_small_int(self.payload())),
            Tag::SmallRat => {
                let (num, den) = decode_small_rat(self.payload());
                write!(f, "SmallRat({num}/{den})")
            }
            tag => write!(f, "{tag:?}({})", self.payload()),
        }
    }
}

/// Pack an integer into a payload, or report that it needs a side table.
pub(crate) const fn encode_small_int(value: i64) -> Option<u32> {
    if value < SMALL_INT_MIN || value > SMALL_INT_MAX {
        None
    } else {
        Some((value as u32) & PAYLOAD_MASK)
    }
}

/// Recover an integer from a `SmallInt` payload by sign-extending it.
pub(crate) const fn decode_small_int(payload: u32) -> i64 {
    (((payload << TAG_BITS) as i32) >> TAG_BITS) as i64
}

/// Pack a rational into a payload. The caller supplies lowest terms with a
/// positive denominator of at least two; anything else belongs to another tag.
pub(crate) fn encode_small_rat(num: i64, den: u64) -> Option<u32> {
    if den < 2 || den > SMALL_RAT_DEN_MAX {
        return None;
    }
    if num == 0 || num < SMALL_RAT_NUM_MIN || num > SMALL_RAT_NUM_MAX {
        return None;
    }
    if gcd(num.unsigned_abs(), den) != 1 {
        return None;
    }
    let numerator = (num as u32) & RAT_NUM_MASK;
    Some((numerator << RAT_DEN_BITS) | (den as u32))
}

/// Recover a rational from a `SmallRat` payload.
pub(crate) const fn decode_small_rat(payload: u32) -> (i64, u64) {
    let den = (payload & RAT_DEN_MASK) as u64;
    let shift = u32::BITS - RAT_NUM_BITS;
    let num = ((((payload >> RAT_DEN_BITS) << shift) as i32) >> shift) as i64;
    (num, den)
}

pub(crate) const fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_and_payload_round_trip() {
        let tags = [
            Tag::Node,
            Tag::SmallInt,
            Tag::Symbol,
            Tag::SmallRat,
            Tag::BigInt,
            Tag::BigRat,
        ];
        for tag in tags {
            for payload in [0, 1, 12_345, MAX_PAYLOAD - 1, MAX_PAYLOAD] {
                let id = ExprId::from_parts(tag, payload);
                assert_eq!(id.tag(), tag);
                assert_eq!(id.payload(), payload);
            }
        }
    }

    #[test]
    fn distinct_tags_never_alias() {
        let a = ExprId::from_parts(Tag::Node, 7);
        let b = ExprId::from_parts(Tag::Symbol, 7);
        let c = ExprId::from_parts(Tag::SmallInt, 7);
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn small_integers_round_trip() {
        let cases = [
            0,
            1,
            -1,
            2,
            -2,
            1_000_000,
            -1_000_000,
            SMALL_INT_MAX,
            SMALL_INT_MIN,
        ];
        for value in cases {
            let payload = encode_small_int(value).expect("value is in range");
            assert_eq!(decode_small_int(payload), value, "value {value}");
        }
    }

    #[test]
    fn integers_past_the_payload_need_a_side_table() {
        assert!(encode_small_int(SMALL_INT_MAX + 1).is_none());
        assert!(encode_small_int(SMALL_INT_MIN - 1).is_none());
        assert!(encode_small_int(i64::MAX).is_none());
        assert!(encode_small_int(i64::MIN).is_none());
    }

    #[test]
    fn small_rationals_round_trip() {
        let cases = [
            (1i64, 2u64),
            (-1, 2),
            (3, 4),
            (-3, 4),
            (1, SMALL_RAT_DEN_MAX),
            (SMALL_RAT_NUM_MAX, 2),
            (SMALL_RAT_NUM_MIN + 1, 2),
        ];
        for (num, den) in cases {
            let payload = encode_small_rat(num, den).expect("value is in range");
            assert_eq!(decode_small_rat(payload), (num, den), "value {num}/{den}");
        }
    }

    #[test]
    fn rationals_outside_the_contract_are_refused() {
        assert!(encode_small_rat(1, 1).is_none(), "denominator one is an integer");
        assert!(encode_small_rat(1, 0).is_none(), "zero denominator");
        assert!(encode_small_rat(0, 2).is_none(), "zero numerator is an integer");
        assert!(encode_small_rat(2, 4).is_none(), "not in lowest terms");
        assert!(encode_small_rat(1, SMALL_RAT_DEN_MAX + 1).is_none());
        assert!(encode_small_rat(SMALL_RAT_NUM_MAX + 1, 2).is_none());
    }

    #[test]
    fn inline_tags_are_the_number_tags() {
        assert!(Tag::SmallInt.is_inline());
        assert!(Tag::SmallRat.is_inline());
        assert!(!Tag::Node.is_inline());
        assert!(!Tag::Symbol.is_inline());
        assert!(!Tag::BigInt.is_inline());
    }

    #[test]
    fn the_layout_fits_the_word() {
        assert_eq!(TAG_BITS + PAYLOAD_BITS, u32::BITS);
        assert_eq!(RAT_NUM_BITS + RAT_DEN_BITS, PAYLOAD_BITS);
        assert_eq!(size_of::<ExprId<'_>>(), size_of::<u32>());
    }

    #[test]
    fn gcd_agrees_with_the_definition() {
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(12, 18), 6);
        assert_eq!(gcd(17, 5), 1);
    }
}
