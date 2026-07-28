//! Layer A at construction, against `docs/layer-a.md`.

use vieta_store::{CanonicalSignature, ExprId, ModuleId, Store};

fn app<'s>(store: &'s Store, head: ExprId<'s>, args: &[ExprId<'s>]) -> ExprId<'s> {
    store.app(head, args).expect("not cancelled")
}

fn plus<'s>(store: &'s Store, args: &[ExprId<'s>]) -> ExprId<'s> {
    app(store, store.plus(), args)
}

fn times<'s>(store: &'s Store, args: &[ExprId<'s>]) -> ExprId<'s> {
    app(store, store.times(), args)
}

fn power<'s>(store: &'s Store, base: ExprId<'s>, exponent: ExprId<'s>) -> ExprId<'s> {
    app(store, store.power(), &[base, exponent])
}

fn int(store: &Store, value: i64) -> ExprId<'_> {
    store.int(value).expect("in range")
}

fn rat(store: &Store, numerator: i64, denominator: i64) -> ExprId<'_> {
    store.rat(numerator, denominator).expect("in range")
}

fn declare<'s>(store: &'s Store, name: &str, signature: CanonicalSignature<'s>) -> ExprId<'s> {
    store.declare(ModuleId::CORE, name, signature).expect("open")
}

// The acceptance demo's item 4, and the reason structural equality means
// anything at all.
#[test]
fn a_sum_is_the_same_term_in_either_order() {
    let store = Store::new();
    let x = store.symbol("x");
    let two = int(&store, 2);
    assert_eq!(plus(&store, &[x, two]), plus(&store, &[two, x]));
}

#[test]
fn an_undeclared_operator_keeps_the_order_it_was_given() {
    let store = Store::new();
    let f = store.symbol("f");
    let x = store.symbol("x");
    let two = int(&store, 2);
    assert_ne!(app(&store, f, &[x, two]), app(&store, f, &[two, x]));
}

#[test]
fn nested_applications_of_an_associative_head_are_spliced() {
    let store = Store::new();
    let a = store.symbol("a");
    let b = store.symbol("b");
    let c = store.symbol("c");
    let nested = plus(&store, &[a, plus(&store, &[b, c])]);
    assert_eq!(nested, plus(&store, &[a, b, c]));
    assert_eq!(store.arity(nested), 3);
}

#[test]
fn units_are_dropped_and_annihilators_absorb() {
    let store = Store::new();
    let x = store.symbol("x");
    let zero = int(&store, 0);
    let one = int(&store, 1);
    assert_eq!(plus(&store, &[x, zero]), x);
    assert_eq!(times(&store, &[x, one]), x);
    assert_eq!(times(&store, &[x, zero]), zero);
}

#[test]
fn an_associative_head_collapses_at_arity_one_and_zero() {
    let store = Store::new();
    let x = store.symbol("x");
    assert_eq!(plus(&store, &[x]), x);
    assert_eq!(plus(&store, &[]), int(&store, 0));
    assert_eq!(times(&store, &[]), int(&store, 1));
}

#[test]
fn exact_numbers_fold() {
    let store = Store::new();
    assert_eq!(plus(&store, &[int(&store, 2), int(&store, 3)]), int(&store, 5));
    assert_eq!(times(&store, &[int(&store, 2), int(&store, 3)]), int(&store, 6));
    assert_eq!(
        plus(&store, &[rat(&store, 1, 2), rat(&store, 1, 3)]),
        rat(&store, 5, 6)
    );
    assert_eq!(times(&store, &[int(&store, 2), rat(&store, 1, 2)]), int(&store, 1));
}

#[test]
fn like_terms_collect() {
    let store = Store::new();
    let x = store.symbol("x");
    let two = int(&store, 2);
    let three = int(&store, 3);
    assert_eq!(plus(&store, &[x, x]), times(&store, &[two, x]));
    assert_eq!(
        plus(&store, &[times(&store, &[two, x]), times(&store, &[three, x])]),
        times(&store, &[int(&store, 5), x])
    );
    let half_x = times(&store, &[rat(&store, 1, 2), x]);
    assert_eq!(plus(&store, &[half_x, half_x]), x);
}

#[test]
fn a_term_and_its_negation_cancel_to_zero() {
    let store = Store::new();
    let x = store.symbol("x");
    let negated = times(&store, &[int(&store, -1), x]);
    assert_eq!(plus(&store, &[x, negated]), int(&store, 0));
}

#[test]
fn like_bases_combine_over_positive_integer_exponents() {
    let store = Store::new();
    let x = store.symbol("x");
    let two = int(&store, 2);
    let three = int(&store, 3);
    assert_eq!(times(&store, &[x, x]), power(&store, x, two));
    assert_eq!(
        times(&store, &[power(&store, x, two), power(&store, x, three)]),
        power(&store, x, int(&store, 5))
    );
    assert_eq!(times(&store, &[x, power(&store, x, two)]), power(&store, x, three));
}

#[test]
fn a_power_of_one_is_its_base_and_a_power_of_zero_is_not_one() {
    let store = Store::new();
    let x = store.symbol("x");
    assert_eq!(power(&store, x, int(&store, 1)), x);
    let zeroth = power(&store, x, int(&store, 0));
    assert_ne!(zeroth, int(&store, 1), "x^0 is not 1 at x = 0");
    assert_eq!(store.head(zeroth), Some(store.power()));
}

#[test]
fn numeric_powers_fold_only_at_integer_exponents() {
    let store = Store::new();
    assert_eq!(power(&store, int(&store, 2), int(&store, 3)), int(&store, 8));
    assert_eq!(power(&store, int(&store, 2), int(&store, -1)), rat(&store, 1, 2));
    assert_eq!(
        power(&store, rat(&store, 1, 2), int(&store, 3)),
        rat(&store, 1, 8)
    );

    // A branch choice is not a constructor's to make.
    let root = power(&store, int(&store, 4), rat(&store, 1, 2));
    assert_eq!(store.head(root), Some(store.power()));

    // No exact result, so the fold is declined rather than decided.
    for (base, exponent) in [(0, 0), (0, -1)] {
        let term = power(&store, int(&store, base), int(&store, exponent));
        assert_eq!(store.head(term), Some(store.power()), "{base}^{exponent}");
    }
}

// The condition on this pair is the acceptance demo's whole thesis, so Layer A
// must not delete it by cancelling at construction.
#[test]
fn nothing_cancels_at_construction() {
    let store = Store::new();
    let x = store.symbol("x");
    let inverse = power(&store, x, int(&store, -1));
    let quotient = times(&store, &[x, inverse]);
    assert_ne!(quotient, int(&store, 1), "x/x is 1 only away from zero");
    assert_eq!(store.head(quotient), Some(store.times()));
    assert_eq!(store.arity(quotient), 2);

    let a = store.symbol("a");
    let minus_one = int(&store, -1);
    let numerator = plus(&store, &[power(&store, a, int(&store, 2)), minus_one]);
    let denominator = plus(&store, &[a, minus_one]);
    let ratio = times(&store, &[numerator, power(&store, denominator, minus_one)]);
    assert_ne!(ratio, plus(&store, &[a, int(&store, 1)]));
}

#[test]
fn a_declared_operator_gets_the_same_treatment_as_a_kernel_one() {
    let store = Store::new();
    let empty = store.symbol("EmptySet");
    let union = declare(
        &store,
        "Union",
        CanonicalSignature {
            associative: true,
            commutative: true,
            idempotent: true,
            unit: Some(empty),
            zero: None,
        },
    );
    let a = store.symbol("a");
    let b = store.symbol("b");

    assert_eq!(app(&store, union, &[a, b]), app(&store, union, &[b, a]));
    assert_eq!(app(&store, union, &[a, a, b]), app(&store, union, &[a, b]));
    assert_eq!(app(&store, union, &[a, empty]), a);
    assert_eq!(app(&store, union, &[]), empty);
    assert_eq!(
        app(&store, union, &[a, app(&store, union, &[a, b])]),
        app(&store, union, &[a, b])
    );
}

#[test]
fn associativity_without_commutativity_flattens_and_keeps_order() {
    let store = Store::new();
    let concat = declare(
        &store,
        "Concat",
        CanonicalSignature { associative: true, ..CanonicalSignature::EMPTY },
    );
    let a = store.symbol("a");
    let b = store.symbol("b");
    let c = store.symbol("c");

    let nested = app(&store, concat, &[a, app(&store, concat, &[b, c])]);
    assert_eq!(nested, app(&store, concat, &[a, b, c]));
    assert_ne!(nested, app(&store, concat, &[c, b, a]));
}

#[test]
fn idempotence_without_commutativity_collapses_adjacent_arguments_only() {
    let store = Store::new();
    let head = declare(
        &store,
        "Squash",
        CanonicalSignature { idempotent: true, ..CanonicalSignature::EMPTY },
    );
    let a = store.symbol("a");
    let b = store.symbol("b");
    assert_eq!(app(&store, head, &[a, a, b]), app(&store, head, &[a, b]));
    assert_eq!(store.arity(app(&store, head, &[a, b, a])), 3);
}

#[test]
fn the_canonical_order_puts_numbers_first_then_symbols_then_applications() {
    let store = Store::new();
    let f = store.symbol("f");
    let y = store.symbol("y");
    let applied = app(&store, f, &[y]);
    let two = int(&store, 2);
    let sum = plus(&store, &[applied, y, two]);
    assert_eq!(store.collect_args(sum), vec![two, y, applied]);
}

// The order has to be a function of content. An id-ordered sort would make this
// depend on which symbol was interned first, and with it every canonical
// printed form and any implementation-independent conformance suite.
#[test]
fn the_canonical_order_does_not_depend_on_interning_order() {
    fn argument_names(interning: &[&str]) -> Vec<String> {
        let store = Store::new();
        for name in interning {
            store.symbol(name);
        }
        let args: Vec<_> = ["zeta", "alpha", "mu"]
            .iter()
            .map(|name| store.symbol(name))
            .collect();
        let sum = plus(&store, &args);
        store
            .collect_args(sum)
            .into_iter()
            .map(|id| {
                store
                    .with_symbol_name(id, |name| name.to_owned())
                    .expect("a symbol")
            })
            .collect()
    }

    let expected = vec!["alpha".to_owned(), "mu".to_owned(), "zeta".to_owned()];
    assert_eq!(argument_names(&["zeta", "alpha", "mu"]), expected);
    assert_eq!(argument_names(&["mu", "alpha", "zeta"]), expected);
    assert_eq!(argument_names(&["alpha", "zeta", "mu"]), expected);
}

/// A deterministic stream, so a failure is reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize
    }
}

// Canonicity for the commutative case: the id is a function of the argument
// multiset and of nothing else, and it is a function of the sequence when the
// operator has not declared commutativity.
#[test]
fn permuting_arguments_reaches_the_same_id_exactly_when_it_should() {
    let store = Store::new();
    let commutative = declare(
        &store,
        "Meet",
        CanonicalSignature { commutative: true, ..CanonicalSignature::EMPTY },
    );
    let ordered = store.symbol("Ordered");

    let atoms: Vec<ExprId<'_>> = ["p", "q", "r", "s", "t"]
        .iter()
        .map(|name| store.symbol(name))
        .collect();
    let mut pool: Vec<ExprId<'_>> = atoms.clone();
    pool.extend((0..4).map(|value| int(&store, value)));
    pool.push(app(&store, ordered, &[atoms[0], atoms[1]]));
    pool.push(power(&store, atoms[2], int(&store, 2)));

    let mut rng = Rng(0x1a2b_3c4d);
    for _ in 0..500 {
        let arity = 2 + rng.next() % 4;
        let args: Vec<ExprId<'_>> = (0..arity).map(|_| pool[rng.next() % pool.len()]).collect();

        let mut permuted = args.clone();
        for index in (1..permuted.len()).rev() {
            permuted.swap(index, rng.next() % (index + 1));
        }

        assert_eq!(
            app(&store, commutative, &args),
            app(&store, commutative, &permuted),
            "a commutative head distinguished a permutation"
        );
        if permuted != args {
            assert_ne!(
                app(&store, ordered, &args),
                app(&store, ordered, &permuted),
                "an undeclared head merged a permutation"
            );
        }
    }
}

// Every id the store hands out is already normal, which is what lets Layer A be
// a function applied once per node rather than a relation run to a fixpoint.
#[test]
fn rebuilding_a_term_from_its_own_parts_changes_nothing() {
    let store = Store::new();
    let x = store.symbol("x");
    let y = store.symbol("y");
    let built = [
        plus(&store, &[x, y, int(&store, 3)]),
        times(&store, &[int(&store, 2), x, y]),
        plus(&store, &[times(&store, &[int(&store, 2), x]), y]),
        power(&store, plus(&store, &[x, y]), int(&store, 2)),
    ];
    for term in built {
        let head = store.head(term).expect("an application");
        let args = store.collect_args(term);
        assert_eq!(app(&store, head, &args), term);
    }
}
