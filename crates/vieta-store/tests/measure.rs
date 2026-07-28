//! The store measurements from architecture.md §1.9 item 1, taken from the
//! shipping component rather than a prototype.
//!
//! Run with output:
//!
//! ```text
//! cargo test -p vieta-store --release --test measure -- --nocapture
//! ```
//!
//! `VIETA_MEASURE_ATTEMPTS` sets the number of applications issued. The default
//! keeps the test suite quick; the numbers that inform the tag layout should be
//! taken from a run in the millions.

use std::time::Instant;

use vieta_store::{ExprId, Store};

const DEFAULT_ATTEMPTS: usize = 200_000;

/// Applications issued per node interned, which §1.9 calls heavy sharing. The
/// generator sizes its argument universe to hit roughly this ratio, so the
/// corpus keeps the property at any number of attempts.
const TARGET_SHARING: usize = 4;

/// Terms are built in rounds. Within a round the set of available arguments is
/// frozen, which is what bounds the reachable `(head, args)` combinations and
/// therefore forces repeats. Each round draws on the previous one's output, so
/// the corpus gains a level of depth per round.
const ROUNDS: usize = 8;

const HEADS: [&str; 7] = ["Plus", "Times", "Power", "Sin", "Cos", "f", "g"];
const MAX_ARITY: usize = 3;
const ATOMS: [&str; 5] = ["x", "y", "z", "a", "b"];

/// A deterministic stream, so a reported number is reproducible.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize
    }
}

fn attempts() -> usize {
    std::env::var("VIETA_MEASURE_ATTEMPTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_ATTEMPTS)
}

/// The universe width whose reachable `(head, args)` combinations number about
/// one per `TARGET_SHARING` applications in a round.
fn universe_width(attempts: usize) -> usize {
    let per_round = (attempts / ROUNDS).max(1);
    let combinations = (per_round / TARGET_SHARING / HEADS.len()).max(1);
    let width = (combinations as f64).powf(1.0 / MAX_ARITY as f64) as usize;
    width.clamp(4, 8_192)
}

/// Half leaves and half a spread sample of the previous round, so every depth
/// keeps leaves available and the corpus does not degenerate into one spine.
fn universe<'s>(atoms: &[ExprId<'s>], previous: &[ExprId<'s>], width: usize) -> Vec<ExprId<'s>> {
    let leaves = (width / 2).clamp(1, atoms.len());
    let mut chosen: Vec<ExprId<'s>> = atoms[..leaves].to_vec();
    let wanted = width.saturating_sub(leaves);
    if wanted > 0 && !previous.is_empty() {
        let stride = (previous.len() / wanted).max(1);
        chosen.extend(previous.iter().step_by(stride).take(wanted).copied());
    }
    chosen
}

/// Issue `attempts` applications in rounds, returning the last round's terms.
fn build<'s>(store: &'s Store, attempts: usize, width: usize, seed: u64) -> Vec<ExprId<'s>> {
    let heads: Vec<ExprId<'s>> = HEADS.iter().map(|name| store.symbol(name)).collect();

    let mut atoms: Vec<ExprId<'s>> = ATOMS.iter().map(|name| store.symbol(name)).collect();
    atoms.extend((0..8).map(|value| store.int(value).expect("small")));

    let per_round = (attempts / ROUNDS).max(1);
    let mut rng = Rng::new(seed);
    let mut args = Vec::with_capacity(MAX_ARITY);
    let mut previous: Vec<ExprId<'s>> = Vec::new();
    let mut produced: Vec<ExprId<'s>> = Vec::with_capacity(per_round);

    for _ in 0..ROUNDS {
        let frozen = universe(&atoms, &previous, width);
        produced.clear();
        for _ in 0..per_round {
            let head = heads[rng.next() % heads.len()];
            let arity = 1 + rng.next() % MAX_ARITY;
            args.clear();
            for _ in 0..arity {
                args.push(frozen[rng.next() % frozen.len()]);
            }
            produced.push(store.app(head, &args).expect("not cancelled"));
        }
        previous.clear();
        previous.extend_from_slice(&produced);
    }
    produced
}

fn per_second(count: usize, seconds: f64) -> f64 {
    if seconds <= 0.0 { f64::INFINITY } else { count as f64 / seconds }
}

#[test]
fn store_measurements() {
    let attempts = attempts();
    let width = universe_width(attempts);
    let store = Store::new();

    let start = Instant::now();
    let pool = build(&store, attempts, width, 0x5eed_1234);
    let construction = start.elapsed().as_secs_f64();

    let stats = store.stats();
    assert!(stats.nodes > 0, "the corpus built no nodes");

    let sharing = attempts as f64 / stats.nodes as f64;
    assert!(
        sharing >= 2.0,
        "corpus shared {sharing:.2} applications per node, \
         which does not measure what §1.9 asks about"
    );

    // Re-issuing the same corpus hits the intern table on every call, which is
    // what establishing structural identity costs.
    let start = Instant::now();
    let repeat = build(&store, attempts, width, 0x5eed_1234);
    let lookup = start.elapsed().as_secs_f64();
    assert_eq!(pool, repeat, "rebuilding the corpus produced different ids");
    assert_eq!(
        store.stats().nodes,
        stats.nodes,
        "rebuilding the corpus allocated new nodes"
    );

    // Structural equality itself, once the ids are in hand.
    let start = Instant::now();
    let mut equal = 0usize;
    for _ in 0..64 {
        for (left, right) in pool.iter().zip(&repeat) {
            if left == right {
                equal += 1;
            }
        }
    }
    let comparisons = pool.len() * 64;
    let compare = start.elapsed().as_secs_f64();
    assert_eq!(equal, comparisons);

    // A whole-store walk through the public accessors, so the number includes
    // what a traversal actually pays.
    let start = Instant::now();
    let mut visited = 0usize;
    let mut arg_reads = 0usize;
    for id in store.node_ids() {
        visited += 1;
        for index in 0..store.arity(id) {
            assert!(store.arg(id, index).is_some());
            arg_reads += 1;
        }
    }
    let walk = start.elapsed().as_secs_f64();
    assert_eq!(visited, stats.nodes);
    assert_eq!(arg_reads, stats.arg_words);

    println!();
    println!("store measurements (architecture.md §1.9 item 1)");
    println!("  applications issued   {attempts} over {ROUNDS} rounds");
    println!("  argument universe     {width} terms per round");
    println!("  nodes interned        {}", stats.nodes);
    println!("  symbols               {}", stats.symbols);
    println!("  argument words        {}", stats.arg_words);
    println!("  intern table slots    {}", stats.table_slots);
    println!("  sharing               {sharing:.2} applications per node");
    println!(
        "  memory per node       {:.1} bytes used, {:.1} bytes reserved",
        stats.used_bytes_per_node(),
        stats.reserved_bytes_per_node()
    );
    println!(
        "  heap                  {} bytes used, {} bytes reserved",
        stats.used_bytes, stats.reserved_bytes
    );
    println!(
        "  construction          {:.0} applications/s ({construction:.3} s)",
        per_second(attempts, construction)
    );
    println!(
        "  intern lookup         {:.0} applications/s ({lookup:.3} s)",
        per_second(attempts, lookup)
    );
    println!(
        "  structural equality   {:.0} comparisons/s ({compare:.3} s)",
        per_second(comparisons, compare)
    );
    println!(
        "  whole-store walk      {:.0} nodes/s ({walk:.3} s)",
        per_second(visited, walk)
    );
    println!();
}
