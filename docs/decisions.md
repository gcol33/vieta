# Vieta: Irreversible Decision Register

Revised 2026-07-27. Renumbered once in this revision to follow the order of
`architecture.md`; references to earlier numbering do not carry over.

**Numbering is append-only from here.** D25 through D32 record the language
constitution and logically precede D1. They are numbered last so that existing
citations stay valid, which is worth more than a tidy reading order. Read D25
first.

Decisions that cannot be changed cheaply once mathematical algorithms are written
against them. Each entry records the decision, the alternatives considered, why
reversal is expensive, and current status.

Status values: **Decided** (settle before slice 1 code), **Open** (needs a call
before the named milestone), **Deferred** (safe to postpone, tracked so it is not
forgotten).

---

## D1. Host language is Rust; the mathematical library self-hosts in Vieta

**Decision.** The permanent host layer is Rust. Vieta itself becomes the primary
language in which symbolic algorithms, transformation rules, and solver strategy
are written.

**Decision rule.** Choose the functional host when the host implementation is
meant to remain the language symbolic algorithms are developed in. Choose the
systems host when Vieta is meant to become that language, leaving the host as a
performance-sensitive runtime. The second matches the stated ambition.

> Haskell is best at the layer Vieta is meant to replace. Rust is best at the layer
> Vieta cannot replace.

**What permanently stays in the host.** Interned store, memory management and
structural sharing, evaluator primitives, bytecode machine, matcher and rule
index, native library integration, cancellation and resource accounting, runtime
machinery, runtime-parameterized domain objects, parser and printers.

**Alternatives rejected.** Haskell, whose strongest claim (compile-time typed
mathematical domains) evaporates because domain parameters are runtime values, and
whose least idiomatic territory is the store, the single most-touched component.
OCaml, which loses less but loses the same argument. A two-implementation route
(reference in one language, production in another), rejected because a reference
implementation nobody runs rots within a year and is then worse than nothing; the
executable specification belongs in Vieta (D-note below).

**Arguments explicitly withdrawn.** Foreign-call cancellation is not a language
argument: terminating arbitrary C mid-mutation is unsafe in any host (D22). Axiom's
fragmentation is not evidence about static domain typing; the causal claim is
unsupported and deleted.

**Residual honest cost of the rejected option.** GHC forces a per-call choice
between `unsafe` (blocks the capability including GC) and `safe` (thread-handoff
overhead). A FLINT-backed CAS makes enormous numbers of *short* native calls, so
this is a throughput cost proportional to call volume.

**Reversal cost.** Total. This is the one decision with no partial retreat.

**Falsifiable premise.** The argument rests on the self-hosting fraction being
high, and that fraction is not uniform: rule corpora, classification, strategy
drivers, and identity tables are naturally Vieta, while tight coefficient loops,
the matcher, and polynomial arithmetic are naturally native. **Audit at M5**:
measure the Vieta-source fraction of the library and the Vieta-level fraction of
runtime. If the first is low, the premise failed.

**Status: Decided.**

---

## D2. Four meanings of equality, surfaced in the language

**Decision.** Vieta distinguishes four relations, before simplification rule one:

```
Structural      same interned term, modulo binder representation
Domain          equal as elements of a selected algebraic structure
Provable        equal under the current assumptions and available theories
Extensional     same value at every point where both are defined
                (equality as partial functions, with domains of definition)
```

`(a^2-1)/(a-1)` and `a+1` are domain-equal in `Q(a)` and are not extensionally
equal, differing at `a = 1`.

**Language-level consequences, which are the point.** All four are user-visible
predicates with distinct names. The bare `==` is the conservative one that cannot
lie. Every algorithm and every rule declares which equality it preserves, and a
transformation that preserves only the weaker relation emits the gap as an
obligation (D16, §5 of `architecture.md`).

**Alternatives rejected.** Two predicates (Mathematica's `Equal` and `SameQ`),
which is the current state of the art and is two short. Leaving the distinction
implicit in the implementer's understanding.

**Reversal cost.** Every rule has already assumed one reading. Retrofitting means
re-auditing the entire library to determine which relation each transformation
actually preserves, which is not mechanically checkable.

**Status: Decided.**

---

## D3. Rewrite results are `NoMatch`, a guarded set, or a typed failure

**Decision.** A primitive rule application returns exactly one of three things:

```rust
enum RewriteResult {
    NoMatch,
    Rewrote(GuardedSet),
    Failed(OperationStatus),
}

struct GuardedSet { branches: NonEmpty<Branch> }
struct Branch { guard: ObligationId, term: TermId, derivation: DerivationRef }
```

Unconditional application is the singleton branch with guard `True`. An
`Obligation` is logical structure supporting conjunction, disjunction, and branch
selection, not a flat list.

**Rules report; strategies decide.** A primitive rule does not decide what an
undischarged guard becomes. Whether it turns into a `Piecewise`, whether branches
are kept and explored, whether the assumption engine escalates a tier, whether the
user is consulted, or whether the rewrite is abandoned, belongs to the surrounding
evaluation strategy. Embedding that judgement in the rule would hard-code one
policy into several hundred rules and make strategy experimentation impossible.

**Alternatives rejected.** `Maybe (Term, [Condition], Derivation)`, which conflates
no-match with operational failure, cannot express case splits, and cannot express
disjunctive obligations. Separate `Applied` and `Alternatives` constructors, which
create two code paths that must agree about composition under nested rewriting and
will drift.

**Reversal cost.** Every rule, every strategy, every consumer of a rewrite.

**Status: Decided.**

---

## D4. Truth values and operation status are different types

**Decision.**

```
Truth    True | False | Unknown
Status   Success | OutsideDomain | Unable | Cancelled
         | ResourceLimit | InternalFailure
```

`Unknown`, `Undefined`, and `Failed` are never merged. `IsZero` is a `Truth` and
never a `Bool`.

**Justification.** Richardson's theorem: zero-equivalence is undecidable for
expressions built from rationals, pi, `ln 2`, a variable, `exp`, `sin`, and `abs`.
A boolean signature asserts something the mathematics does not support. FLINT's
generic-ring layer makes the same split independently (`T_TRUE`/`T_FALSE`/
`T_UNKNOWN` for predicates, status codes for operations), which is good evidence
it is the right one.

**Consequence.** A rule never fires unconditionally on `Unknown`. It returns the
undischarged obligation as a branch guard and stops, per D3.

**Reversal cost.** Every rule that branches on zero-ness, which is most of them.

**Status: Decided.**

---

## D5. `Conditional` and `Piecewise` are core constructors with algebra

**Decision.** Both are core term constructors. `Piecewise` has real algebra: sums,
products, and compositions produce piecewise terms over the refined partition,
with unsatisfiable branches pruned by the assumption engine.

**Alternatives rejected.** A library-level conditional wrapper. Returning the
principal answer with a warning.

**Reversal cost.** Conditional validity becomes impossible, and D3's guards have
nowhere to land when a strategy chooses to materialise them.

**Status: Decided.**

---

## D6. Explicit binding forms with alpha-invariant representation

**Decision.** Vieta has explicit binding forms. The internal representation is
alpha-invariant. Surface syntax is named, and names are preserved as
reconstruction hints for printing and debugging.

**Deliberately open: the encoding.** De Bruijn indices, de Bruijn levels, locally
nameless, or another scoped representation, chosen after binder semantics are
enumerated, behind a traversal API that keeps it swappable.

The tradeoff is real and neither choice is obviously right. Indices make a
subterm's representation depth-dependent, so identical subterms at different
depths do not share, partially defeating hash-consing. Levels share better under a
fixed context and complicate open-term manipulation.

**Required first: enumerate what binds.** Lexical function arguments, integration
variables, summation indices, pattern variables, and sequence-pattern variables are
binders. Global symbols, domain generators, and generated indeterminates are not.
Expect the enumeration to show that more things bind than assumed.

**Alternatives rejected.** Named bound variables with capture-avoiding renaming at
substitution time. Committing to a specific encoding before the enumeration.

**Reversal cost.** The *semantic* commitment is irreversible: every traversal that
goes under a binder, plus free alpha-equivalence under hash-consing. The
*encoding* is behind an API and is not.

**Status: Decided (semantics), Open (encoding, before slice 1 ships).**

---

## D7. Hash-consing, id space, and tag bits

**Decision.** Open-addressed hash-cons table keyed on head plus arguments.
`ExprId` is a `Word32`. Reserve tag bits for small integers, small rationals, and
common symbols so arithmetic on small values never touches the table.

**Alternatives rejected.** A recursive algebraic tree with equality by traversal.
Interning added later.

**Reversal cost.** Full rewrite of the store and every consumer. Retrofitting tag
bits after the id space is in use means auditing every construction and comparison
site.

**Status: Decided (scheme), Open (tag layout, measure during the spike).**

---

## D8. `ExprId` stability across GC, and the external-handle rule

**Decision.** Store GC compacts and renumbers ids. **Nothing outside the store may
cache a raw `ExprId` across a safepoint.** Every external holder (client
connections, caches, the derivation store, the native handle table, compiled-code
constant pools) goes through a handle that survives renumbering.

**Alternatives rejected.** Non-moving GC with stable ids, which fragments and
forfeits locality. Reference counting every node, viable since the DAG is acyclic
but costly in counter traffic on a hot store.

**Reversal cost.** Every client and every cache. Also gates D23, since the wire
format must transfer ids in a renumbering-tolerant way.

**Status: Decided (invariant), Open (collector algorithm, before M1).**

---

## D9. Semantic equivalence is a context-scoped layer, never store identity

**Decision.** The primary term store provides **structural identity** and an
architecture compatible with a later e-graph. It is not itself a congruence
structure.

Congruence closure additionally requires equivalence classes over terms, union and
rebuild machinery to merge classes and restore invariants, and canonicalization of
nodes modulo those classes. The hash-cons table participates in the last of these,
since rebuilding rehashes nodes whose children's classes moved. That participation
does not make the store a congruence closure and it must not be described as one.

Any equivalence structure is a **separate, context-scoped layer over the store**.
The core store never unions terms because a temporary assumption says they are
equal. Structural identity stays stable while semantic equivalence varies by
context.

**Alternatives rejected.** Treating interning as congruence closure. Mutating
store identity to reflect contextual equalities.

**Reversal cost.** Contextual reasoning becomes impossible, and assumption-scoped
equalities leak across contexts. Also forecloses equality saturation, which
requires that an `ExprId` not be assumed to be a canonical representative that
never changes.

**Status: Decided.**

---

## D10. Layer A normalization: scope, versioning, and trace omission

**Decision.** Layer A is total, terminating, confluent, cheap, deterministic,
**versioned**, and not arbitrarily user-extensible. Scope: flatten `Flat` heads,
sort `Orderless` arguments canonically, fold exact numbers, apply identity and
annihilator laws, collect like terms. Runs at construction.

**Trace policy.** Layer A is **omitted from normal derivation traces**, because its
effect is reconstructible from the input plus the normalization version. Omitted is
not forbidden: a debugging mode, or a future certificate mode that must exhibit
every step to an external checker, may emit Layer A events. The version stamp is
what makes a replayed trace trustworthy, which is why versioning is part of the
decision rather than an implementation detail.

**Alternatives rejected.** No normalization at construction, with canonical form as
a separately computed attribute. User-extensible auto-evaluation. Unversioned
normalization, which makes replay unverifiable across releases. A blanket rule that
Layer A can never emit information, which would foreclose certificate modes.

**Reversal cost.** Determines what structural equality *means*. Every algorithm
relying on `x + 2 === 2 + x` breaks if the boundary moves.

**Required.** Write the specification, including the termination and confluence
argument, before writing the code.

**Status: Decided (scope and policy), Open (written specification, before slice 1
code).**

---

## D11. Construction is pure; simplification is never automatic

**Decision.** Constructing an expression applies Layer A and nothing else. No
expansion, factoring, trig identities, or domain-dependent rewriting. `Hold` and
`Inert` allow faithful representation of literal input including forms Layer A
would otherwise normalize.

**Alternatives rejected.** Constructor-time simplification with an
`evaluate=False` escape hatch added afterwards.

**Reversal cost.** Every test's expected output, every printer path, and the
meaning of every stored expression.

**Status: Decided.**

---

## D12. Two printers

**Decision.** A **canonical printer** satisfying `parse(canonicalPrint(t)) == t`
exactly, tested as a property from the first commit. A **pretty printer** whose
output need only satisfy `parse(prettyPrint(t))` alpha-equivalent to `t`, free to
invent binder names, drop redundant parentheses, choose infix forms, abbreviate
large expressions, and elide internal annotations.

**Alternatives rejected.** One printer serving both roles, which eventually
constrains both: lossless round-tripping forces ugliness, and readability forces
lossiness.

**Reversal cost.** Moderate rather than catastrophic, but the round-trip property
test only has value if it exists from the start; added later it codifies whatever
drift has already occurred.

**Status: Decided.**

---

## D13. Rule sets are values

**Decision.** A `RuleSet` is an ordinary Vieta value passed explicitly to
transformation operations. Named global defaults are sugar over explicit passing.
Definitions are lexically scoped rule environments.

**Alternatives rejected.** Global mutable definition tables attached to symbols.

**Reversal cost.** Determines whether memoization is sound (cache keys include
rule-set identity), whether conflicting rule sets can coexist, whether `simplify`
has a specification, and, per D20, whether Vieta code can be compiled at all.

**Note.** Upvalues are worth keeping as a *mechanism*, a head declaring how it
interacts with existing operators, while the table they live in is a value rather
than global mutable state.

**Status: Decided.**

---

## D14. Matching semantics contract

**Decision.** The matching *contract* is irreversible and written down before the
rule corpus grows. It answers, explicitly:

| Question | Why it binds |
|---|---|
| Completeness guarantee for AC matching | Incomplete matching breaks rules silently |
| All results, or one selected result | A first-match-only index cannot later serve all-matches |
| Match ordering | Determines reproducibility; specified by what, if at all |
| Deduplication | AC matching produces equivalent substitutions; who collapses them |
| Laziness | Whether a caller can take the first match without paying for the rest |
| Behaviour under resource limits | A truncated match set must be distinguishable from a complete one |

The last row is easy to skip and expensive to skip: a matcher silently returning
fewer results under pressure converts a resource limit into a wrong answer.

**Deliberately open: the index.** `RuleSet` owns a replaceable pattern index and
matcher semantics do not depend on which one is installed. The first can be
`head -> arity class -> candidate rules`; a discrimination net or family of
specialised indexes follows once pattern semantics stabilise. Associative,
commutative, flat, sequence, optional, typed, and conditional patterns all
constrain which schemes are viable, so the structure should follow the semantics.

**Alternatives rejected.** Treating a discrimination net as itself irreversible.
Leaving the completeness and ordering guarantees implicit.

**Reversal cost.** Contract: every rule silently changes meaning. Index: none, by
construction.

**Status: Decided (that a contract is fixed early), Open (its content, before the
rule count reaches the low hundreds).**

---

## D15. Domain descriptors are values; FLINT `gr` is an engine, not a constitution

**Decision.** `Domain` is data: `PolyRing (Frac ZZ) ["x","y"] Lex`, `GF 7`,
`AlgExt QQ <polyId>`. Dispatch between domains is value-level. Implementations
*within* a domain may use monomorphic typed representations. Exactly one
reification seam exists, at the lowering boundary.

FLINT's `gr` module is adopted, if the audit supports it, **behind an abstraction
boundary**:

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

**Four classifications, routinely conflated and never the same thing.**

```
Runtime type       closure, term, vector, domain element, native handle
Mathematical domain  Z, Q, GF(p), Q[x,y], a matrix algebra
Logical assumption   x is real, x > 0, n is prime
Representation       dense, sparse, factored, expanded, exact, approximate
```

`x : Real` is an assumption, not a storage type. An unevaluated user expression may
have a known runtime type, an unknown domain, several assumptions, and no
representation commitment at all. Optional annotations and compile-time checking
are compatible with this; requiring every symbolic expression to carry a determined
static domain is not.

**Alternatives rejected.** Type-indexed domains, which cannot express runtime
parameters (`GF(p)` for a computed `p`, `Q[x]/(f)` for a computed `f`) without
existentials at every boundary. Using `gr` directly as Vieta's semantic model.

**Required first.** A **FLINT `gr` capability audit**: coverage, stability, where
its taxonomy diverges from Vieta's needs, and its ternary-predicate and status
discipline. This is the first serious technical investigation and it sizes M2. It
has no dependency on slice 1 and can run in parallel.

**Reversal cost.** Library-wide rewrite if the tower later moves into the type
system or if the boundary is skipped and FLINT's model leaks into Vieta's
semantics.

**Status: Decided (value-level, boundary required), Open (`gr` adoption, pending
the audit, before M2).**

---

## D16. Lowering receives generators and relations, not a flat kernel list

**Decision.** The lowering interface receives an explicit structure containing
generators together with their known relations. Not a flat list of allegedly
independent kernels.

**Justification.** Naive kernels assume algebraic independence and the assumption
is false: `sin(x)` and `cos(x)` satisfy `s^2 + c^2 - 1 = 0`. Flat kernels produce
expressions that fail to simplify and equalities that fail to be detected.

**Deliberately not decided: a generic relation-theory interface.** A polynomial
ideal captures algebraic relations only. Vieta will also meet differential
relations, analytic continuation and branch constraints, order relations,
domain-dependent identities, functional equations, periodicity, and conditional
identities. **Implement the algebraic-ideal case and a differential-tower case
(that is Risch) before extracting a generic interface.** An interface designed
before two working instances exist will be the wrong interface.

**Reference implementation to read rather than theorise about.** Calcium
represents exact complex numbers using algebraic and transcendental extension
structures and returns `Unknown` when its heuristics cannot prove an identity.

**Reversal cost.** Every algorithm that goes through lowering.

**Status: Decided (structure, not list), Deferred (generic theory interface, until
two instances exist).**

---

## D17. Algebraic number representation

**Decision.** Defining polynomial plus isolating interval. Radical forms remain
available as a *presentation* when one exists.

**Alternatives rejected.** Nested radicals as the primary form.

**Reversal cost.** Determines whether exact sign determination, exact comparison,
real root isolation, and real algebraic geometry are possible at all.
Library-wide.

**Status: Decided.**

---

## D18. Ball arithmetic is the default inexact type

**Decision.** Arb-style real and complex balls (via FLINT 3) are the default
inexact number type. Machine `Double` is a distinct, explicitly requested type,
primarily a compiler target. No silent coercion from exact to double.

**Alternatives rejected.** IEEE double as the default with implicit coercion,
which is every mainstream CAS and is how they acquire their numerical
embarrassments.

**Reversal cost.** Changing the default changes the meaning of every numeric result
ever produced, including in saved sessions.

**Payoff.** A numeric result becomes a rigorous enclosure, and residual-based
verification becomes a certificate rather than a heuristic.

**Status: Decided.**

---

## D19. Session state is a value; the REPL is a fold

**Decision.** A session is an immutable value. Evaluating an input is a pure
function from (session, input) to (session', output). The REPL is a fold. No
ambient mutable global state.

**Alternatives rejected.** Mutable global symbol tables and context stacks.

**Reversal cost.** Every stateful component. Forecloses reproducible sessions,
forkable exploration, replay, time-travel debugging, deterministic tests,
distributed evaluation, and, per D20, cheap world snapshots.

**Note.** The expression store is *not* part of the session value; it is a
content-addressed pool growing monotonically between safepoints, and the session
holds handles into it (D8). This keeps the session value cheap to copy.

**Status: Decided.**

---

## D20. Compiled Vieta code runs against immutable versioned worlds

**Decision.**

> Every dependency of compiled Vieta code is explicit, captured, or versioned.
> Compiled code executes against an immutable world snapshot. Redefinition creates
> a new world and either invalidates and recompiles dependent code or leaves
> existing compiled code attached to its original world. Explicit dynamic lookup is
> permitted; it is an optimization barrier and it is visible in the program
> semantics.

**What is forbidden is invisible or untrackable late binding**, meaning a compiled
function whose behaviour silently changes because something it never named was
mutated elsewhere. Late binding the program declares is a legitimate feature with a
known cost.

**Alternatives rejected.** A blanket prohibition on features requiring late
binding. Too restrictive for an interactive symbolic language: it would foreclose
dynamic definitions, runtime package loading, reflection, dynamically supplied rule
sets, and evaluation in a modified session, all of which Vieta should have.

**Justification.** If most of the library is Vieta source, Vieta's execution speed is
the system's speed for everything that is not a native call. Maxima and REDUCE are
self-hosted *and* natively compiled because their host Lisp compiles the library
for free; a Rust-hosted Vieta does not get that. The system whose library language
is interpreted is SymPy, and library-level slowness is its most persistent
complaint.

**Scope, which is narrower than "compile everything".**

| Executes through | Work |
|---|---|
| The native matcher, unchanged | Vieta-level declarative rule sets |
| A compilation path, required | Control flow, recursive strategies, classifiers, solver orchestration, series drivers |

**First implementation target is a bytecode VM**, not native ahead-of-time
compilation. A JIT is a later option, not a prerequisite. The compilation path must
exist **before a substantial self-hosted mathematical library is built**, because
what is affordable to write shapes what gets written.

**Enabled by.** D13 and D19 make worlds cheap: a world is a session value plus the
rule-set values reachable from it, so snapshotting is already the natural operation
rather than a mechanism added for the compiler.

**Reversal cost.** Retrofitting world discipline onto a language with ambient
mutable definitions means changing the semantics of existing programs.

**Status: Decided (world model), Open (invalidation versus world-pinning policy,
before M4).**

---

## D21. Laziness is first-class, explicit, and selective

**Decision.** Vieta's own language provides lazy or coinductive structures as a
first-class feature. Evaluation is **strict by default** (D25); laziness is
requested, never ambient. Its uses are formal power series, potentially infinite
sequences, delayed transformation search, and candidate solution streams.

**Quotation is a separate mechanism.** `quote` produces a term, unevaluated because
it is data. A delay construct produces a computation, deferred because it may not
be needed. They must not share a keyword. Mathematica's `Hold` carries both jobs,
and that overloading is part of why its evaluation semantics resist statement.

**Recommendation on the open mechanism**, recorded so the reasoning survives even
if the choice changes: prefer value-level thunks and streams to an evaluation-mode
annotation on bindings. A value keeps the delay local and visible to its caller; an
annotation contaminates the strictness reasoning of everything downstream.

**Justification.** Choosing a strict systems host relocates the formal-power-series
requirement into Vieta rather than removing it. Series machinery is load-bearing for
limits, asymptotics, integration, and special functions, and candidate streams in
transformation search want the same machinery. Designed in, this recovers the best
thing a lazy functional host would have provided. Left out, the capability is lost
rather than moved.

**Alternatives rejected.** Native-only series with a strict Vieta surface, which
puts the most mathematically interesting manipulations permanently out of reach of
the self-hosted library and undercuts D1's premise.

**Reversal cost.** Adding laziness to a strict language later means either a
parallel evaluation mode or pervasive explicit thunking in library code already
written.

**Status: Decided (that Vieta has it), Open (mechanism: lazy by annotation,
explicit streams, or coinductive types, before M5).**

---

## D22. Interrupt, fuel, and progress live in the evaluation context

**Decision.** Every evaluation carries a context with a cancellation token, a fuel
counter, and a progress channel. The rewrite loop checks them. Every long native
call either respects a cooperative cancellation protocol, is split into cancellable
stages, or runs in a worker process that can be terminated.

**Explicitly not a language argument.** Terminating arbitrary C mid-mutation is
unsafe in any host. This is architecture, and the same three options apply
regardless of D1.

**Alternatives rejected.** Top-level signal handling only. Timeouts wrapping whole
computations.

**Reversal cost.** Cannot be threaded in afterwards; it touches every recursive
evaluation path and every native binding.

**Status: Decided (context carries them), Open (in-process cooperative hooks versus
worker-process isolation, before M2).**

---

## D23. One binary store-segment format, three uses

**Decision.** A single binary format serves the wire protocol, session persistence,
and the on-disk cache. Contents: topologically ordered node table, symbol table,
number blob, shared-node back-references, optional side tables for obligations and
derivations. Version negotiation from v1. No JSON in the architecture.

**Alternatives rejected.** Separate formats per use. JSON as anything beyond a
disposable throwaway test.

**Reversal cost.** Every client, every saved session, every cache entry.

**Required.** Define it in slice 1 even though the first REPL sends text. The
transport is multiplexed so abort and progress travel out of band alongside
results.

**Status: Decided (one format), Open (encoding details).**

---

## D24. Derivation is sink-based

**Decision.** The rewrite loop emits derivation events to a sink. When provenance
is off the sink is a no-op and nothing is allocated. Never
materialise-then-discard.

**Alternatives rejected.** Tracing bolted on afterwards, which never exceeds
partial coverage because every path written without it must be revisited and the
missed ones are invisible. Constructing full derivation objects and dropping them,
which pays allocation on every microscopic step.

**Interaction with D10.** Layer A is omitted from normal traces, which removes most
of the volume before the sink design has to carry it.

**Cost when off.** Near zero: a rewrite step already knows the rule, the matched
subterm, the substitution, and the discharged obligations. Provenance means routing
that to a sink rather than dropping it on the floor.

**Status: Decided.**

---

## D25. Vieta is a strict, expression-oriented functional-rewrite language

**Decision.** The constitutional statement, which logically precedes every other
entry in this register:

> Vieta is a strict, expression-oriented functional-rewrite language. Ordinary
> computation uses lexically scoped functional programming. Symbolic computation
> uses first-class guarded rewrite rules controlled by explicit strategies. Syntax,
> terms, and runtime values are distinct but interoperable. Mathematical domains,
> assumptions, rule sets, and sessions are values. Effects and dynamic lookup are
> explicit. Vieta code compiles against immutable versioned worlds.

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

**Strictness, specifically.** Eager call-by-value for ordinary function arguments.
Immutable bindings, lexical scope, closures, algebraic data types, structural
pattern matching, persistent collections, higher-order functions.

**Alternatives rejected.** A conventional functional language with a rewriting
library, which ends up simulating rewriting through functions. A Mathematica-style
system with functions added, which ends up simulating control flow through rule
ordering and package load order. Global non-strictness, rejected because memory
behaviour becomes hard to reason about over a multi-million-node store, native call
ordering stops being locally visible when those calls mutate FLINT objects,
compilation gets harder exactly where D20 says it must not, and a symbolic language
already carries enough non-obvious evaluation order.

**Reversal cost.** Total, in the same sense as D1. Strictness in particular cannot
be reversed: making a strict language lazy changes the meaning of every program
already written.

**Deliberately open.** Concrete surface syntax and all internal encodings, until the
semantic contracts in §2 of `architecture.md` are written down.

**Status: Decided.**

---

## D26. Syntax, Term, and Value are distinct representations

**Decision.**

```
Syntax    scoped program structure; what the parser produces and macros consume
Term      symbolic or mathematical expression; the interned DAG
Value     evaluated runtime object: closure, compiled function, dense matrix,
          domain element, native polynomial handle
```

> Everything is representable as a term. Not everything is stored as one.

Specialized values reify into terms on demand rather than living as terms.

**Required: the reification contract, in writing.** Is reification total, or may a
value refuse? Is it injective, so that reifying and re-evaluating yields an
equivalent value? Does `reify(v1) == reify(v2)` imply anything about `v1` and `v2`?
What does structural equality mean between a value and a term? Same class of
question as D2, and it deserves the same treatment: answered before values exist,
because afterwards every answer is a migration.

**Second consequence, which is why the trichotomy is load-bearing.** Racket-style
hygiene needs syntax objects carrying lexical scope information that accumulates
during expansion. Interning those in the term store would force a choice between
scope-decorated nodes that defeat sharing and mutation of interned structure.
Keeping Syntax separate avoids the choice and is what leaves D29 available.

**Alternatives rejected.** One universal representation for everything, which is
SymPy's `Basic` uniformity and its cost: a dense FLINT matrix pushed through a
generic expression tree is slow for no gain. A parser producing terms directly,
which is the shortcut that forecloses hygienic macros.

**Reversal cost.** Introducing the distinction after macros exist is a rewrite of
the front end. Introducing it after values exist is a migration of every value kind.

**Status: Decided (the trichotomy), Open (the reification contract, before slice 1
ships).**

---

## D27. Five named operations, never merged

**Decision.**

| Operation | What the user asks for | Mechanism |
|---|---|---|
| `eval` | execute a Vieta program | Layer B |
| `normalize` | deterministic structural canonicalization | Layer A |
| `simplify` | a preferable equivalent form under a context and cost function | Layer C |
| `prove` | establish a proposition | assumption engine |
| `approximate` | cross deliberately into numeric representation | exact/inexact boundary |

Layers describe where code lives; operations describe what a user asks for.
`prove` and `approximate` correspond to no layer, which is why one vocabulary
cannot carry both. The mapping is maintained explicitly or the two drift.

**`prove` returns a `Truth`, never a `Bool`**, paired with a `Status` (D4). It must
never answer `False` when it means `Unknown`. That is the most abusable signature in
the system, because a user reading `False` will act on the negation.

**Ordinary evaluation never launches `simplify`.** Constructing `(x^2 - 1)/(x - 1)`
yields a normalized term that still has a singularity at `x = 1` (D11).

**Alternatives rejected.** One `Simplify` doing all five jobs, which is Mathematica
and is unspecifiable by construction. Implicit numeric coercion instead of an
explicit `approximate`, which is how every mainstream CAS acquires its numerical
embarrassments (D18).

**Reversal cost.** Merging later is easy and splitting later is what Mathematica
could not do, because by then every user program depends on the merged behaviour.

**Status: Decided.**

---

## D28. Functions, rules, and strategies are three first-class kinds

**Decision.**

```
Function    maps argument values to a result value
Rule        relates term patterns; returns a guarded set (D3)
Strategy    decides where, when, and in what order rules are tried
```

All three are first-class immutable values. D13 covers rule sets; this extends the
same commitment to strategies.

**Strategies as values is the part usually missed.** With traversal and control
hard-coded into the engine, rule ordering, package load order, and registration
order become the de facto strategy: accidental, unspecifiable, and liable to differ
between sessions. Take Stratego's and Maude's vocabulary as ordinary combinators:
`topdown`, `bottomup`, `innermost`, `try`, `repeat`, `choice`, composition.

**A rule carries no traversal policy.** *Rules report; strategies decide* now has
two readings and both hold: a rule decides neither what becomes of an undischarged
guard (D3) nor where it is applied.

**Dependency.** The claim that `simplify` means extraction of a minimal-cost
representative under a declared cost function from a declared rule set is true only
if strategies are values. Otherwise it is aspiration.

**Alternatives rejected.** Traversal as a fixed engine behaviour with a mode flag.
Strategy encoded implicitly in rule order.

**Reversal cost.** Every rule application site, plus the meaning of every existing
transformation, since the implicit strategy would have to be reconstructed and made
explicit rule by rule.

**Status: Decided (three kinds, strategies are values), Open (combinator
vocabulary, before the rule corpus grows past a few dozen).**

---

## D29. Macros are hygienic, phased, and separate from symbolic rewriting

**Decision.** Macros transform Syntax before evaluation. Rules transform Terms
during it. Different input representation, different phase, different requirements.
Take Racket's model: syntax objects carrying lexical context, explicit expansion
phases, module-scoped binding, and a deliberate escape hatch for intentional
capture.

**Hygiene cannot be retrofitted.** Every Lisp that began with unhygienic
substitution and later attempted hygiene found the existing macro corpus already
depended on capture, so the fix broke working code.

**The temptation specific to Vieta.** The system already has pattern matching over
expressions, so `macro` looks like a special case of `rule`. It is not. A rule
matching `x_` matches a term. A macro mentioning `x` has to know which binding `x`
refers to at the use site, and a term carries no lexical context to consult.

**Alternatives rejected.** Macros as symbol substitution over terms. Unifying the
macro system with the rewrite system.

**Reversal cost.** Total for the macro corpus, and it is not the kind of cost that
announces itself: unhygienic macros work until the day a user names a local
variable the same as something a macro expands to.

**Status: Decided (hygiene and phase separation required), Deferred
(implementation; not slice 1, and D26 is what keeps it available).**

---

## D30. Effects are explicit and visible to the compiler

**Decision.** Tracked categories: `IO`, `Random`, `Time`, `MutableState`,
`DynamicLookup`, `Native`, `Unsafe`. Surface syntax need not be monadic; effect
blocks or a `do` form are acceptable. What is required is that the compiler knows
where effects occur.

**Why it is not optional.** D20's world discipline is enforceable only if
`DynamicLookup` is something the compiler can see. An effect the compiler cannot see
is exactly the invisible late binding D20 forbids.

**Interaction with D19.** Session-as-a-value already makes top-level evaluation
pure, so effects must be threaded through the session or handled at its boundary.
That is a bill D19 was always going to present, and paying it is part of what
reproducible sessions cost.

**Alternatives rejected.** Untracked effects with a documentation convention, which
means the compiler cannot distinguish a pure function from one that reads the clock,
and neither can a cache.

**Reversal cost.** Adding effect tracking to a language with untracked effects means
auditing every function ever written, and the audit is not mechanically checkable.

**Status: Decided (explicit and tracked), Open (checked versus declared, before
M4).**

The open fork is real. *Checked* means effects are inferred and verified and a pure
function calling an effectful one is an error; stronger, and considerably harder on
a dynamically symbolic surface language. *Declared* means annotated and trusted,
useful for optimization and reproducibility but guaranteeing nothing.

---

## D31. Multiple dispatch is constrained by ownership

**Decision.** Dispatch is value-level, over runtime types and domain descriptors
(D15). A module may define a method when it owns the operation, owns at least one of
the domains or types involved, or imports an explicit extension namespace.

**Why the constraint.** Unconstrained global extension makes behaviour depend on
which packages happen to be loaded, which is precisely the property D13 and D20
exist to eliminate. Julia names the violation type piracy and its ecosystem
documents the consequences.

**Preference for symbolic behaviour.** Explicit rule sets over global extension:
`simplify(expr, using = QuantumMechanics.rules)` rather than an invisible upvalue
installed on a shared head.

**Alternatives rejected.** Open global method tables. Symbolic extension exclusively
through upvalues attached to symbols, which D13 already rejects as global mutable
state.

**Reversal cost.** Once a package ecosystem exists, imposing an ownership rule
breaks published packages. The rule has to exist before the ecosystem, not after.

**Status: Decided (ownership rule), Open (where the method/rule boundary falls,
before M4).**

The working answer for the open part, to be tested against the library before the
library is written: a method resolves an operation on *values* whose domains are
known; a rule transforms *terms*, including terms whose domain is unknown.

---

## D32. Internal mutation is permitted and must be unobservable

**Decision.** Symbolic terms, assumptions, rule sets, domains, sessions, and
ordinary language values appear immutable in Vieta semantics. Native Rust and FLINT
implementations may mutate buffers and objects internally, and are expected to,
because FLINT's in-place idiom is where its performance comes from (§7.5 of
`architecture.md`). The requirement is that no such mutation is observable.

Mutation inside Vieta is available only through explicitly marked regions, which is
`MutableState` under D30.

**Alternatives rejected.** Pure-all-the-way-down, which forfeits the in-place native
idiom and is why per-object FFI wrappers look attractive and then do not scale.
Observable mutation of shared values, which breaks structural sharing, caching, and
D19 simultaneously.

**Reversal cost.** Observable mutation, once it leaks into semantics, cannot be
withdrawn: some program depends on it. This is a principle rather than a mechanism,
recorded because principles are what get eroded quietly.

**Status: Decided.**

---

## Note: the executable specification

Not a numbered decision because it is not irreversible, but recorded because the
alternative is tempting and wrong.

A second reference implementation in another host language is rejected: one that
nobody runs rots within a year and is then worse than nothing, because it still
looks authoritative. Instead, once the language can carry it, write the executable
specification as a **metacircular reference evaluator in Vieta itself**. It runs on
the real kernel so it cannot rot, it is the most demanding available test of the
surface language, and it is the strongest evidence that Vieta expresses its own
semantics.

The **conformance suite** is separate, comes earlier (M1), and is
implementation-independent. For a project measured in decades it is the artifact
that actually survives.

---

## Open items

| Item | Needed before | Reference |
|---|---|---|
| Binder encoding (indices, levels, locally nameless) | Slice 1 ships | D6 |
| Enumeration of what binds | Before choosing the encoding | D6 |
| Tag-bit layout in the id space | Slice 1 | D7, measure during the spike |
| Layer A written specification | Slice 1 code | D10 |
| Matching semantics contract content | Rule count in the low hundreds | D14 |
| Canonical ordering for `Orderless` arguments | Slice 1 | D10 |
| Reification contract (totality, injectivity, value/term equality) | Slice 1 ships | D26 |
| Surface syntax | Slice 1 | Explicit `*` in core; implicit multiplication confined to a marked math-input mode |
| Strategy combinator vocabulary | Rule corpus past a few dozen | D28 |
| Store GC algorithm | M1 | D8, epoch or region-based with compaction is the working assumption |
| Conformance suite format | M1 | Implementation-independent |
| FLINT `gr` capability audit | M2 | D15, no dependency on slice 1, can start now |
| Native cancellation strategy | M2 | D22 |
| World invalidation versus pinning policy | M4 | D20 |
| Effects checked versus declared | M4 | D30 |
| Method versus rule boundary | M4 | D31 |
| Laziness mechanism in Vieta | M5 | D21 |
| Concurrent hash-cons strategy | When anything is parallel | Sharded tables or lock-free insert |
| License | Before any publication | Permissive maximizes survivability for a solo project |
