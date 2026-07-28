# Vieta

A strict, expression-oriented functional-rewrite language for computer algebra.
Vieta is a programming language whose largest standard library is a computer
algebra system. Compilation covers the control flow, the recursion, and the calls;
terms, patterns, guards, domains, and obligations stay first-class runtime values.

Ordinary computation uses lexically scoped functional programming. Symbolic
computation uses first-class guarded rewrite rules controlled by explicit
strategies. Syntax, terms, and runtime values are distinct but interoperable.
Mathematical domains, assumptions, rule sets, and sessions are values. Effects
and dynamic lookup are explicit. Vieta code compiles against immutable versioned
worlds.

The organising thesis: **lowering emits obligations; lifting preserves them.**

## Status

Early implementation. The kernel spine is store, parser, compiler, bytecode
machine, printer, in that order. `crates/vieta-store` carries the first of them:
a hash-consed term store with a tagged 32-bit id space, where structural equality
is a word comparison and equal subterms are stored once.

## Documents

- [`docs/architecture.md`](docs/architecture.md) — architectural assessment: what
  the artifact is and where the mathematics sits, the host-language argument, the
  self-hosting thesis and how it stays falsifiable, the FLINT-`gr` abstraction
  boundary, and the milestone sequence.
- [`docs/decisions.md`](docs/decisions.md) — irreversible-decision register,
  D1 through D36: each entry records the decision, the alternatives considered,
  why reversal is expensive, and current status. Read D25 first.
- [`docs/layer-a.md`](docs/layer-a.md) — the construction-time normalization
  specification: the canonical signature vocabulary, the canonical order, the
  pass sequence, and the canonicity theorem that gives structural equality its
  meaning.
- [`docs/measurements.md`](docs/measurements.md) — numbers taken from the
  components themselves, with the machine, the corpus, and what each one settles.

## License

MIT. See [LICENSE](LICENSE).
