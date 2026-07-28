# Vieta: Architectural Assessment

Revised 2026-07-28. Companion document: `decisions.md` (the irreversible-decision
register, renumbered to follow the order of this document).

Settled since the first draft: the host language is Rust, the mathematical
library self-hosts in Vieta, and conditions lead the differentiator sequence. The
reasoning for each is recorded here because in five years the reasoning is worth
more than the conclusion.

---

## 0. Verdict in one page

§0.5 and §0.6 state what Vieta *is*, and are prior to everything here. Given
that, five decisions determine whether Vieta reaches its endpoint. Everything
else is recoverable engineering.

**D2. Vieta distinguishes four meanings of equality before simplification rule
one.** Structural, domain, provable, extensional. `(a^2-1)/(a-1)` and `a+1` are
equal in the rational function field `Q(a)` and are not the same partial function
at `a = 1`. Systems that conflate these become quietly dishonest, and the
conflation is not fixable afterwards because every rule has already assumed one
reading.

**D3. Every rewrite returns a guarded result set, with obligations and a
derivation sink.** Not `Maybe Term`, not a term plus a conjunction of side
conditions. Rewriting naturally produces case splits, and the disjunctive
structure has to be in the return type from rule one.

**D7 and D9. Hash-consed store with alpha-invariant binders, and semantic
equivalence as a separate context-scoped layer.** Structural identity is stable;
semantic equivalence varies with assumptions. Collapsing the two makes contextual
reasoning impossible.

**D13 and D19. Rule sets and session state are values.** These were justified on
soundness and reproducibility grounds. They turn out also to be the decisions
that make Vieta code compilable, which the self-hosting thesis requires (§3).

**D33. Layer B is compiled from slice 1.** Syntax lowers to bytecode and a machine
runs it. No tree-walking evaluator is written. The compilation path decides what is
affordable to write in Vieta, so it has to exist before the library it is meant to
carry rather than after it (§3.1).

The organising thesis, which everything else serves:

> **Lowering emits obligations; lifting preserves them.**

---

## 0.5 The constitution

Settled, and recorded as identity rather than as a revisable design choice:

> **Vieta is a strict, expression-oriented functional-rewrite language. Ordinary
> computation uses lexically scoped functional programming. Symbolic computation
> uses first-class guarded rewrite rules controlled by explicit strategies. Syntax,
> terms, and runtime values are distinct but interoperable. Mathematical domains,
> assumptions, rule sets, and sessions are values. Effects and dynamic lookup are
> explicit. Vieta code compiles against immutable versioned worlds.**

Compactly:

```
strict functional core
+ explicit quotation
+ hygienic macros
+ first-class guarded rewriting
+ explicit transformation strategies
+ immutable versioned worlds
+ explicit effects and dynamic lookup
+ runtime mathematical domains
+ three-valued logical reasoning
+ an early bytecode compilation path
```

The load-bearing word is **functional-rewrite**. A functional language with a
rewriting library ends up simulating rewriting through functions, and a rewriting
system with functions bolted on ends up simulating control flow through rule
ordering. Both mechanisms are primitive in Vieta, and §2.10 fixes which is which.

What the constitution rules out, listed so it is not relitigated: global laziness
(§2.7), one universal storage representation for every runtime value (§2.8), a
single evaluate-until-fixpoint mechanism (§2.9), rules that carry their own
traversal policy (§2.10), macros as symbol substitution (§2.11), untracked effects
(§2.12), and unconstrained global method extension (§2.13).

Registered as D25 through D32. Exact syntax and internal encodings stay open until
the semantic contracts in §2 are written down.

---

## 0.6 What the artifact is

Two properties are routinely fused and are independent axes. One is mathematical
breadth: how much algebra, calculus, number theory, and analysis the system knows.
The other is the execution model for symbolic work: whether transformation runs
through a dynamic evaluator or through compiled code.

```
                        mathematical breadth
                               high
                                |
                Mathematica     |     Vieta's target
                                |
   evaluated symbolic ----------+---------- compiled symbolic
                                |
                Maude, Stratego |
                                |
                               low
```

Mathematica has the breadth, reached on an evaluator that rewrites to fixpoint.
Maude and Stratego execute symbolic transformation as compiled languages and
carry no mathematical library. Vieta is aimed at the quadrant holding both, and
that aim is what §3's self-hosting thesis and D33 are for.

The positioning follows:

> **Vieta is a programming language. The computer algebra system is its largest
> standard library.**

### What compiled symbolic execution means

A conventional compiler consumes syntax and emits code operating over numbers,
strings, records, and objects. Program structure is a compile-time artifact and
stops being a runtime value at the end of compilation.

Vieta compiles the control flow, the recursion, the branching, and the calls.
Terms, patterns, guards, domains, and obligations remain first-class runtime
values. **What gets compiled is the machinery that manipulates symbols.**

The characteristic function is the one that takes a term apart. Surface syntax is
illustrative and open:

```
fn differentiate(expr, x) =
    match expr {
        Add(terms...) =>
            Add(map(fn(t) = differentiate(t, x), terms)...)
        Mul(a, b) =>
            differentiate(a, x) * b + a * differentiate(b, x)
        _ when free_of(expr, x) =>
            0
    }
```

The recursion, the dispatch, the map, and the guard compile. `expr`, the `Add`
pattern, and the result term stay symbolic. That is a different artifact from an
expression library embedded in Rust or Python, where the host compiles and the
symbolic layer is data the host interprets. Two register entries carry the
consequences: D34 fixes that a term is one kind of runtime value, and D35 fixes
that the destructuring compiles alongside the recursion.

### Where the mathematics sits

```
Vieta language
  |- symbolic term runtime
  |- rules and strategies
  |- domains and obligations
  |- compiler and bytecode machine
  |- module system
  `- standard libraries
       |- algebra
       |- calculus
       |- differential equations
       |- number theory
       |- probability
       `- symbolic-numeric compilation
```

This re-reads §10 without moving anything in it, which is the useful check on the
framing. M0 is the language. M1, M2, M3, and M5 through M7 are standard-library
milestones on a kernel that already exists, and M4 is the point where the library
starts being written in the language it belongs to. The sequence was already
shaped this way; naming the artifact makes the shape legible rather than
incidental.

### The traditions being combined

Each exists separately and each holds one part:

| Tradition | Its first-class symbolic objects |
|---|---|
| Computer algebra (Mathematica, Maple, SymPy) | mathematical expressions |
| Rewriting languages (Maude, ELAN) | equations, rules, strategies |
| Metaprogramming languages (Stratego, Rascal) | programs and syntax |
| Proof assistants (Lean, Coq, Agda) | propositions and proofs |
| Compiled functional languages (OCaml, Haskell) | none; they compile the rest |

Wolfram Language is the closest unified precedent, holding mathematical breadth
and expression-as-code uniformity together with compilation facilities, on
evaluation semantics that grew around a dynamic global evaluator. Maude is a
genuine executable symbolic language whose centre is rewriting logic and
verification. Stratego covers the transformation half and transforms programs.
Rascal treats source as data for analysis and transformation. What none of them
holds is a mathematical universe with assumptions, partial equality, and exact
arithmetic sitting on a compiled symbolic core.

---

## 1. Host language

### 1.1 The decision rule

The question is not which language is better at symbolic computation. Haskell is
probably better at writing the first evaluator. The question is what permanently
remains in the host language once Vieta implements its own mathematics.

> Choose the functional host when the host implementation is intended to remain
> the primary language in which symbolic algorithms and evaluator behaviour are
> developed.
>
> Choose the systems host when Vieta is intended to become that language, leaving
> the host as a performance-sensitive runtime.

For the stated ambition the second is more coherent, which settles it:

> **Haskell is best at the layer Vieta is meant to replace. Rust is best at the
> layer Vieta cannot replace.**

### 1.2 What permanently stays in the host

Interned expression store, memory management and structural sharing, evaluator
primitives and the bytecode machine, pattern matcher and rule index, native
library integration, cancellation and resource accounting, process and runtime
machinery, domain objects whose parameters are known only at runtime, parser and
printers.

These are systems problems. Not one of them is a place where a functional host
language's strengths are decisive.

### 1.3 Why the strongest Haskell argument evaporates

The best case for Haskell was typed mathematical domains: `Polynomial Rational`,
`Polynomial (FiniteField 7)`, `Matrix (Polynomial Rational)`.

Vieta users construct domains dynamically. `GF[p]` where `p` was computed at
runtime. `ExtensionField[Q, f]` where `f` came out of an earlier factorization.
At some point the host has to package these existentially, and once you cross
that seam the compile-time domain structure is gone from the orchestration layer
where it was supposed to help. What survives is typed algorithms *inside* each
domain implementation, and Rust does that with traits and private types.

Vieta needs runtime domain values regardless of host language. That is a property
of the mathematics, not of the implementation.

### 1.4 The store is where you spend twenty years

A semantic prototype can be a recursive algebraic data type. The real store needs
compact ids, hash-consing, mutable intern tables, arenas, lifecycle management,
concurrent caches, densely packed argument arrays, explicit allocation control,
and probably custom collection. In a garbage-collected functional host that
becomes `ST`, primitive arrays, strict fields, mutable hash tables, foreign
pointers, and unsafe operations behind a pure facade.

Possible, and it means using the language most intensively in the component where
it is least idiomatic, for the whole life of the project. That is a compounding
cost rather than a one-time one, and it is the strongest practical argument after
the self-hosting one.

### 1.5 The honest residual FFI cost

The first draft of this document argued that Haskell's inability to interrupt a
long foreign call was a Rust argument. **That argument is wrong and is withdrawn.**
Terminating arbitrary C mid-mutation is unsafe in any host. The real solutions are
cooperative cancellation inside the algorithm, splitting algorithms into
cancellable stages, and running dangerous long operations in worker processes.
See D22.

The residual cost that does survive is narrower and different in kind. GHC forces
a per-call choice: `unsafe` blocks the capability including garbage collection,
`safe` pays thread-handoff overhead. A FLINT-backed CAS makes enormous numbers of
*short* calls, and that is where the choice hurts. It is a throughput argument
about call volume, not a cancellation argument.

### 1.6 What is given up, and where it goes

**Lazy formal power series.** McIlroy's corecursive construction is genuinely
elegant and series machinery underlies limits, asymptotics, integration
heuristics, and special functions. Giving up host laziness relocates the
requirement rather than removing it: **Vieta itself must have first-class lazy or
coinductive structures** (D21). The self-hosting thesis recovers the advantage,
but only deliberately.

**Persistent structures for transformation search.** Still available in Rust with
explicit design. More work, no capability lost.

**Fluency.** A real cost on a twenty-year horizon, accepted knowingly rather than
argued away.

### 1.7 What is not an argument

Two claims from the first draft are withdrawn.

Foreign-call interruption, per §1.5.

Axiom's fragmentation as evidence against static domain typing. The causal claim
is not supported and is deleted. The argument in §1.3 stands on its own from the
runtime-domain observation, and attaching a contested history to a sound technical
claim only weakens it.

### 1.8 The premise, and keeping it honest

The whole argument rests on the self-hosting fraction being high. That fraction
is not uniformly distributed.

| Naturally Vieta | Contested | Naturally native |
|---|---|---|
| Rule corpora (derivative tables, trig identities, integral tables) | Gruntz limits | Tight loops over coefficient arrays |
| Classification and dispatch (what kind of ODE is this) | Risch (Hermite reduction, Rothstein-Trager are algorithmic; the tower is symbolic) | The matcher itself |
| Strategy drivers | Groebner strategy (selection is Vieta-able, the reduction loop is not) | Polynomial arithmetic |
| Special-function identities | Series manipulation | Linear algebra kernels |
| Assumption inference rules | | |
| User-facing domain definitions | | |

High by line count, lower by runtime. That is the correct outcome, and it means
the decision rule in §1.1 is falsifiable. Make it auditable rather than
permanent-by-assumption: **measure the Vieta-source fraction of the mathematical
library and the Vieta-level fraction of runtime, as a standing metric the test
suite reports from the first Vieta source file onward.**

A checkpoint at one milestone reports that the premise failed after the code that
failed it is already written. A standing number reports it while the ratio is
still moving, which is the only time the reading is actionable.

### 1.9 The measurements, taken in place

The language comparison arm is gone, and with it the reason to build a throwaway
prototype to carry it. The absolute numbers still inform the design, so take them
from the real components, before anything depends on the answers.

1. **Interned store.** The store *is* the first measurement. Hash-cons table, flat
   argument array, tagged small integers, `Word32` id space, built as the shipping
   component with its stress harness in-crate. Load a synthetic corpus of a few
   million nodes with heavy sharing. Measure construction throughput, memory per
   node, structural-equality throughput, whole-store walk time. Take the numbers
   while the tag layout is still free to move, which is the window D7 is about.
2. **Candidate index under load.** An index over a synthetic rule corpus in the
   low thousands, run against corpus (1). This measures the *index* rather than the
   matcher, which is the right thing to measure before D14's semantics are settled,
   and it is the number deciding whether Vieta can carry a Mathematica-scale rule
   library. It follows the store because it has nothing to run against until terms
   exist.
3. **FLINT round-trip under pressure.** A hundred thousand short-lived small
   `fmpz_poly` objects through your intended arena discipline. Watch resident
   memory across the run. This tests the arena design rather than a collector, and
   it is where the mistake in §7.5 shows up if you made it. It belongs with the
   `gr` capability audit (§5.2) rather than with (1) and (2): both size M2, both
   need FLINT built, and neither blocks slice 1, which carries no FFI.

### 1.10 What does not change with the language

The specification and the conformance suite remain the durable asset. For a
project measured in decades the artifact that survives is the one that is written
down, not the one that is compiled. Build the conformance suite as an
implementation-independent thing from M1.

The corollary about a second implementation is in §3.4.

---

## 2. The semantic model

### 2.1 Four meanings of equality

This is foundational and precedes every rewrite rule.

```
Structural      same interned expression, modulo binder representation
Domain          equal as elements of a selected algebraic structure
Provable        equal under the current assumptions and available theories
Extensional     same value at every point where both are defined
```

`(a^2-1)/(a-1)` and `a+1` are **domain**-equal in `Q(a)` and are not
**extensionally** equal, because they differ at `a = 1`. That single gap is where
most computer algebra becomes quietly dishonest, and Vieta's differentiating claim
is that the gap is recorded rather than crossed silently.

Two consequences.

**The taxonomy must surface in the language.** Users need distinct predicates,
and the default must be the conservative one. Mathematica has `Equal` and
`SameQ`, needs four, and the resulting confusion is permanent. Name all four,
document which one every operation preserves, and make the bare `==` the one that
cannot lie.

**Every algorithm declares which equality it preserves.** A rational-function
canonicaliser preserves domain equality and not extensional equality, and it says
so, and the difference becomes an obligation (§5).

### 2.2 Evaluation is three layers

Mathematica has one mechanism, rewrite until nothing changes. That single choice
produces its unpredictable termination, opaque performance, unsound caching, and
unspecifiable `Simplify`. Split it.

**Layer A, normalization.** Total, terminating, confluent, cheap, not arbitrarily
user-extensible. Flatten `Flat` heads, sort `Orderless` arguments canonically,
fold exact numbers, apply identity and annihilator laws, collect like terms. Runs
at construction. This is what makes `x + 2` and `2 + x` the same `ExprId`, and
therefore what makes structural equality mean anything.

`docs/layer-a.md` is the specification. It sharpens one word above: because
construction is bottom-up and every argument is already normal, Layer A is a
function applied once per node rather than a relation run to a fixpoint, so the
property it owes is canonicity of that function, and confluence of a rewrite
system over arbitrary terms is a larger obligation than the design incurs.

Layer A is **deterministic, terminating, versioned, and replayable**, and is
therefore **omitted from normal derivation traces**. Its effect is reconstructible
from the input plus the normalization version, so recording it by default is
waste. Omitted is not forbidden: a debugging mode, or a future certificate mode
that must exhibit every step to an external checker, may emit Layer A events. The
version stamp is what makes a replayed trace trustworthy, and it is why the
version is part of the decision rather than an implementation detail. This removes
most of the provenance volume problem before D24's sink design has to carry it.

**Layer B, evaluation.** The programming language. Definitions, application,
control flow, `Hold`, scoping, namespaces. Deterministic and specified. Not a
fixpoint search.

**Layer C, transformation.** The mathematics. Rule sets applied under explicit
strategies, each rule carrying a direction, a cost, and side conditions. Never
implicit, never global, never "until nothing changes" without a declared
termination argument.

`Simplify` then means something it can never mean in Mathematica: extraction of a
minimal-cost representative under a declared cost function from a declared rule
set. That is a specification rather than a pile of heuristics.

### 2.3 Rewrite results are guarded sets

A primitive rule application returns exactly one of three things:

```rust
enum RewriteResult {
    NoMatch,                            // the rule does not apply here
    Rewrote(GuardedSet),                // it applies; here is what follows
    Failed(OperationStatus),            // separately typed operational failure
}

struct GuardedSet { branches: NonEmpty<Branch> }

struct Branch {
    guard:      ObligationId,    // Obligation::True for unconditional application
    term:       TermId,
    derivation: DerivationRef,   // sink handle, not a materialised object
}
```

Five properties, each of which `Maybe (Term, [Condition], Derivation)` lacks.

**`NoMatch` differs from `Failed`.** "This rule does not apply here" and "this rule
applies and the computation could not proceed" are different facts with different
consequences for the caller. `Failed` carries an `OperationStatus` (§2.4), which is
a different type from a truth value.

**Results are disjunctive.** Solvers produce case splits natively. A rule firing
one way under `a != 0` and another under `a = 0` produces two branches, not one
term with an attached conjunction.

**One constructor, not two.** Unconditional application is the singleton branch
with guard `True`. Separate `Applied` and `Alternatives` constructors mean two code
paths that must agree about composition when you rewrite inside a branch, and they
will drift.

**Obligations are logical structure, not lists.** An `Obligation` supports
conjunction, disjunction, and branch selection:

```
a != 0  AND  x > 0
(a > 0  AND  branch = 1)  OR  (a < 0  AND  branch = 2)
```

A flat `[Condition]` encodes the first and not the second.

**The primitive rule does not decide what an unknown guard becomes.** A rule
reports a guarded set and stops. Whether an undischarged guard turns into a
`Piecewise`, whether both branches are kept and explored, whether the assumption
engine escalates a tier, whether the user is asked, or whether the rewrite is
abandoned, all belong to the surrounding evaluation strategy. Putting that
judgement inside the rule would hard-code one policy into several hundred rules
and make strategy experimentation impossible. Rules report; strategies decide.

### 2.4 Truth values and operation status are different types

Do not merge `Unknown`, `Undefined`, and `Failed`.

```
Truth      True | False | Unknown          "is this proposition true?"
Status     Success | OutsideDomain | Unable | Cancelled
           | ResourceLimit | InternalFailure
                                            "did this algorithm produce a result?"
```

FLINT's generic-ring layer makes exactly this split: predicates return
`T_TRUE`/`T_FALSE`/`T_UNKNOWN` while operations return statuses distinguishing
success from domain failure from inability. That an independent mature C library
converged on the same separation is good evidence it is the right one.

`IsZero` is a `Truth`, never a `Bool`. Richardson's theorem makes zero-equivalence
undecidable for expressions built from rationals, pi, `ln 2`, a variable, `exp`,
`sin`, and `abs`. A boolean signature is a claim the mathematics does not support,
and systems that make it are wrong somewhere and cannot say where.

**A rule never fires unconditionally on `Unknown`.** It returns the undischarged
obligation as a branch guard and stops. What happens next is the strategy's call,
per §2.3. This is the whole differentiator, and it is a discipline rather than a
feature, which is why it has to hold from rule one.

### 2.5 The assumption engine, three tiers, honestly incomplete

**Tier 1, the predicate lattice.** Sorts and attributes with precomputed
implication closure: complex, real, rational, integer, natural; positive,
negative, nonzero; even, odd; algebraic, transcendental. Constant-time lookup.
SymPy's ambition, done once. SymPy ran two assumption systems side by side for
years; never start a second.

**Tier 2, linear arithmetic.** `x > 0`, `y >= x`, `n >= 1` over exact rationals.
Simplex or Fourier-Motzkin for reals, Presburger for integers. Decidable, fast,
and it covers the large majority of real side conditions. PPL, cddlib, and
Normaliz are usable here.

**Tier 3, escalation.** Nonlinear real arithmetic to CAD or an SMT backend under
an explicit budget. Cylindrical algebraic decomposition is doubly exponential, so
this tier is a best-effort oracle with a timeout, recorded in the derivation as an
oracle step.

A condition that cannot be discharged flows outward into the result. An answer
with attached conditions is a correct answer.

### 2.6 Numbers

Exact by default. Integers and rationals on GMP or FLINT. Algebraic numbers as a
defining polynomial plus an isolating interval, not as radical expressions; that
is the representation supporting real root isolation, sign determination, and
exact comparison, and switching later is a library-wide rewrite (D17).

Ball arithmetic is the **default** inexact type (D18), not machine doubles. Arb
balls carry rigorous error bounds, so a numeric result is a proof of an enclosure.
Machine `Double` stays available as a distinct, explicitly requested type,
primarily a compiler target. Silent double coercion is how every other system
acquires its numerical embarrassments, and it is also what makes verification
cheap: residual checking with balls is a certificate.

### 2.7 Strictness, laziness, and quotation

**Eager call-by-value is the default** (D25). Ordinary Vieta programming uses
immutable bindings, lexical scope, closures, algebraic data types, structural
pattern matching, persistent collections, and higher-order functions, evaluated
strictly.

Global non-strictness is rejected for four reasons that compound in this
particular system: memory behaviour becomes hard to reason about in a program that
already holds a multi-million-node store; ordering of native calls stops being
locally visible, which matters when those calls mutate FLINT objects; compilation
gets harder exactly where §3.1 says it must not; and a symbolic language already
carries enough non-obvious evaluation order without adding a second source of it.

**Laziness is explicit and selective**, for formal power series, potentially
infinite sequences, delayed transformation search, and candidate solution streams
(D21). Recommendation, with the mechanism still open: prefer value-level thunks
and streams over an evaluation-mode annotation on bindings. A value keeps the
delay local and visible to the caller; an annotation contaminates the strictness
reasoning of everything downstream of it.

**Quotation is not laziness.** They are separate concepts and need separate
constructs:

```
quote { 1 + 2 }              a term, unevaluated because it is data
lazy  { expensive_search() } a computation, deferred because it may not be needed
```

Mathematica's `Hold` does both jobs, and the overloading is a real part of why its
evaluation semantics are hard to state. Vieta keeps holding constructs for faithful
representation of input (D11) distinct from the delay mechanism.

### 2.8 Syntax, terms, and values

Three representations, deliberately not unified:

```
Syntax    scoped program structure; what the parser produces and macros consume
Term      symbolic or mathematical expression; the interned DAG
Value     evaluated runtime object: closure, compiled function, dense matrix,
          domain element, native polynomial handle
```

The principle:

> **Everything is representable as a term. Not everything is stored as one.**

A dense FLINT matrix, a compiled function, and a ball are values. Forcing them
through the universal representation would make them slow for no gain, and it is
the mistake that makes SymPy's `Basic` uniformity expensive. Values reify into
terms on demand.

**The reification contract has to be decided rather than discovered.** Is
reification total, or may a value refuse? Is it injective, so that reifying and
re-evaluating yields an equivalent value? Does `reify(v1) == reify(v2)` imply
anything about `v1` and `v2`? What does structural equality mean between a value
and a term? These are the same class of question as §2.1's equality taxonomy and
deserve the same treatment: answered in writing before values exist, because
afterwards every answer is a migration.

**The trichotomy also settles a store-level macro question**, which is what makes
it load-bearing rather than descriptive. Racket-style hygiene needs syntax objects
carrying lexical scope information that accumulates during expansion. Interning
those in the term store would force a choice between scope-decorated nodes that
defeat sharing and mutation of interned structure, and both are bad. Keeping
Syntax a separate representation avoids the choice, and it is why §2.11 remains
available.

### 2.9 Five operations, distinct from the three layers

| Operation | What the user asks for | Mechanism |
|---|---|---|
| `eval` | execute a Vieta program | Layer B |
| `normalize` | deterministic structural canonicalization | Layer A |
| `simplify` | a preferable equivalent form under a context and cost function | Layer C |
| `prove` | establish a proposition | assumption engine (§2.5) |
| `approximate` | cross deliberately into numeric representation | exact/inexact boundary (§2.6) |

Two vocabularies on purpose. Layers describe where code lives; operations describe
what a user asks for. `prove` and `approximate` do not correspond to layers at all,
which is why one vocabulary cannot carry both. Keep the mapping explicit or the two
drift apart.

`prove` returns a `Truth`, never a `Bool`, and is paired with a `Status`. It must
never answer `False` when it means `Unknown`. That is the most abusable signature
in the system, because a user who reads `False` will act on the negation.

`approximate` as a named operation rather than an implicit coercion is what D18
buys, and it is why exact-to-double never happens silently.

Ordinary evaluation never launches `simplify`. Constructing `(x^2 - 1)/(x - 1)`
yields a normalized term that still has a singularity at `x = 1` (D11). The
information is discarded only by an operation the user asked for, and even then it
leaves an obligation (§5).

### 2.10 Functions, rules, and strategies are three kinds

```
Function    maps argument values to a result value
Rule        relates term patterns; returns a guarded set (§2.3)
Strategy    decides where, when, and in what order rules are tried
```

All three are first-class immutable values.

**Strategy as a value is the part usually missed.** When traversal and control are
hard-coded into the engine, rule ordering, package load order, and registration
order become the de facto strategy: accidental, unspecifiable, and liable to differ
between sessions. Stratego's answer is the vocabulary a simplifier wants anyway,
supplied as ordinary combinators:

```
strategy algebraic =
    bottom_up(repeat(trig_identities | rational_reduce | collect_terms))

rewrite(expr, using = algebraic)
```

This is the mechanism `Simplify` never had. §2.2's claim that simplification
becomes extraction of a minimal-cost representative under a declared cost function
from a declared rule set is only true if strategies are values; otherwise it is
aspiration.

A rule carries no traversal policy. **Rules report; strategies decide** now has two
readings and both hold: a rule decides neither what becomes of an undischarged
guard (§2.3) nor where it is applied.

### 2.11 Macros are separate from symbolic rewriting

Macros transform Syntax before evaluation. Rules transform Terms during it.
Different input representation, different phase, different hygiene requirement.

**Hygiene is required and cannot be retrofitted.** Every Lisp that began with
unhygienic substitution and later attempted hygiene found the existing macro corpus
already depended on capture. Take Racket's model: syntax objects carrying lexical
context, explicit expansion phases, module-level binding, and a deliberate escape
hatch for intentional capture.

The temptation specific to Vieta is real, because the system already has pattern
matching over expressions, so `macro` looks like a special case of `rule`. It is
not. A rule matching `x_` matches a term. A macro mentioning `x` has to know which
binding `x` refers to at the use site, and a term carries no lexical context to
consult. Raw symbol substitution is not a macro system; it is a bug generator with
good ergonomics.

Not slice 1. What slice 1 must not do is foreclose it, which §2.8 handles.

### 2.12 Effects are explicit

Tracked categories:

```
IO   Random   Time   MutableState   DynamicLookup   Native   Unsafe
```

Surface syntax need not be monadic; effect blocks or a `do` form are fine. What is
required is that the compiler knows where effects occur, because D20's world
discipline is enforceable only if `DynamicLookup` is something it can see.

**Open, and a real fork.** Whether effects are *checked* (inferred and verified, so
that a pure function calling an effectful one is an error) or *declared*
(annotated, trusted, used for optimization and reproducibility rather than
soundness). Checked is stronger and considerably harder on a dynamically symbolic
surface language. Declared is cheap and guarantees nothing. Decide before M4, when
compiled code makes it matter.

Consequence of D19 worth naming: session-as-a-value already makes top-level
evaluation pure, so effects have to be threaded through the session or handled at
its boundary. That is a bill D19 was always going to present, and paying it is part
of what reproducible sessions cost.

### 2.13 Dispatch is constrained by ownership

Multiple dispatch is right for mathematical operations: `add` over integers, over
`Polynomial(R)`, over `Matrix(R)`. Domains are values (D15), so dispatch is
value-level.

Constrain extension by ownership. A module may define a method when it owns the
operation, owns at least one of the domains or types involved, or imports an
explicit extension namespace. Julia names the violation type piracy, and its
consequence is behaviour that depends on which packages happen to be loaded, which
is precisely the property D13 and D20 exist to eliminate.

Prefer explicit rule sets to global extension for symbolic behaviour:

```
simplify(expr, using = QuantumMechanics.rules)
```

**Open: where the boundary between a method and a rule falls.** Users will ask this
constantly and an unanswered version accretes into inconsistency. The working
answer, to be tested against the library before the library is written: a method
resolves an operation on *values* whose domains are known; a rule transforms
*terms*, including terms whose domain is unknown.

---

## 3. Self-hosting and what it commits you to

The architecture is:

```
Vieta mathematics and transformation rules
            |
Vieta evaluator and bytecode
            |
Rust runtime, stores, domain adapters
            |
FLINT / GMP / MPFR
```

Three consequences that are easy to miss and expensive to retrofit.

### 3.1 The evaluator's speed becomes the CAS's speed

If most of the mathematical library is Vieta source, then Vieta's execution speed is
the system's speed for everything that is not a FLINT call.

This is where the Lisp-hosted systems have a structural advantage that is rarely
named. Maxima's and REDUCE's libraries are self-hosted **and natively compiled**,
because the host Lisp compiler compiles them for free. A Rust-hosted Vieta gets
self-hosting without that. The system whose library language is interpreted is
SymPy, and library-level slowness is its most persistent complaint.

So Vieta needs a compilation path for its own code. **The first implementation
target is a bytecode machine**, not native ahead-of-time compilation; a JIT is a
later option, not a prerequisite.

The risk is concentrated rather than uniform, which tells you what has to compile
and what does not.

| Executes through | Work |
|---|---|
| The native matcher, unchanged | Vieta-level declarative rule sets |
| A compilation path, required | Control flow, recursive strategies, classifiers, solver orchestration, series drivers |

Declarative rules stay fast because the matcher is native and always was. What
needs compiling is the imperative and recursive scaffolding around them, and it
needs it **before a substantial self-hosted mathematical library is built**, not
after, because the library's shape is influenced by what is affordable to write.

Taken literally, "before" means slice 1, and that is D33. A tree-walking evaluator
built at M0 and replaced when the machine arrives is the largest discardable
artifact in the plan, and every line written against it in the interval inherits
its performance shape. D20's world discipline is a compilation decision, so
nothing exercises it until compiled code runs; deferring the machine defers the
first honest test of the world model past several milestones of semantics written
on the assumption that it works. The machine is smaller than the matcher and the
ground is well trodden. Build it once, early, and never build the other one.

### 3.1.1 Immutable versioned worlds, not a ban on late binding

The naive form of this requirement is that no feature may require late binding.
That is too restrictive for an interactive symbolic language. It would foreclose
dynamic definitions, runtime package loading, reflection, dynamically supplied
rule sets, and evaluating an expression in a modified session, all of which Vieta
should have.

The commitment is narrower and is about visibility rather than prohibition (D20):

> Every dependency of compiled Vieta code is explicit, captured, or versioned.
> Compiled code executes against an immutable world snapshot. Redefinition creates
> a new world and either invalidates and recompiles dependent code or leaves
> existing compiled code attached to its original world. Explicit dynamic lookup is
> permitted; it is an optimization barrier and it is visible in the program
> semantics.

What is forbidden is **invisible or untrackable** late binding: a compiled function
whose behaviour silently changes because something it never named was mutated
elsewhere. Late binding that the program declares is a legitimate feature with a
known cost.

D13 (rule sets are values) and D19 (session state is a value) are what make worlds
cheap. A world is a session value plus the rule-set values reachable from it, so
snapshotting is already the natural operation rather than a mechanism bolted on for
the compiler. Both decisions were argued from soundness and reproducibility, and
they turn out to be the compilability decisions too.

### 3.2 Laziness has to reappear in Vieta

Series machinery is load-bearing for limits, asymptotics, integration, and special
functions. Choosing a strict systems host relocates that requirement into Vieta's
own language design rather than removing it. **Vieta needs first-class lazy or
coinductive structures** (D21). Candidate streams in transformation search want the
same machinery. Designed in, this recovers the best thing the functional host
would have given; left out, the capability is simply lost.

### 3.3 The runtime will not be compact

Store, evaluator primitives, bytecode machine, matcher, rule index, obligation
representation, derivation infrastructure, domain registry, FFI adapters,
cancellation and resource accounting, parser, two printers. That is a large
systems codebase.

This strengthens the language decision rather than weakening it, since a large
systems codebase is more clearly Rust's territory than a small one. Budget for it
honestly. Every project that promised a small trusted kernel grew one.

### 3.4 The executable specification belongs in Vieta

A separate reference implementation in a second host language is the wrong shape.
The failure mode is not the duplicated effort; it is that a reference
implementation nobody runs rots within a year, after which it is worse than
nothing because it still looks authoritative.

The variant worth keeping: once the language can carry it, write the executable
specification as a **metacircular reference evaluator in Vieta itself**. It runs on
the real kernel so it cannot rot, it is the most demanding possible test of the
surface language, and it is the strongest available evidence that Vieta expresses
its own semantics. The conformance suite is separate from it and comes earlier.

---

## 4. What to borrow, and what each source got wrong

**Mathematica.** Take: everything-is-an-expression; attribute-driven matching
(`Flat`, `Orderless`, `OneIdentity`, `Hold*`, `Listable`); sequence and
conditional patterns; `Hold` and controlled evaluation; upvalues, which are the
best extensibility mechanism in any CAS because a new head can declare how it
interacts with existing operators without modifying them. Reject:
evaluate-to-fixpoint as the single mechanism; global mutable definition tables;
implicit multiplication by juxtaposition; three overlapping scoping constructs;
undocumented auto-evaluation; ambient mutable session state; two equality
predicates where four are needed.

One apparent contradiction in that pair, resolved: attributes driving **matching
and normalization** are worth taking, and attributes driving **the evaluator's
control flow** are the complexity worth rejecting. `Flat` and `Orderless` tell the
matcher and Layer A what shape a head has, which is declarative and local.
`Hold*` telling the evaluator to suspend its own recursion in one of several
partially overlapping ways is control flow smuggled into a symbol's metadata.
Vieta takes the first and expresses the second through quotation and holding
constructs that are visible at the use site (§2.7). D36 makes "declarative and
local" concrete by splitting the attribute bag four ways: canonical-shape laws
belong to the operator identity and are immutable, while definitions, matching
policy, and notation are versioned world state.

**Racket.** Take the syntax model: syntax objects carrying lexical context, phase
separation between expansion and evaluation, module-scoped bindings, and hygiene
with a deliberate escape hatch. This is the reference design for §2.11, and the
thing to take is the *representation*, not the surface syntax. Reject the idea that
raw lists plus symbol names are an adequate representation of scoped program
structure, which is the shortcut every homoiconic language is tempted by and which
§2.8 exists to refuse.

**Julia.** Take interactive compiled execution, generic mathematical programming,
value-level multiple dispatch, versioned views of definitions (its world-age
mechanism is the closest existing analogue to D20), and direct native-library
interoperability. Reject unconstrained global method extension, per §2.13. Julia is
also the closest existing evidence that an interactive mathematical language can be
compiled rather than interpreted, which is the assumption §3.1 rests on.

**Lisp and term rewriting.** Take homoiconicity and macros, and take the theory
that CAS projects routinely ignore: termination orderings (LPO, RPO, KBO),
critical pairs and Knuth-Bendix completion, and the known complexity of
associative-commutative matching. Take Maude's separation of rules from the
*strategy* for applying them, and Stratego's strategy combinators (`topdown`,
`bottomup`, `innermost`, `try`, `repeat`). A simplification engine wants exactly
this vocabulary; Mathematica's lack of it is why its transformation control is a
pile of special cases.

Take Maude a second time, for the thing §0.6's diagram understates. Maude compiles
matching modulo associativity, commutativity, and identity, and that is the
hardest single piece of Vieta's kernel (D35). On the execution axis it sits
further toward compiled symbolic execution than its mathematical breadth suggests,
which makes it the implementation to read rather than the sibling to surpass. The
same holds for the rewriting-logic literature behind it: compiled equational
matching is a solved problem with a published record, and the failure mode
available here is deriving a weaker matcher from first principles and giving it a
standard name.

**Axiom and FriCAS.** Take the category and domain distinction, categories as
theories and domains as implementations, and the underlying insight that
correctness requires knowing which ring you are in. Reject putting the tower in a
static type system. Note the specific gap to avoid: Axiom had no good answer for
expressions whose domain is unknown, which is what users actually type. The
dynamic-surface plus typed-internals synthesis is the fix, and Magma and Sage
arrived at it independently.

**REDUCE.** Take the kernel idea seriously. REDUCE routes nearly all algebra
through one canonical representation, a rational function over a set of kernels,
where a kernel is any subexpression the polynomial machinery treats as opaque.
This is the bridge from universal expressions to typed domains and it is why
REDUCE is small and fast. See §5 for the refinement it needs.

**SymPy.** Take the ambition of pervasive assumptions, the acknowledgment that
faithful representation of unevaluated input matters, the `Basic`/`args`
uniformity, the codegen design. Reject expression nodes as host heap objects; a
constructor that simplifies; two assumption systems; `simplify()` as an
unspecified heuristic bag; a domain tower grafted on late; and an interpreted
library language with no compilation path (§3.1).

**Modern compiler IRs.** Take MLIR's dialect model: coexisting IR levels with
defined lowerings and shared infrastructure for passes and verification. That maps
onto universal expression, lowered to typed domain, lowered to native call, and it
gives you vocabulary for the boundaries.

Take **e-graphs and equality saturation** seriously; this is the most valuable
modern idea available to a new CAS and no major CAS uses it. An e-graph avoids
choosing a rewrite direction: saturate with known equalities, then extract the
best representative under a cost function. That dissolves the expand-versus-factor
dilemma. Caveats to know up front: saturation does not terminate under
associativity, commutativity, and distributivity together without care; extraction
with shared subexpressions is NP-hard; conditional rewriting under assumptions
requires e-class analyses. Herbie applied the same machinery to numerical
stability rewriting, which is differentiator 3, so M7 and M8 share more than they
appear to. Know where it is the wrong tool: never put a hundred-thousand-term
polynomial in an e-graph. Canonical forms for the algebraic layer, saturation for
the transcendental and heuristic layer.

**Proof assistants.** Take alpha-invariant term representation. Take the
kernel-and-elaborator split, which maps onto a small trusted canonicaliser plus
certificate checker and a large untrusted algorithm layer, and which makes
verification architectural rather than bolted on. Take proof terms as data: the
derivation graph *is* a proof term and should be replayable and checkable rather
than a human-facing log. Take Lean's `simp` with explicit simp-sets as a much
better specification of rewriting than `Simplify`. Take metavariables and
unification for solving. Take the definitional-versus-propositional equality
discipline, which generalises to §2.1's four-way taxonomy.

---

## 5. Dynamic surface meets typed domains

The most important interface in the system.

1. **Kernel selection.** Find the maximal subexpressions the target domain cannot
   represent. Those become generators.
2. **Domain inference.** Compute the smallest domain descriptor in which the
   expression is a polynomial or rational function over them. A lattice join over
   the leaves. Failure is a normal outcome, not an error.
3. **Lowering.** Construct the typed native object. Record every assumption the
   construction made.
4. **Compute** in the typed domain.
5. **Lifting.** Map back to expressions, resubstituting generators.
6. **Obligation discharge.** Assumptions from step 3 become obligations on the
   result, discharged against the assumption context or attached to the output.

Step 6 is where every other system is wrong. Leading-coefficient nonvanishing,
denominator nonzero, a parameter's sign, a branch choice: each is created at a
lowering boundary and each is a place existing systems drop information. Restated
against §2.1, lowering moves you from extensional equality to domain equality, and
the obligation is the record of the move.

### 5.1 Lowering takes a structure, not a flat kernel list

Naive kernels assume algebraic independence and that assumption is false.
`sin(x)` and `cos(x)` satisfy `s^2 + c^2 - 1 = 0`. Flat kernels produce
expressions that fail to simplify and equalities that fail to be detected.

The irreversible part is only that **the lowering interface takes a structure
carrying known relations** rather than a list. The taxonomy of relation theories
is not irreversible, and should not be designed up front. A polynomial ideal
captures algebraic relations only; Vieta will also meet differential relations,
analytic continuation and branch constraints, order relations, domain-dependent
identities, functional equations, periodicity, and conditional identities. A
pluggable theory interface designed before there are two working instances will be
the wrong interface.

Sequencing: build the algebraic ideal case concretely, build the differential
tower concretely (that is Risch), then extract the interface from two things that
work.

**Read Calcium rather than theorising.** It represents exact complex numbers using
algebraic and transcendental extension structures and returns `Unknown` when its
heuristics cannot prove an identity. It is a working implementation of this
design, and reading it beats reasoning about it.

### 5.2 FLINT's generic rings are an engine, not a constitution

FLINT's `gr` module is close to the value-level domain design: a domain descriptor
context, opaque elements valid in that context, a method table, capability
predicates, and generic fallback algorithms. Runtime contexts support things like
integers modulo a user-supplied `n`. This is strong independent evidence that
value-level domains are current CAS design rather than a workaround for a missing
type system.

Adopt it as an engine behind an abstraction boundary:

```
Vieta Domain
      |
Domain capability / operation interface
      |
FLINT-gr adapter  |  Vieta-native domain  |  later backend
```

Without the boundary Vieta inherits FLINT's evolving API, its domain taxonomy, its
C memory conventions, its error model, its current implementation limits, and
assumptions that may not match Vieta's surface semantics.

**The first serious technical investigation should be a FLINT `gr` capability
audit, not a new polynomial implementation.** What does it cover, what is stable,
where does its taxonomy diverge from what Vieta needs, and what is its ternary
predicate and status discipline. That audit sizes M2 and may remove months of work
from the critical path.

---

## 6. Expression store architecture

**Node layout.** `(Head: ExprId, Arity: Word32, ArgOffset: Word32)` into a flat
`Word32` argument array. Atoms in side tables: symbol ids, exact numbers (inline
when small, handles when large), machine literals, native handles.

The head is an ordinary id rather than a separate code space. A symbol head is a
symbol-tagged id, so the general case costs nothing, and computed heads
(`f[x][y]`, `Derivative[n][f]`) work without a second mechanism. A distinct
head-code space would foreclose them, and §4 takes exactly the Mathematica
constructs that need them.

**Interning.** Open-addressed hash-cons table keyed on head plus arguments,
yielding `ExprId`. Structural equality is a `Word32` comparison.

**Tagged ids.** Reserve tag bits for small integers, small rationals, and common
symbols so arithmetic on small values never touches the table. Retrofitting tag
bits after the id space is in use means auditing every construction and comparison
site.

**Garbage collection.** Roots are session bindings and active computation stacks.
Working assumption: epoch or region-based collection at explicit safepoints with
compaction that renumbers ids. Compaction implies an invariant to state now:
**nothing outside the store may cache a raw `ExprId` across a safepoint** (D8).
Clients, caches, the derivation store, and the native handle table all go through
handles that survive renumbering.

**Hash-consing gives structural identity and e-graph compatibility. It does not
give congruence closure.** Interning knows that `f(a)` and `f(a)` are the same
node. It does not know that `a = b` implies `f(a) = f(b)`.

Congruence closure additionally requires equivalence classes over terms, union and
rebuild machinery to merge them and restore invariants, and canonicalization of
nodes modulo those classes. The hash-cons table participates in the last of these,
since rebuilding works by rehashing nodes whose children's classes moved, but the
primary term store is not itself a congruence structure and must not be described
as one.

The right claim is the modest one: the ordinary term store provides structural
identity and an architecture compatible with a later e-graph.

The design consequence is D9: **the equivalence structure is a separate,
context-scoped layer over the store, never a mutation of store identity.**
Assumptions are contextual and temporary; structural identity must stay stable
while semantic equivalence varies by context. Keeping the identity model
compatible with e-class ids (that is, not assuming an `ExprId` is a canonical
representative that never changes) keeps equality saturation available as an
engine you add rather than a rewrite you dread.

**Binders.** The commitment is explicit binding forms with an alpha-invariant
internal representation, and named surface syntax with names preserved as
reconstruction hints. The **encoding stays open** (D6) for a concrete reason
neither obvious choice settles: de Bruijn *indices* make a subterm's
representation depth-dependent, so identical subterms at different depths do not
share, partially defeating hash-consing, while *levels* share better under a fixed
context and complicate open-term manipulation. Settle it after enumerating what
actually binds, and expect the enumeration to show that more things bind than you
assumed. Lexical arguments, integration variables, summation indices, pattern
variables, and sequence-pattern variables are all binders; global symbols, domain
generators, and generated indeterminates are not.

**Matching.** Two separable things, and only one of them is irreversible.

The **matching semantics contract** is irreversible and must be written down before
the rule corpus grows (D14). It has to answer, explicitly:

| Question | Why it binds |
|---|---|
| Completeness guarantee for AC matching | Incomplete matching breaks rules silently, the worst available failure mode |
| All results, or one selected result | An index supporting first-match-only cannot later serve all-matches |
| Match ordering | Whether it is specified, and if so by what, determines reproducibility |
| Deduplication | AC matching produces equivalent substitutions; who collapses them |
| Laziness | Whether a caller can take the first match without paying for the rest |
| Behaviour under resource limits | A truncated match set must be distinguishable from a complete one |

That last row is easy to skip and expensive to skip. A matcher that silently
returns fewer results under pressure turns a resource limit into a wrong answer.

The **index** is replaceable engineering behind that contract. `RuleSet` is a
value, `RuleSet` owns a pattern index, and matcher semantics do not depend on which
index is installed. The first index can be `head -> arity class -> candidate
rules`; a discrimination net or a family of specialised indexes comes after the
pattern semantics stabilise. Associative, commutative, flat, sequence, optional,
typed, and conditional patterns all constrain which schemes are viable, which is
exactly why the structure should follow the semantics rather than lead it.

**Serialization.** One binary format for three uses: wire protocol, session
persistence, on-disk cache. Topologically ordered node table, symbol table, number
blob, shared-node back-references, optional side tables for obligations and
derivations. Define it in slice 1 even though the first REPL sends text.

**Concurrency.** The hash-cons table becomes a contention point once anything is
parallel. Plan for sharded tables or a lock-free insert path.

---

## 7. Native dependencies and FFI discipline

### 7.1 Foundational

**GMP** for bignums. **FLINT 3 or later**, which absorbed Arb (ball arithmetic),
Antic (number fields), and Calcium (exact real and complex arithmetic with zero
testing), and added the `gr` generic-rings layer. One dependency covering `fmpz`,
`fmpq`, dense and sparse polynomials over Z, Q, and finite fields, multivariate
polynomials, matrices, factorization, LLL, rigorous balls, number fields, exact
constants, formal power series, and ternary predicates.

Two parts deserve specific attention. **Calcium** does exact zero testing over a
large class of constants, and zero recognition is the undecidable core of symbolic
simplification. **`gr`** is the subject of §5.2's capability audit.

### 7.2 Strong candidates

**PARI/GP** for algebraic number theory: class groups, elliptic curves, number
field arithmetic, L-functions. Its `avma` stack model is hostile to embedding,
which makes it the strongest argument for the arena discipline in §7.5.
**msolve** or **Singular** for Groebner bases and polynomial systems; msolve is
modern and strong on real solutions. **OpenBLAS/LAPACK** and **SuiteSparse** for
numeric and sparse linear algebra. **Cranelift** or **LLVM** for the compiler
backend; Cranelift is easier to embed and JITs quickly, LLVM produces better code.
Emit your own typed numeric IR and treat both as backends alongside C source
output. Do not couple the CAS to either.

### 7.3 Optional oracles

**Z3** or **CVC5** for hard side conditions, marked as oracle steps in the
derivation. **PPL**, **cddlib**, or **Normaliz** for tier-2 polyhedral reasoning.
A CAD implementation for real quantifier elimination when you will pay for it.

### 7.4 Skip

**NTL** is largely subsumed by FLINT. **GiNaC** is a competing architecture rather
than a component. Anything that insists on owning the process, initialising global
state, or installing signal handlers belongs behind a subprocess boundary.

### 7.5 Rust removes the collector, not the design problem

The obvious Rust binding is per-object RAII:

```rust
struct FlintInteger { raw: fmpz }
impl Drop for FlintInteger {
    fn drop(&mut self) { unsafe { fmpz_clear(&mut self.raw) } }
}
```

That is correct for the handful of long-lived objects and wrong in inner loops,
where you pay construction and destruction per element and abandon FLINT's
in-place idiom entirely. Rust makes the pattern that does not scale easy to write,
which is worth naming because it is exactly the shape people reach for when
arguing that Rust's FFI is nicer.

The discipline that scales, in any host:

**Put the boundary at the algorithm, not at the object.** Transfer a whole problem
into native representation, run the whole algorithm there, transfer the result
back.

**For objects that persist across many operations**, such as a matrix over `Q[x]`
the user manipulates over twenty REPL steps, use a kernel-owned handle table:
native objects live in a registry with lifetimes tied to store safepoints, freed
in bulk under your control. RAII wraps the arena, not the element.

**Every long native call needs a cancellation path**, and that is an architectural
decision rather than a language one (D22).

---

## 8. Irreversible decisions

Full register with alternatives and reversal costs in `decisions.md`.

| # | Decision |
|---|---|
| 1 | Host language is Rust; the mathematical library self-hosts in Vieta |
| 2 | Four meanings of equality, surfaced in the language |
| 3 | Rewrites return `NoMatch`, a guarded result set, or a typed operational failure |
| 4 | Truth values and operation status are different types |
| 5 | `Conditional` and `Piecewise` are core constructors with algebra |
| 6 | Explicit binding forms with alpha-invariant representation; encoding open |
| 7 | Hash-consing, id space, tag bits |
| 8 | `ExprId` stability across GC, and the external-handle rule |
| 9 | Semantic equivalence is a context-scoped layer, never store identity |
| 10 | Layer A scope, versioning, and omission from normal traces |
| 11 | Construction is pure; simplification is never automatic |
| 12 | Two printers: canonical lossless, and human pretty |
| 13 | Rule sets are values |
| 14 | Matching semantics contract is fixed; the index is replaceable |
| 15 | Domain descriptors are values; FLINT `gr` is an engine behind a boundary |
| 16 | Lowering takes a relation-carrying structure, not a flat kernel list |
| 17 | Algebraic numbers as defining polynomial plus isolating interval |
| 18 | Ball arithmetic is the default inexact type |
| 19 | Session state is a value; the REPL is a fold |
| 20 | Compiled Vieta code runs against immutable versioned worlds |
| 21 | Vieta has first-class lazy or coinductive structures |
| 22 | Interrupt, fuel, and progress live in the evaluation context |
| 23 | One binary store-segment format for wire, disk, and cache |
| 24 | Derivation is sink-based, never materialise-then-discard |
| 25 | Vieta is a strict expression-oriented functional-rewrite language (§0.5) |
| 26 | Syntax, Term, and Value are distinct representations (§2.8) |
| 27 | Five named operations, never merged into one (§2.9) |
| 28 | Functions, rules, and strategies are three first-class kinds (§2.10) |
| 29 | Macros are hygienic, phased, and separate from rewriting (§2.11) |
| 30 | Effects are explicit and visible to the compiler (§2.12) |
| 31 | Multiple dispatch is constrained by ownership (§2.13) |
| 32 | Internal mutation is permitted and must be unobservable |
| 33 | Layer B is compiled from slice 1; there is no tree-walking evaluator |
| 34 | A term is one kind of runtime value (§0.6) |
| 35 | Term construction and destructuring compile (§0.6) |
| 36 | A term is an element of a quotient algebra, and the theory is carried by its operators |

Numbering is append-only from this revision onward. D25 is the constitutional
statement and logically precedes D1; it is numbered last to keep existing citations
stable, which is worth more than a tidy ordering.

---

## 9. The first vertical slice

**From the original list:** lexer, parser, printer, symbols, exact integers and
rationals, universal expression representation, canonical arithmetic, evaluation,
definitions, patterns and rules, substitution, differentiation, enough assumptions
to show conditional simplification, REPL.

**Additions, each about foreclosure rather than features:**

- **One binding form** with alpha-invariant representation behind a traversal API,
  so the encoding stays swappable while the semantics get exercised.
- **Two printers.** `parse(canonicalPrint(t)) == t` as a property test from the
  first commit. The pretty printer only needs `parse(prettyPrint(t))` to be
  alpha-equivalent, and it may invent binder names, drop redundant parentheses,
  choose infix forms, and elide annotations. One printer cannot serve both roles
  without eventually constraining both.
- **The four-way equality taxonomy** in the language, with the conservative
  default.
- **`Truth` and `Status` as separate types** from the first line that needs
  either.
- **Guarded result sets** as the rewrite return type, exercised by at least one
  rule that produces two branches.
- **Obligations as logical structure**, exercised by one disjunctive obligation.
- **Conditional results three ways**: `sqrt(x^2)` with no assumptions, under `x`
  real, and under `x >= 0`, each returning a different answer with its condition.
- **Rule sets as values**, passed explicitly, with a named default.
- **Derivation sink** on differentiation and one conditional simplification, with
  the sink off by default and Layer A omitted from normal traces but reachable
  under a debug mode.
- **Hash-consed store with tagged small integers**, plus a stress test measuring
  sharing and memory per node.
- **Binary store-segment serialization** with a round-trip test.
- **A candidate index behind the matcher contract**, with the contract written
  down even though the first index is trivial.
- **Fuel and abort in the evaluation context**, checked in the bytecode dispatch
  loop and the rewrite loop.
- **Session state as a value**, with save, restart, resume.

**From the constitution (§0.5), the parts that must exist in slice 1 because they
are foreclosed otherwise:**

- **Strict evaluation with lexical scope and closures.** The default, established
  before any code depends on an evaluation order.
- **Layer B compiled, not interpreted** (D33). Syntax lowers to bytecode and the
  machine runs it. There is no tree-walking evaluator to replace later, and D20's
  world model gets its first exercise here rather than at M4.
- **Quotation as a construct distinct from the delay mechanism** (§2.7), even
  though slice 1 has nothing to delay yet. The two must never share a keyword.
- **The Syntax / Term / Value distinction visible in the implementation** (§2.8),
  even with only one or two kinds of Value. A parser producing terms directly is
  the shortcut that forecloses hygienic macros.
- **The reification contract written down**, even if the only reifiable values are
  trivial.
- **`eval`, `normalize`, and `simplify` as separately named entry points** (§2.9),
  with `prove` and `approximate` reserved rather than implemented. What matters is
  that they never collapse into one.
- **Strategies as values with two or three combinators.** Slice 1 has rules, so it
  needs somewhere for traversal policy to live that is not the engine.

**Deliberately not in slice 1, and none of it foreclosed:** hygienic macros
(§2.11), the effect system (§2.12), multiple dispatch (§2.13), laziness (§2.7).
Each has a written contract before it has an implementation.

**Leave out:** the domain tower, all FFI, native code generation and JIT, the R
DSL, e-graphs, and every mathematical algorithm beyond differentiation.

### 9.1 The acceptance demo

A script, run end to end:

1. Define a symbolic function with rules; call it.
2. Add a user rule that changes the result of a built-in simplification, proving
   library and user code use one mechanism.
3. Hold an expression and inspect its canonical form.
4. Show `x + 2` and `2 + x` are the same `ExprId`.
5. Show `sqrt(x^2)` returning three different answers under three assumption
   contexts, each with its condition.
6. Print the derivation for (5), showing which rule fired and which condition was
   discharged by which tier.
7. Show a rewrite returning **two guarded branches** rather than picking one.
8. Show a computation returning a **condition** rather than silently assuming,
   because the assumption was absent.
9. Show that `(a^2-1)/(a-1)` reduces to `a+1` **with `a != 1` recorded**, and that
   the four equality predicates give different answers on the pair.
10. Serialize the session, restart the process, resume, continue.

Items 8 and 9 are what no other system can do and they are the whole thesis. Item
10 is nearly free if D19 holds and impossible otherwise, which makes it a good
test of whether you actually did it.

### 9.2 The R client

Design the wire protocol in slice 1; write a *minimal* R client in slice 2 or 3,
before the R DSL is nice. A second frontend is the only thing that reliably forces
the kernel to have no hidden host coupling, and finding that coupling in month
four is much cheaper than in year four. Framed binary store segments with a small
envelope, on a multiplexed transport carrying abort and progress out of band.
Jupyter's channel model is a reasonable reference for message shape.

---

## 10. Milestones

Sizing bands assume solo work at a serious sustainable pace. Variance past M3 is
large.

**M0. Kernel and language.** Slice 1, the bytecode machine included (D33).
*Months.*
Unlocks everything. Over-investment here pays back longest, per §1.8.

**M1. Exact arithmetic and canonical rational forms.** FLINT-backed integers and
rationals. Canonical polynomial and rational-function normal form over a
relation-carrying generator structure. Structural zero testing over that fragment.
The conformance suite starts here as an implementation-independent artifact.
*Months.*
Unlocks the largest single correctness win available, and gives the lowering
machinery a real target.

**M2. Domain descriptors and the native bridge.** Opens with the FLINT `gr`
capability audit (§5.2). Value-level domain tower behind the capability interface,
handle table, arena discipline, cancellation strategy decided. Polynomial GCD,
factorization over Z, Q, and GF(p), resultants. *Months.*
**Certificate emission starts here as a policy, not a later phase.** Factorization
checks by multiplying back, GCD by cofactor divisibility. Both nearly free, and
establishing the policy before the algorithm count grows is what makes
verification reachable.

**M3. Assumptions and conditions, full engine.** Tier 1 lattice with precomputed
closure, tier 2 linear arithmetic, tier 3 escalation with budgets. `Piecewise`
algebra. Obligation propagation through every existing rule. *Months to a year.*
From here Vieta does something no other system does.

**M4. The first self-hosted library.** The derivative table, trig identities, and
the first classification logic move into Vieta source, on the machine M0 already
built. *Months.*
Placed before the heavy mathematics deliberately: the library that gets written
after this point gets written in Vieta, and the earlier that switch happens the
less native code has to be migrated later. With the machine in M0 the switch can
begin earlier still, and the standing self-hosting metric (§1.8) is what reports
whether it is happening.

**M5. Calculus.** Limits by the Gruntz algorithm rather than heuristics. Lazy
formal power series, exercising D21. Integration: Risch for the rational and
transcendental-elementary cases, plus a Rubi-style rule corpus written in Vieta.
*A year or more.*
By here the standing self-hosting metric (§1.8) has a long enough history to read
as a trend rather than a reading. If the Vieta-source fraction is flat, D1's
premise failed and this is the last milestone at which acting on that is cheap.

**M6. Equation solving.** Univariate over radicals plus `RootOf` with real root
isolation. Linear systems over domains. Triangular decomposition and Groebner via
msolve or Singular. *Months to a year.*
The rule: `Solve` never silently drops solutions and never silently assumes a
parameter is nonzero. Every branch is enumerated or attached as an obligation.
This is where M3 pays off visibly, and where guarded result sets stop being
theoretical.

**M7. Linear algebra over domains.** Fraction-free elimination (Bareiss), Smith
and Hermite normal forms, characteristic polynomials, symbolic eigenstructure,
then ball-backed numeric linear algebra with rigorous bounds. *Months.*

**M8. Equality saturation.** E-graph engine with cost-driven extraction and
assumption-aware e-class analyses, for the transcendental and heuristic layer, as
a context-scoped structure over the store per D9. Canonical forms retain the
algebraic layer. *Months to a year, with research risk.*

**M9. Symbolic-numeric compiler.** Typed numeric IR, CSE, sparsity, Jacobians and
Hessians by symbolic forward and reverse mode on the DAG, Herbie-style stability
rewriting reusing M8's e-graph, backends to C, Cranelift JIT, and LLVM. *Months to
a year.*

**M10. Verification as a maturing policy.** Certificates per algorithm class:
factorization by multiplication, integration by differentiation and zero testing,
solving by substitution, GCD by Bezout cofactors, linear algebra by ball
residuals, plus randomized exact identity testing via Schwartz-Zippel at random
points modulo random primes. Lean export for a subset much later, without making
theorem proving a prerequisite. *Continuous from M2.*

**M11. Metacircular specification and packaging.** The executable spec in Vieta
(§3.4), package system, namespaces, documentation tooling. *Continuous from M5.*

**M12. Frontends and scale.** Rich R DSL, Python client, notebook protocol, remote
kernels, parallel and distributed evaluation.

**M13. Breadth.** Special functions, integral transforms, differential equations,
probability, number theory via PARI, tensors.

On differential equations, since you named them as an example: build on the
algorithmic literature rather than a method zoo. Differential algebra (Ritt and
Kolchin, characteristic sets), Lie point symmetries, Kovacic for second-order
linear, Prelle-Singer for first-order. That base improves by adding theory rather
than special cases, which is the difference between a library that compounds and
one that accretes.

---

## 11. Novel, and known traps

### 11.1 Genuinely novel

**Pervasive conditional validity.** Mathematica has `ConditionalExpression`, Maple
has `assuming`, SymPy has assumptions. None is pervasive and none is sound in the
sense that a rule cannot fire on an undischargeable condition.

**The four-way equality taxonomy surfaced in the language.** Proof assistants
distinguish definitional from propositional equality. No CAS gives users the
distinction between domain and extensional equality, which is precisely the one
that matters for cancellation.

**Checkable derivation graphs.** No CAS makes every transformation produce a
replayable, checkable certificate.

**Equality saturation as the simplification engine.** Not done in any CAS.

**Ball arithmetic as default inexact semantics**, integrated with symbolic zero
testing. Sage and FLINT have the pieces; nobody makes it the default.

**Stability-aware symbolic-to-numeric compilation.** Herbie exists separately; no
CAS integrates it.

**Session state as a value.** Reproducible, forkable, replayable sessions.

### 11.2 Known traps

1. **`simplify()` as an unspecified heuristic bag.** Every CAS has one; every CAS
   regrets it. The §2.2 split plus cost-driven extraction is the escape.
2. **Zero testing.** Richardson's theorem. Three-valued always, with rules
   obligated to handle `Unknown`.
3. **Branch cuts and multivalued functions.** `sqrt`, `log`, `arctan`, `x^y`. Well
   studied rather than fresh: the Corless-Jeffrey unwinding number is the standard
   tool, the alternative is representing multivalued results as sets or `RootOf`.
   Read that literature before designing the functions.
4. **Domain tower rigidity from static typing.** §1.3.
5. **Ad-hoc coercion between domains.** Sage's coercion model is a research
   contribution and still has sharp edges. The trap is pairwise rules that are not
   confluent, producing an `a + b` that differs from `b + a`. Coercion is a join in
   a lattice with a uniqueness argument; pairwise special cases are forbidden.
6. **Global mutable rule tables making caching unsound**, and now also making
   compilation impossible. D13, D20.
7. **Intermediate expression swell.** The actual enemy in Groebner bases,
   integration, and resultants. No architecture removes it; you need modular,
   p-adic, and evaluation-interpolation algorithms throughout, which is the deepest
   reason FLINT matters.
8. **An interpreted library language.** SymPy. §3.1. The near-miss version is
   building a tree-walking evaluator for now and scheduling the compiler for
   later. By the time later arrives the library exists and its shape was set by
   what the interpreter made affordable. D33.
9. **Building the notebook UI before the semantics.** This has consumed years of
   more than one project.
10. **Reimplementing bignums and polynomial arithmetic.** Already rejected. Stay
    rejected.
11. **Assuming rewriting terminates.** Associativity, commutativity,
    distributivity, and two hundred of your own rules do not terminate together.
    The trap is discovering it at rule two hundred. Termination orderings from day
    one, or e-graphs, which sidestep by declining to commit.
12. **Two assumption systems.** SymPy lived with this for years.
13. **Per-object FFI wrappers in hot paths.** §7.5. Easy to write in Rust, and the
    cost appears only at scale.
14. **Naive rule matching.** Linear scan walls out sooner than expected. The
    contract in D14 is what keeps the fix cheap.
15. **Printer and parser drift.** Two printers with the canonical one under a
    round-trip property test from the first commit, or they diverge permanently.
16. **Bootstrapping the library in the surface language at the wrong time.** Too
    early and the language is not ready, so you rewrite; too late and it never gets
    exercised, so it stays bad. M4's placement is the answer: rule-based parts move
    to Vieta as soon as the bytecode machine exists, algorithmic parts stay native.
17. **Interruption as an afterthought.** D22.
18. **Unhygienic macros.** They work until the day a user names a local variable
    the same as something a macro expands to, and by then the corpus depends on
    capture. There is no cheap fix and no early warning. §2.11.
19. **Effects discovered rather than declared.** A cache that memoizes a function
    which reads the clock is wrong in a way that reproduces intermittently. The
    compiler has to know, which means the language has to say. §2.12.
20. **Three extensibility mechanisms with no stated boundary.** Vieta has dispatch
    methods, rewrite rules, and rule sets. Each is right for something. Without a
    written answer to "method or rule", the library accretes both for the same job
    and the answer becomes whichever the author preferred that week. §2.13.
21. **Compiling the control flow and interpreting the destructuring.** The subtler
    relative of trap 8, and it survives D33. A machine that compiles recursion,
    branching, and calls, then reaches term patterns through an interpretive call
    into the native matcher, leaves the characteristic function of the self-hosted
    library half-compiled at the half where the time goes. D35.
22. **Normalized terms outliving the laws they were normalized under.** Compiled
    code that assumed `Orderless` on `Plus` can be invalidated and recompiled. A
    node already flattened under `Flat` cannot: it is in the pool, shared, and held
    by live sessions and caches, with no recompile step for data. Dependency
    capture reaches code and not data, which is why D36 puts the shape laws in the
    operator identity and forbids the redeclaration instead of managing it.
23. **Single-maintainer project death.** Named even for a deliberately solo
    project. Maxima and REDUCE survived by being ported and cultivated by small
    niche communities; Axiom fragmented into forks. For a solo project the
    mitigation is not recruiting, it is making the artifact survivable: a written
    specification, a large conformance suite, a documented architecture, and a
    permissive license. Those are what let someone pick it up, including you after
    a three-year gap.

---

## 12. The opening sequence

**The specification block, ahead of the code that depends on each entry.**

- the Layer A normalization specification, written as `docs/layer-a.md`: the
  signature vocabulary, the canonical order, the pass sequence, the termination
  argument, and the canonicity theorem with the point where completeness stops;
- the enumeration of what binds, which the store's traversal API has to cover
  (D6);
- the four-way equality taxonomy with the language-level predicates;
- the `RewriteResult` and `Obligation` types;
- the `Truth`/`Status` split;
- the matcher contract;
- the constitutional statement and what it rules out (§0.5);
- the Syntax / Term / Value boundary and the reification contract (§2.8);
- the signatures of the five operations, including what `prove` may and may not
  return (§2.9);
- the function / rule / strategy split and the initial strategy combinators
  (§2.10);
- the value kinds the machine computes with (D34), which precede the instruction
  set because every opcode signature quotes them, and which say nothing about how
  a value is represented;
- the bytecode instruction set and calling convention (D33).

The block grew when the language design settled, and this is the right place for
it to grow. It is the smallest set of documents with the largest blast radius, and
it is what you will want in hand when you are debugging a non-confluence in year
three. Not every entry gates the first line of code: the Layer A specification
gated the store and is written, the binder enumeration gates the store's
traversal API, the value kinds gate the instruction set which gates the compiler,
and the matcher contract has until the rule count reaches the low hundreds.

**The spine, in order, each piece shipping rather than prototyping.**

```
store  ->  Syntax and parser  ->  compiler  ->  bytecode machine
                                                     |
                                        canonical printer, round-trip test
```

The store comes first because everything else holds ids into it, and because the
id space, the tag bits, and the safepoint invariant are the decisions that turn
into a whole-codebase audit if they move later (D7, D8). It carries its own
measurements (§1.9) and its tag layout is settled from them before anything else
holds an id.

The parser produces Syntax and never terms (D26); a parser emitting terms directly
is the shortcut that forecloses hygienic macros. The compiler targets Syntax, so
surface-syntax churn reaches the parser and stops there.

The first demonstrable result is a Vieta expression that parses, compiles, runs on
the machine, and prints. Layer A lands next, and with it `x + 2` and `2 + x`
collapsing to one `ExprId`. Then rules, guarded sets, and strategies, on a
language that already runs.

**In parallel, whenever it fits.** The FLINT `gr` capability audit (§5.2) and the
arena measurement (§1.9 item 3). Neither has a dependency on the spine, both need
FLINT built, and together they size M2.

Everything after that is §9.
