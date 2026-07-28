//! Kernel arithmetic (`docs/layer-a.md` §7).
//!
//! Three operator identities normalize by rules that are not derivable from any
//! canonical signature and are not available to user operators: `x + x -> 2*x`
//! relates `Plus` to `Times`, and no per-operator law says that. These rules are
//! attached to `Core.Plus`, `Core.Times`, and `Core.Power` and go no further.
//!
//! Everything here obeys the criterion of §2: a rule applies only when it holds
//! with no side condition. `x^1 -> x` qualifies. `x^0 -> 1` does not, because of
//! `0^0`. Combining `x^2` with `x^(-1)` does not, because the two sides disagree
//! at zero, which is why like bases combine only over positive integer
//! exponents and why nothing here ever cancels.

use crate::cancel::Cancelled;
use crate::node::{View, is_number};
use crate::num::Num;
use crate::store::Store;

/// What kernel arithmetic made of an application.
pub(crate) enum Outcome {
    /// A new argument list, still to be sorted and interned.
    Args(Vec<u32>),
    /// A complete answer that replaces the application.
    Done(u32),
}

/// Apply the rules belonging to one of the three kernel arithmetic heads.
pub(crate) fn normalize(store: &Store, head: u32, list: Vec<u32>) -> Result<Outcome, Cancelled> {
    let kernel = store.kernel();
    if head == kernel.power {
        return Ok(power(store, list));
    }
    let folded = match fold(store, head, &list) {
        Some(folded) => folded,
        None => return Ok(Outcome::Args(list)),
    };
    let collected = if head == kernel.plus {
        collect_terms(store, folded)?
    } else {
        collect_bases(store, folded)?
    };
    Ok(Outcome::Args(collected))
}

fn power(store: &Store, list: Vec<u32>) -> Outcome {
    if list.len() != 2 {
        return Outcome::Args(list);
    }
    let (base, exponent) = (list[0], list[1]);
    if exponent == store.kernel().one {
        return Outcome::Done(base);
    }
    if let (Some(base_value), Some(exponent_value)) = (Num::from_id(base), Num::from_id(exponent)) {
        if exponent_value.is_integer() {
            if let Some(id) = base_value.pow(exponent_value.num).and_then(|v| to_id(store, v)) {
                return Outcome::Done(id);
            }
        }
    }
    Outcome::Args(list)
}

/// Combine the exact numbers among the arguments into one.
///
/// Sorting has already placed them first, so they are a contiguous prefix and
/// the fold runs over them in a fixed order. `None` reports that some step had
/// no representable result, which leaves every argument standing and takes the
/// whole node out of collection as well, so that the two passes cannot disagree
/// about how many numbers a list holds.
fn fold(store: &Store, head: u32, list: &[u32]) -> Option<Vec<u32>> {
    let count = list.iter().take_while(|&&id| is_number(id)).count();
    if count < 2 {
        return Some(list.to_vec());
    }
    let plus = head == store.kernel().plus;
    let mut total = if plus { Num::ZERO } else { Num::ONE };
    for &id in &list[..count] {
        let value = Num::from_id(id)?;
        total = if plus { total.add(value)? } else { total.mul(value)? };
    }
    let mut folded = Vec::with_capacity(list.len() - count + 1);
    folded.push(to_id(store, total)?);
    folded.extend_from_slice(&list[count..]);
    Some(folded)
}

struct Term {
    coefficient: Num,
    monomial: u32,
    original: u32,
}

/// Group the arguments of a sum by monomial and sum their coefficients.
///
/// This is what makes `x - x` the term `0` and `x/2 + x/2` the term `x`, since
/// `-x` is `Times(-1, x)` and the rest follows from the unit and zero laws on
/// `Times`.
fn collect_terms(store: &Store, list: Vec<u32>) -> Result<Vec<u32>, Cancelled> {
    let mut collected = Vec::with_capacity(list.len());
    let mut terms = Vec::with_capacity(list.len());
    for &id in &list {
        if is_number(id) {
            collected.push(id);
        } else {
            terms.push(split_term(store, id)?);
        }
    }
    if terms.len() < 2 {
        return Ok(list);
    }
    terms.sort_unstable_by_key(|term| term.monomial);

    let mut start = 0;
    while start < terms.len() {
        let mut end = start + 1;
        while end < terms.len() && terms[end].monomial == terms[start].monomial {
            end += 1;
        }
        let group = &terms[start..end];
        match scaled(store, group)? {
            Some(term) => collected.extend(term),
            None => collected.extend(group.iter().map(|term| term.original)),
        }
        start = end;
    }
    Ok(collected)
}

/// One argument of a sum, read as a coefficient times a monomial.
fn split_term(store: &Store, id: u32) -> Result<Term, Cancelled> {
    let times = store.kernel().times;
    let rest = store.with_view(|view| {
        if !view.is_headed_by(id, times) {
            return None;
        }
        let args = view.args_at(id);
        let coefficient = Num::from_id(*args.first()?)?;
        Some((coefficient, args[1..].to_vec()))
    });
    match rest {
        Some((coefficient, factors)) => {
            let monomial = store.app_raw(times, &factors)?;
            Ok(Term { coefficient, monomial, original: id })
        }
        None => Ok(Term { coefficient: Num::ONE, monomial: id, original: id }),
    }
}

/// The single term a group collapses to, or `None` when its coefficient has no
/// representable sum and the group has to stay as it was.
fn scaled(store: &Store, group: &[Term]) -> Result<Option<Vec<u32>>, Cancelled> {
    if group.len() < 2 {
        return Ok(Some(vec![group[0].original]));
    }
    let mut total = Num::ZERO;
    for term in group {
        match total.add(term.coefficient) {
            Some(sum) => total = sum,
            None => return Ok(None),
        }
    }
    if total == Num::ZERO {
        return Ok(Some(Vec::new()));
    }
    if total == Num::ONE {
        return Ok(Some(vec![group[0].monomial]));
    }
    let Some(coefficient) = to_id(store, total) else {
        return Ok(None);
    };
    let scaled = store.app_raw(store.kernel().times, &[coefficient, group[0].monomial])?;
    Ok(Some(vec![scaled]))
}

struct Factor {
    base: u32,
    exponent: i64,
    original: u32,
}

/// Group the arguments of a product by base and add their exponents, over the
/// positive integer exponents only.
fn collect_bases(store: &Store, list: Vec<u32>) -> Result<Vec<u32>, Cancelled> {
    let mut collected = Vec::with_capacity(list.len());
    let mut factors = Vec::with_capacity(list.len());
    store.with_view(|view| {
        for &id in &list {
            match split_factor(view, store.kernel().power, id) {
                Some(factor) => factors.push(factor),
                None => collected.push(id),
            }
        }
    });
    if factors.len() < 2 {
        return Ok(list);
    }
    factors.sort_unstable_by_key(|factor| factor.base);

    let mut start = 0;
    while start < factors.len() {
        let mut end = start + 1;
        while end < factors.len() && factors[end].base == factors[start].base {
            end += 1;
        }
        let group = &factors[start..end];
        match raised(store, group)? {
            Some(id) => collected.push(id),
            None => collected.extend(group.iter().map(|factor| factor.original)),
        }
        start = end;
    }
    Ok(collected)
}

/// One argument of a product, read as a base raised to a positive integer
/// exponent. `None` for anything that may not be combined: a number, and a power
/// whose exponent is not a positive integer, which is where `x^2 * x^(-1)`
/// disagrees with `x` at zero.
fn split_factor(view: View<'_>, power: u32, id: u32) -> Option<Factor> {
    if is_number(id) {
        return None;
    }
    if !view.is_headed_by(id, power) {
        return Some(Factor { base: id, exponent: 1, original: id });
    }
    let args = view.args_at(id);
    if args.len() != 2 {
        return None;
    }
    let exponent = Num::from_id(args[1])?;
    if !exponent.is_integer() || exponent.num <= 0 {
        return None;
    }
    Some(Factor { base: args[0], exponent: exponent.num, original: id })
}

/// The single power a group collapses to, or `None` when the exponents have no
/// representable sum.
fn raised(store: &Store, group: &[Factor]) -> Result<Option<u32>, Cancelled> {
    if group.len() < 2 {
        return Ok(Some(group[0].original));
    }
    let mut total: i64 = 0;
    for factor in group {
        match total.checked_add(factor.exponent) {
            Some(sum) => total = sum,
            None => return Ok(None),
        }
    }
    let Some(exponent) = store.int(total).map(|id| id.bits()) else {
        return Ok(None);
    };
    Ok(Some(store.app_raw(store.kernel().power, &[group[0].base, exponent])?))
}

fn to_id(store: &Store, value: Num) -> Option<u32> {
    let id = if value.is_integer() {
        store.int(value.num)
    } else {
        store.rat(value.num, value.den)
    };
    id.map(|id| id.bits())
}
