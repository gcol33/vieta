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

Underneath, Vieta treats symbolic expressions as elements of free algebras modulo
explicit theories, interprets them in runtime mathematical models, and computes
with them through condition-preserving transformations. The organising thesis:
**lowering emits obligations; lifting preserves them.**

## Status

Early implementation. The kernel spine is store, parser, compiler, bytecode
machine, printer, in that order.

`crates/vieta-store` is a hash-consed term store with a tagged 32-bit id space,
normalizing at construction, so `x + 2` and `2 + x` are one id and structural
equality is a word comparison.

`crates/vieta-syntax` is the syntax layer: a lossless concrete tree that
reproduces its source byte for byte, malformed input included, and the surface
syntax elaboration reads, which drops grouping and trivia and keeps names,
binding forms, and where each node came from.

## Documents

- [`docs/architecture.md`](docs/architecture.md) — architectural assessment: what
  the artifact is and where the mathematics sits, the host-language argument, the
  self-hosting thesis and how it stays falsifiable, the FLINT-`gr` abstraction
  boundary, the milestone sequence, and a map of the surface that remains.
- [`docs/decisions.md`](docs/decisions.md) — irreversible-decision register,
  D1 through D37: each entry records the decision, the alternatives considered,
  why reversal is expensive, and current status. Read D25 first.
- [`docs/layer-a.md`](docs/layer-a.md) — the construction-time normalization
  specification: the canonical signature vocabulary, the canonical order, the
  pass sequence, and the canonicity theorem that gives structural equality its
  meaning.
- [`docs/measurements.md`](docs/measurements.md) — numbers taken from the
  components themselves, with the machine, the corpus, and what each one settles.

## License

MIT. See [LICENSE](LICENSE).
