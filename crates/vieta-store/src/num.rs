//! Exact rational arithmetic on the values that fit in an id.
//!
//! Every operation is checked and returns `None` when its result cannot be
//! represented. Layer A treats that as a reason to decline a fold rather than
//! as an error (`docs/layer-a.md` §8), so an unrepresentable result leaves the
//! arguments standing.
//!
//! The range is the id payload's, which is what the store can hold until the
//! large-number side table arrives with M1.

use core::cmp::Ordering;

use crate::id::{Tag, decode_small_int, decode_small_rat, gcd};
use crate::node::{payload, tag};

/// An exact rational in lowest terms with a positive denominator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Num {
    pub(crate) num: i64,
    pub(crate) den: i64,
}

impl Num {
    pub(crate) const ZERO: Num = Num { num: 0, den: 1 };
    pub(crate) const ONE: Num = Num { num: 1, den: 1 };

    /// A rational in lowest terms with a positive denominator, or `None` when
    /// the denominator is zero or reduction overflows.
    pub(crate) fn reduced(num: i64, den: i64) -> Option<Num> {
        if den == 0 {
            return None;
        }
        let (num, den) = if den < 0 {
            (num.checked_neg()?, den.checked_neg()?)
        } else {
            (num, den)
        };
        if num == 0 {
            return Some(Num::ZERO);
        }
        let divisor = gcd(num.unsigned_abs(), den as u64) as i64;
        Some(Num { num: num / divisor, den: den / divisor })
    }

    /// The number an id denotes, or `None` when the id denotes something else.
    pub(crate) fn from_id(id: u32) -> Option<Num> {
        match tag(id) {
            Tag::SmallInt => Some(Num { num: decode_small_int(payload(id)), den: 1 }),
            Tag::SmallRat => {
                let (num, den) = decode_small_rat(payload(id));
                Some(Num { num, den: den as i64 })
            }
            Tag::BigInt | Tag::BigRat => {
                unreachable!("large numbers arrive with the side table at M1")
            }
            _ => None,
        }
    }

    pub(crate) fn is_integer(self) -> bool {
        self.den == 1
    }

    pub(crate) fn add(self, other: Num) -> Option<Num> {
        let den = self.den.checked_mul(other.den)?;
        let left = self.num.checked_mul(other.den)?;
        let right = other.num.checked_mul(self.den)?;
        Num::reduced(left.checked_add(right)?, den)
    }

    pub(crate) fn mul(self, other: Num) -> Option<Num> {
        Num::reduced(
            self.num.checked_mul(other.num)?,
            self.den.checked_mul(other.den)?,
        )
    }

    /// `self` raised to an integer power, or `None` when there is no exact
    /// result. `0^0` has no exact result here, which is the reason `x^0 -> 1`
    /// is not a Layer A rule.
    pub(crate) fn pow(self, exponent: i64) -> Option<Num> {
        if self.num == 0 {
            return match exponent.cmp(&0) {
                Ordering::Greater => Some(Num::ZERO),
                Ordering::Equal | Ordering::Less => None,
            };
        }
        let magnitude = u32::try_from(exponent.unsigned_abs()).ok()?;
        let (num, den) = if exponent >= 0 {
            (self.num.checked_pow(magnitude)?, self.den.checked_pow(magnitude)?)
        } else {
            (self.den.checked_pow(magnitude)?, self.num.checked_pow(magnitude)?)
        };
        Num::reduced(num, den)
    }
}

impl PartialOrd for Num {
    fn partial_cmp(&self, other: &Num) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Num {
    fn cmp(&self, other: &Num) -> Ordering {
        let left = i128::from(self.num) * i128::from(other.den);
        let right = i128::from(other.num) * i128::from(self.den);
        left.cmp(&right)
    }
}

#[cfg(test)]
mod tests {
    use super::Num;

    #[test]
    fn reduction_normalizes_sign_and_terms() {
        assert_eq!(Num::reduced(2, 4), Some(Num { num: 1, den: 2 }));
        assert_eq!(Num::reduced(1, -3), Some(Num { num: -1, den: 3 }));
        assert_eq!(Num::reduced(-2, -4), Some(Num { num: 1, den: 2 }));
        assert_eq!(Num::reduced(0, 5), Some(Num::ZERO));
        assert_eq!(Num::reduced(1, 0), None);
    }

    #[test]
    fn addition_and_multiplication_are_exact() {
        let half = Num::reduced(1, 2).expect("in range");
        let third = Num::reduced(1, 3).expect("in range");
        assert_eq!(half.add(third), Num::reduced(5, 6));
        assert_eq!(half.mul(third), Num::reduced(1, 6));
        assert_eq!(half.add(half), Some(Num::ONE));
    }

    #[test]
    fn addition_declines_rather_than_wrapping() {
        let big = Num { num: i64::MAX, den: 1 };
        assert_eq!(big.add(Num::ONE), None);
    }

    #[test]
    fn powers_handle_both_signs() {
        let two = Num { num: 2, den: 1 };
        assert_eq!(two.pow(3), Num::reduced(8, 1));
        assert_eq!(two.pow(-1), Num::reduced(1, 2));
        assert_eq!(two.pow(0), Some(Num::ONE));
        assert_eq!(Num::reduced(1, 2).expect("in range").pow(3), Num::reduced(1, 8));
        assert_eq!(Num::reduced(-2, 1).expect("in range").pow(3), Num::reduced(-8, 1));
    }

    #[test]
    fn zero_powers_that_have_no_exact_value_are_declined() {
        assert_eq!(Num::ZERO.pow(0), None);
        assert_eq!(Num::ZERO.pow(-1), None);
        assert_eq!(Num::ZERO.pow(3), Some(Num::ZERO));
    }

    #[test]
    fn a_huge_exponent_overflows_instead_of_looping() {
        let two = Num { num: 2, den: 1 };
        assert_eq!(two.pow(1_000_000_000), None);
        assert_eq!(Num::ONE.pow(1_000_000_000), Some(Num::ONE));
    }

    #[test]
    fn ordering_compares_values_not_representations() {
        let half = Num::reduced(1, 2).expect("in range");
        let two = Num { num: 2, den: 1 };
        let minus = Num { num: -1, den: 1 };
        assert!(minus < half);
        assert!(half < two);
        assert_eq!(half.cmp(&Num::reduced(2, 4).expect("in range")), core::cmp::Ordering::Equal);
    }
}
