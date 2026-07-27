# Vieta

A strict, expression-oriented functional-rewrite language for computer algebra.

Ordinary computation uses lexically scoped functional programming. Symbolic
computation uses first-class guarded rewrite rules controlled by explicit
strategies. Syntax, terms, and runtime values are distinct but interoperable.
Mathematical domains, assumptions, rule sets, and sessions are values. Effects
and dynamic lookup are explicit. Vieta code compiles against immutable versioned
worlds.

The organising thesis: **lowering emits obligations; lifting preserves them.**

## Status

Design stage. This crate carries no implementation and exists to hold the name
while the design settles.

## Documents

- [`docs/architecture.md`](docs/architecture.md) — architectural assessment: the
  host-language argument, the self-hosting thesis and how it stays falsifiable,
  the FLINT-`gr` abstraction boundary, and the milestone sequence.
- [`docs/decisions.md`](docs/decisions.md) — irreversible-decision register,
  D1 through D32: each entry records the decision, the alternatives considered,
  why reversal is expensive, and current status. Read D25 first.

## License

MIT. See [LICENSE](LICENSE).
