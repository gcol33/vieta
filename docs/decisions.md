# Vieta: Irreversible Decision Register

Revised 2026-07-28. Renumbered once, to follow the order of `architecture.md`;
references to earlier numbering do not carry over.

**Numbering is append-only.** D25 through D32 record the language constitution and
logically precede D1. They are numbered last so that existing citations stay
valid, which is worth more than a tidy reading order. Read D25 first.

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
the matcher, and polynomial arithmetic are naturally native. **Standing metric**:
the test suite reports the Vieta-source fraction of the library and the
Vieta-level fraction of runtime, from the first Vieta source file onward. If the
first is low and flat, the premise failed. A single audit at one milestone reports
that after the code that failed it is written; a standing number reports it while
the ratio is still moving.

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

## D6. Binding has shared semantics and separate representations

**Decision.** Vieta has explicit binding forms, and binding appears in two
representations that share one theory of scope. An executable binder resolves
during elaboration and compiles into local references, code, and a runtime closure.
A symbolic binder is an immutable alpha-invariant term in the store. A closure is a
`Value`; a symbolic binder is a `Term` (D26, D34). Surface syntax is named, and
names are preserved as reconstruction hints for printing and debugging.

> A closure executes binding. A symbolic binder represents binding.

**Deliberately open: the encoding.** De Bruijn indices, de Bruijn levels, locally
nameless, or another scoped representation, chosen after binder semantics are
enumerated, behind a traversal API that keeps it swappable. There are two encodings
to choose, one on each side, and they are not required to agree.

The tradeoff is real and neither choice is obviously right. Indices make a
subterm's representation depth-dependent, so identical subterms at different
depths do not share, partially defeating hash-consing. Levels share better under a
fixed context and complicate open-term manipulation.

**Required first: enumerate what binds.** Lexical function arguments, integration
variables, summation indices, pattern variables, and sequence-pattern variables are
binders. Global symbols, domain generators, and generated indeterminates are not.
Expect the enumeration to show that more things bind than assumed.

**Three cases the list above does not reach**, each of which changes what the
traversal API goes under (§13.1):

- *Quantified variables inside a proposition.* A proposition is a term, and
  `forall n. n >= 1 => f(n) > 0` binds `n` inside it. §2.5's tiers take free
  variables constrained by the context, so accepting a quantified assumption at all
  is a decision this enumeration forces.
- *Comprehension and index-set variables.* `{x in S : P(x)}`, and the index of a
  sum whose index set is a term. The dummy-variable case reached through a surface
  form where the binder sits in argument position.
- *Constants generated by a solution set.* Either a fresh global symbol or a
  variable bound by the set. Under the second, `{x = 2*pi*k : k in Z}` written
  twice with different constant names is alpha-equivalent and shares one `ExprId`;
  under the first the two terms never merge.

**Three levels, each doing separate work.** `fn(x) => e` binds a lexical variable
and becomes a closure (D34); a symbolic `Function(x, x + y)` under an integral is a
term whose binder is in the store. Substitution is environment lookup in the first
and capture-avoiding replacement in the second. Both exist, because they represent
different things.

```
fn(x) => x + y            source    Lambda("x", Add(Name "x", Name "y"))
                          resolved  Lambda(Binder 17, Add(Local 17, Global "y"))
term { fn(x) => x + y }   stored    Lambda(Add(Bound 0, FreeSymbol "y"))
```

The source level keeps names and origin (D37). The resolved level is what
elaboration produces, and it is what determines scope, shadowing, free variables,
closure capture, and compilation. The stored level is alpha-invariant term data, so
`term { fn(x) => x + y }` and `term { fn(z) => z + y }` are one `ExprId`. That
sharing is a property the store owns; closure identity is a separate question, and
two closures are not equal for looking alike.

**The context decides which path a written form takes.**

```
let f = fn(x) => x + y            a closure, callable
let t = term { fn(x) => x + y }   a term, inspectable and substitutable into
```

The first carries captured runtime values, a world (D20), and identity. The second
can be substituted into, differentiated around, rewritten, or evaluated later under
an environment. Quotation is the crossing between the two layers and is the only
one.

**What this settles.**

- A bound occurrence and a free symbol are distinguishable after elaboration. A
  lexical `x` and a symbolic `x` are two things.
- Binding is alpha-invariant once elaboration has run, on both sides.
- The store carries explicit symbolic binding forms.
- An executable lambda compiles into code and a closure. It does not become an
  interned binder term.
- Quotation deliberately lowers a binding form into the symbolic store.
- Both layers obey one capture-avoidance and scope contract, behind a shared
  traversal and substitution API.
- The concrete encoding stays deferred on both sides.

**A closure does not reify by reading its source back.** Given `let y = 3` and
`let f = fn(x) => x + y`, `describe(f)` can report the function together with the
capture `{y: 3}`. `reify(f)` returning `term { fn(x) => x + 3 }` is a stronger claim
and is not available in general, since a capture may hold a native domain value, a
mutable resource, an iterator, another closure, or a world-dependent reference.
D26's split between the two operations is what makes the difference statable.

**Where the encoding gets decided.** D38 gives it a setting. The first elaboration
slice carries literals, names, calls, `let`, `lambda`, and quotation, which is the
smallest arrangement in which a lexical binder has to be represented for real. Its
binding shape follows from the above: a named binder becomes a `BinderId`, each
occurrence resolves to a local reference or a global symbol, free-variable analysis
over the resolved form gives the capture set, and quotation elaborates into a
symbolic binding context that builds a store term. Local slots and closure layout
are compilation and wait for D38's evidence. The enumeration above stays the gate on
the store's traversal API, and each encoding gets chosen against code that runs
rather than against the list.

**Alternatives rejected.** Named bound variables with capture-avoiding renaming at
substitution time. Committing to a specific encoding before the enumeration. One
representation serving both layers, which either interns a closure's captured
environment in the store (D19, D34) or gives up alpha-invariance for symbolic
binders. Reifying a closure by recovering its source lambda, which is defined only
when every capture has a symbolic reading.

**Reversal cost.** The *semantic* commitments are irreversible: every traversal that
goes under a binder, free alpha-equivalence under hash-consing, and the split
between the two layers, which decides what the machine holds and what the store
holds. The *encodings* are behind an API and are not.

**Status: Decided (semantics, and that executable and symbolic binders are separate
representations under one scope contract), Open (both encodings and the shared
traversal and substitution API, with the first elaboration slice, D38).**

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
argument, before writing the code. It is `docs/layer-a.md`, version 1: the
canonical signature vocabulary, the canonical order, the pass sequence with the
preconditions each pass establishes, the termination argument, and the canonicity
theorem with the point where completeness deliberately stops.

**The criterion the specification turns on.** Layer A returns an `ExprId` and has
nowhere to put a guard, a `Status`, or a second branch, so every law it applies
holds with no side condition. `x^1 -> x` is in, `x^0 -> 1` is out because of
`0^0`, and cancellation is out entirely, which is what keeps the condition on
`(a^2-1)/(a-1)` from being deleted at construction.

**One consequence worth having in the register.** The canonical order for
commutative arguments is determined by content and never by id. Sorting on the id
word breaks under GC renumbering (D8), disagrees between processes on the wire
(D23), and makes canonical printed output depend on the order symbols were
interned in, which forecloses an implementation-independent conformance suite.

**Status: Decided (scope, policy, and the written specification). Open: whether
construction gets a resource limit, which `docs/layer-a.md` §14 states as a
decision rather than an oversight.**

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
what is affordable to write shapes what gets written. Read literally that means
slice 1, which is D33.

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

**Observation is a fourth channel, and it is inert.** D33 leaves the machine as
the only thing that can report what a program is doing, so the dispatch check that
serves these three also carries the debugger and profiler hook. Sharing one runtime
poll does not merge them: fuel is a budget, cancellation is a request, progress is
a report, and observation must not become any of the three. Enabling it changes no
evaluation result, no world visibility, no rule ordering, no cancellation
behaviour, and no resource accounting beyond documented observer overhead. An
expression evaluated at a breakpoint runs against a derived debug world or a
read-only frame context, never against the captured world under inspection. §13.6.

**Alternatives rejected.** Top-level signal handling only. Timeouts wrapping whole
computations.

**Reversal cost.** Cannot be threaded in afterwards; it touches every recursive
evaluation path and every native binding.

**Status: Decided (context carries them; observation is a fourth channel and is
inert), Open (in-process cooperative hooks versus worker-process isolation, before
M2).**

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

Syntax itself divides once more, into a lossless concrete tree and the
semantically shaped form elaboration reads (D37), for the same reason this entry
gives one level up.

> Everything is representable as a term. Not everything is stored as one.

Specialized values reify into terms on demand rather than living as terms.

The separation also carries the two readings of a written expression. Syntax
preserves what was written, so `quote { y + x }` keeps that order. A term is
canonical under its operators' shape laws (D36), so elaborating the same source
yields `x + y`. Both equalities exist and neither is a compromise of the other.

**What each representation is mathematically.** Syntax is a tree in the free term
algebra, with nothing identified. A Term is an element of that algebra modulo the
equational theory its operators carry (D36), which is where structural equality
acquires meaning. Interpretation in a particular mathematical model is a third
thing again, and it is the domain tower (D15, D16): the same term is read in
`Q(a)` or in a finite field by choosing a domain, not by rebuilding the term.
Value is orthogonal to all three and belongs to the machine (D34).

Reading the three in order gives the reason the trichotomy is not an
implementation convenience: free algebra, then quotient by a theory, then
interpretation in a model. Collapsing any adjacent pair loses a distinction the
mathematics makes.

**Required: the reification contract, in writing.** Is reification total, or may a
value refuse? Is it injective, so that reifying and re-evaluating yields an
equivalent value? Does `reify(v1) == reify(v2)` imply anything about `v1` and `v2`?
What does structural equality mean between a value and a term? Same class of
question as D2, and it deserves the same treatment: answered before values exist,
because afterwards every answer is a migration.

**Inspection is not reification.** A debugger displays runtime values (§13.6), and
a closure, an iterator, a native FLINT polynomial handle, a compiled function, and
an `ExprId` do not share one printer. Pushing each into a Term so that a display
exists is the collapse this entry prevents. The contract therefore covers two
operations: `describe`, total structural inspection defined on every value kind,
and `reify`, the mathematically meaningful conversion a value may refuse. Neither
is one of D27's five, since both inspect the machine rather than answering a
mathematical question.

The closure is the case that shows the split is doing work (D6). A closure carrying
captured values is not its source lambda: `describe` can report the function and the
capture set, while `reify` returning that lambda with captures substituted in is
defined only when every capture has a symbolic reading. Captures holding native
domain values, mutable resources, iterators, or world-dependent references have
none, and the contract has to say so rather than produce a term that reads back
wrong.

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

**Status: Decided (the trichotomy), Open (the reification contract, including the
split between `describe` and `reify`, before slice 1 ships).**

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

**Open: whether a directed request for a form is a sixth operation.** `simplify`
searches an equivalence class under a metric and always returns something. Asking
for the factored form of `(x + 1)^5` supplies a target instead of a metric, and can
be `Unable`. Reaching a named form by tuning a cost function until it wins states
something direct indirectly, and the table has no other slot for it, so `simplify`
will acquire an option per target form unless a `convert(expr, form)` exists to take
them. The cost function itself is unspecified in the same place: nothing yet says
what it ranges over, whether extraction costs, conversion targets, and compilation
objectives (§11.1, M7) share one vocabulary, or that it is a value. It has to be a
value by this register's own argument, since a global mutable notion of simplest is
D13's trap with silent answer changes instead of visible ones. §13.2.

**Alternatives rejected.** One `Simplify` doing all five jobs, which is Mathematica
and is unspecifiable by construction. Implicit numeric coercion instead of an
explicit `approximate`, which is how every mainstream CAS acquires its numerical
embarrassments (D18).

**Reversal cost.** Merging later is easy and splitting later is what Mathematica
could not do, because by then every user program depends on the merged behaviour.

**Status: Decided (the five operations and their separation), Open (whether a
directed form request is a sixth, and the objective vocabulary).**

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

## D33. Layer B is compiled from slice 1; there is no tree-walking evaluator

**Decision.** Vieta's evaluation layer is `Syntax -> compile -> bytecode -> machine`
from the first working evaluator. No tree-walking interpreter is written: not as a
prototype, not as a reference implementation, and not as a debugging path kept
alongside the machine.

**Justification.** Three arguments, and the third is the one that is hard to see
from inside M0.

D20 already requires the compilation path to exist before a substantial
self-hosted library is built, because what is affordable to write shapes what gets
written. Slice 1 writes library code, so "before" resolves to slice 1 rather than
to M4.

A tree-walking evaluator is the largest discardable artifact in the plan. Every
line written against it before the machine arrives inherits its performance shape,
and migrating them is the precise cost D20 exists to avoid paying.

D20's world model is a compilation decision, and nothing exercises it until
compiled code runs. Scheduling the machine at M4 defers the first honest test of
world discipline until three milestones of semantics have been written on the
assumption that it works.

**Cost, stated plainly.** The machine has to exist before the language is
demonstrable, so the first runnable Vieta arrives later than it would behind an
interpreter. That is the trade: a later first result, against no rewrite and an
earlier test of D20.

**Scope.** Bytecode, not native ahead-of-time compilation; a JIT stays a later
option (D20). Declarative rule sets continue to execute through the native matcher
and are not compiled, per D20's scope table.

**Alternatives rejected.** A tree-walking evaluator at M0 with the machine at M4,
which is the sequencing this entry replaces. An interpreter retained beside the
machine as a reference or debugging path, which is the primary-plus-fallback shape
that guarantees two implementations diverge and makes every semantic question ask
which one is authoritative.

**Reversal cost.** Asymmetric, which is why it is registered rather than left as
sequencing. Choosing the machine first and adding an interpreter later costs
nothing anyone wants. Choosing the interpreter first and adding the machine later
means rewriting the evaluator and re-tuning everything written against the
interpreter, and the second cost is invisible while it accrues.

**What removing the interpreter transfers to the machine.** With no second
evaluator, the machine is the only thing that can report what a program is doing,
so a debugger and a profiler have no other host, and observability is part of the
machine's contract instead of something tooling reconstructs from outside. Four
requirements land on the instruction set and the calling convention, none of them
on a tool built later. §13.6.

- An optional artifact side table from instruction range to origin, kept out of the
  instruction encoding so stripped artifacts stay valid. Origin is a chain through
  macro expansion and elaboration, not only D37's source span.
- A stable logical frame model, since tail calls, inlining, native matcher calls,
  and a later JIT each detach the physical stack from the program as written.
- An observation point on the dispatch poll that changes no evaluation result, no
  world visibility, no rule ordering, no cancellation behaviour, and no resource
  accounting beyond documented observer overhead (D22).
- D26's split between inspecting a value and reifying it, since a debugger
  displaying a closure or a native handle must not force it through `Term`.

**Status: Decided (no interpreter, bytecode in slice 1; observability is the
machine's contract), Open (instruction set and calling convention, derived from the
first elaboration slice per D38, with the origin map, the logical frame model, and
the observation boundary settled alongside it).**

---

## D34. A term is one kind of runtime value

**Decision.** The bytecode machine computes with `Value`s. A `Term` is one kind of
`Value`, alongside integers, booleans, closures, compiled functions, domain
elements, and native objects. A term-valued slot carries a reference into the
store; the other kinds do not.

**Derived rather than chosen**, which is the strongest form this entry could take.
D26 already says Value and Term are distinct representations and that specialized
values reify into terms on demand. D19 already says the store is a
content-addressed pool growing monotonically between safepoints. Together those
settle it: making every closure allocation an intern-table insertion would hash a
captured environment on every call and place ephemeral mutable objects in a pool
whose whole design is immutability and sharing.

**What §0.6 does and does not claim.** Symbolic terms are first-class runtime
values. That is a statement about terms being *among* the values, and D26 is the
reason it is not a statement that values *are* terms.

**Two storage situations, one integer.** `let i = 2` holds a runtime integer; the
`2` in `quote { x + 2 }` is a symbolic occurrence inside a term. These are not two
mathematical integers, and the conversions between them are invisible in the
language: `term(2)` yields the symbolic occurrence and evaluating it yields the
runtime integer. Whether either is a tagged immediate, and where the boundary to a
side table falls, is encoding.

**Alternatives rejected.** Everything is a term, which is Wolfram's uniformity;
§2.8 of `architecture.md` rejects it for values on cost grounds and D19's
monotonic pool rejects it for closures.

**Reversal cost.** The semantic half is cheap to state and expensive to retrofit:
a machine built on the assumption that every value is a term puts closures and
native handles in the store, and taking them back out afterwards touches every
value kind. The representation half carries no such cost while it stays internal,
which is the point of keeping it there.

**What it explicitly does not decide, and what is not to be frozen before
measurement.** Whether the machine word is 64 bits, the tagging discipline, how
small integers are encoded, whether closures are pointers or table indices, and how
values appear in the bytecode format. These are engineering choices under
measurement, and none of them may leak: opcodes are typed by value kind, so no bit
pattern appears in the bytecode format, in the language semantics, or in any
signature a Vieta program can observe. Kept internal, the representation stays
revisable after the VM ships.

**Status: Decided (a term is one kind of value; the representation stays internal),
Open (every representation choice, deferred until the machine can be profiled).**

---

## D35. Term construction and destructuring compile

**Decision.** Building a term and matching against a term pattern are **compiled**
operations inside a Vieta function body. A compiled match tree is a function of the
head's canonical signature, which D36 makes immutable, and of the world's matching
policy, which is versioned and therefore a captured dependency.

**Justification.** §0.6's `differentiate` is the characteristic function of the
self-hosted library, and its body is control flow: recursion, dispatch on
structure, a higher-order map, a guard. D20's scope table puts control flow in the
"compilation path, required" column. Reaching the destructuring through an
interpretive call into the native matcher with a runtime pattern term would leave
the recursion compiled and the taking-apart interpreted, which is where a symbolic
library spends its time. Half-compiling the characteristic function forfeits most
of what D33 was for.

**Why the laws have to stop being ambient.** Matching `Add(terms...)` is matching
modulo the laws declared on `Plus`. A compiled match tree is therefore a function of
those laws, which makes them a dependency of compiled code, which under D20 makes
them explicit, captured, or versioned. D36 splits them and the two halves land in
different places: the canonical signature belongs to the operator identity and
cannot move, so the compiler reads it once and carries no dependency edge for it,
while matching policy is world state and is captured and invalidated like any other
world dependency. Mathematica keeps all of it in ambient mutable global state,
which is a sufficient reason why matching there cannot be compiled.

**Boundary with D20's scope table, which is unchanged.** A declarative rule set
executes through the native matcher. A `match` in a function body compiles. The
two mechanisms differ in whether the pattern is known when the code is compiled,
and D28's function/rule/strategy split is what makes the question answerable.

**What "compiles" claims, stated narrowly so the entry does not overreach.** The
discrimination between arms, the binding of pattern variables, the guard, and the
arm bodies become bytecode. Matching modulo `Orderless` and `Flat` involves a
search over argument groupings that no decision tree removes, so the compiled code
calls native search routines at those points, with the pattern shape and the
canonical signature fixed at compile time rather than re-read per call. D1's
placement of the matcher in the permanent host layer stands; what moves is the
dispatch and the per-call re-interpretation of a pattern that was already known.

**Consequence for D10.** The Layer A specification has to say what a compiler reads
to obtain the laws, because this entry makes them a compile-time input rather than
a runtime lookup. D36 answers it: the operator entry reached through the head id.

**Prior art to read rather than derive.** Maude compiles matching modulo
associativity, commutativity, and identity. That is the hardest single piece of
Vieta's kernel and it has a literature; §4 of `architecture.md` covers what to
take.

**Alternatives rejected.** `match` over terms as a library call taking a runtime
pattern term, which is the interpretive half described above. Attributes as
ambient global properties of a symbol, which is the Mathematica arrangement and
forecloses compiled matching permanently.

**Reversal cost.** Low, now that D36 carries the durable half. The match-tree
representation is replaceable engineering behind D14's contract, and the decision
to compile rather than interpret costs only the compiler work already budgeted by
D33.

**Status: Decided (term patterns in function bodies compile), Open (match-tree
representation and its handoff to the native matcher, with the compiler).**

---

## D36. A term is an element of a quotient algebra, and the theory is carried by its operators

**What a term is.** A signature of operators with arities freely generates the
term algebra `T(X)`, in which `(x+2)+y` and `x+(2+y)` are different trees.
Declared laws generate a congruence `=E` on it, and a Vieta term denotes an
element of the quotient `T(X)/=E`. Layer A stores one representative per class,
and `ExprId` equality is equality in the quotient. That is the whole content of
"structural equality means something".

This makes the immutability rule below a mathematical statement rather than an
engineering accommodation. Declaring `Times` commutative after terms exist does
not adjust a flag on a symbol. It replaces one quotient with another, in which
different terms are equal, so stored representatives stand for classes the new
theory does not have. Redefining `f(x) = x^2` as `f(x) = x^3` moves no class at
all and changes only what a call computes. The two operations are unlike each
other at the level of the mathematics, which is why they land in different rows
of the table below.

**Decision.** "Attribute" covers four unrelated things, and only the first of them
determines what a term is.

| Kind | Examples | Determines | Where it lives |
|---|---|---|---|
| Canonical-shape law | associativity, commutativity, idempotence, unit, zero | the stored structure, therefore `ExprId` | the operator identity, immutable |
| Definition | `f(x) = x^2` | what a call computes | the world, versioned |
| Matching policy | Mathematica's `OneIdentity` | how a pattern matches an existing term | the world, versioned |
| Display metadata | precedence, fixity, notation | how a term prints | the world, versioned |

A term head is a resolved **operator identity**. Elaboration resolves the printed
`+` in a world to `Core.Plus`, and the term stores that. A later world binding `+`
differently binds it to a *different* operator and does not mutate `Core.Plus`.
Layer A therefore reads the shape laws through the head id with no world in hand,
and `Store::app(&self, head, args)` keeps the signature it already has.

**The theory is carried by the heads, and needs no separate index.** Each
operator's laws are fixed at its identity, so the theory of a term is a function
of the operators occurring in it. A theory identifier stored alongside a term
would encode the same fact more coarsely: a term mixing a commutative `Plus` with
a non-commutative `Times` has a theory that is the union of two per-operator
theories, so per-operator is the compositional form and a per-term tag is a
summary of it. Two structures over one printed name are two operators, which is
what `x * y` elaborates to differently in `CommutativeRing(Q, [x, y])` and in
`FreeAssociativeAlgebra(Q, [x, y])`. One source string, two free structures, two
terms that are correctly not comparable.

**One relation is not a per-operator law.** `x + x = 2*x` relates `Plus` to
`Times`, and no signature of either states it. `=E` is generated by the declared
signatures together with a fixed set of relations among the kernel's arithmetic
operators. That second part is the mathematical reason the kernel heads are a
closed set: the relations determining term identity are fixed, so no declaration
can enlarge the quotient governing operators already in shared use and invalidate
representatives already interned. `docs/layer-a.md` §4 calls these the two sources
of normalization.

The closure is on identity and not on what can be said. `D(f + g) = D(f) + D(g)`
and `transpose(A + B) = transpose(A) + transpose(B)` are cross-operator relations
and most of the subject is built from them. They live above Layer A, in rule sets
(D13), domain normalizers (D15), and guarded transformations (D3), where a
relation determines what a term rewrites to and leaves what a term is alone. A
user may also declare a new operator family with its own laws and get a genuine
quotient of its own. The single unavailable operation is retroactive enlargement
over shared operators.

**Declared once, immutable afterwards.** Declaring `f` associative when terms
headed by `f` already exist is rejected. The two answers available to the user are
to declare a different operator, or to bind the printed name to a new one in a new
scope. It is the same restriction as being unable to add two fields to a struct
and go on calling it the same type.

**The case this rules out, concretely.** Mathematica lets `SetAttributes[star,
Orderless]` land mid-session, and `ClearAttributes` take it away again. Before it,
`star[b, a]` stays as written; after it, the same input sorts to `star[a, b]`. An
expression evaluated earlier and kept under `HoldComplete` keeps the old
structure, so one session holds `star[b, a]` and `star[a, b]` under one head,
standing for an element of a free algebra and an element of its commutative
quotient at the same time. The head did not acquire a property. It changed which
algebra it belongs to, and the terms already built were not consulted.

Vieta answers with two operators. A `star` with no laws and a `star` declared
commutative are different identities, and elaboration decides which one a printed
symbol resolves to in a given module or domain, so `FreeAssociativeAlgebra(Q, [a,
b])` and `PolynomialRing(Q, [a, b])` can both spell their product `*` without
sharing a quotient. The restriction costs the user the ability to change an
operator in place. It buys the requirement to say which algebra the work is
happening in, which is the thing that actually changed.

**Implementation.** Intern the operator on `(module path, name)` and store its
canonical signature in the entry. Redeclaring an identical signature is the same
operator, which makes module reload idempotent. Redeclaring a different signature
under the same key is the error above. A new module path is a new key and
therefore a new operator, which is what makes the escape hatch work. No generation
counter appears anywhere, because the operation that would produce a second
generation is the one being forbidden.

**Ordinary redefinition never touches term identity.** `f(x) = x^2` becoming
`f(x) = x^3` changes what `f(a)` computes and leaves `f(a)` the same term. Rules,
domains, assumptions, notation, and matching policy are the same case. Only the
canonical signature sits in the identity, and the canonical signature is the one
thing a user is not invited to change after the fact.

**Syntax keeps what was written; terms are canonical.** `quote { y + x }` preserves
the source order, because quotation yields Syntax (D26), which is neither
normalized nor interned. Elaborating the same source into a term under `Core.Plus`
yields the canonical `x + y`. Syntax equality and term equality answer different
questions, both are available, and neither is traded for the other.

**The law vocabulary is closed.** Layer A normalizes against a fixed kernel set of
laws. Arbitrary user equations never enter it: `sin(x)^2 + cos(x)^2 == 1` is an
equational theory for the simplifier and for D13's rule sets, and it takes no part
in construction. This is D10's "not arbitrarily user-extensible" made concrete, and
it is what keeps Layer A's termination and confluence argument finite.

**What this hands to the Layer A specification.** Three bounded things. D10's scope
already folds annihilators, so the vocabulary carries a zero element alongside the
unit. Collecting like terms is arithmetic on the kernel's numeric heads instead of
a law any operator can declare, so the specification describes two sources of
normalization and says which is which. A unit is itself a term, so a signature
refers to terms whose heads are operators; declaration order makes that
well-founded, and a signature naming a unit headed by the operator being declared
is rejected.

**One naming trap, stated because it recurs.** `OneIdentity` in Mathematica is a
pattern-matching convention about `f[x]` standing for `x` while matching. It is not
the claim that the operator has an identity element, and it changes nothing about
what is stored, so it belongs to matching policy. The algebraic fact that `Plus`
has `0` as a unit is a different fact and belongs to the canonical signature. Vieta
names the two separately instead of inheriting the mixed attribute bag.

**Alternatives rejected.** Laws as world state with normalization relative to a
context, which costs D10 and D11 and drops `ExprId` equality from structural
equality of mathematical expressions to equality of unnormalized syntax. That is
the right model for Syntax and the wrong one for Terms. A fresh operator identity
on every redeclaration, which makes the forbidden operation silent instead of
impossible and hands the canonical printer a disambiguation problem for no gain.
A theory identifier in the intern key or a store per theory, which encodes what
the heads already determine, splits the pool so that a subterm common to two
theories is stored twice, and threads a theory argument through every construction
site that the head already carries.

**Why this is the entry that matters.** Two reasons, one mathematical and one
operational.

Changing a shape law changes which terms are equal, so it is a change of quotient
and not a change of configuration. Terms already stored were chosen as
representatives of classes that the new theory does not have, and there is no
sense in which they can be brought forward.

Operationally, the same fact appears as an asymmetry between code and data. A
compiled function that assumed `Orderless` on `Plus` can be invalidated and
recompiled. A node already flattened under `Flat` cannot: it is in the pool,
shared by everything that references it, held by live sessions, caches, and
derivations, with no recompile step for data. World capture handles code and does
not reach terms. Putting the shape laws in the identity removes the case instead
of managing it.

**Reversal cost.** Every term ever built, and the meaning of every equality in the
system. This is D10's reversal cost at the point where it actually gets decided.

**Status: Decided.** The signature vocabulary is fixed in `docs/layer-a.md` §3:
`associative`, `commutative`, `idempotent`, `unit`, `zero`, with `OneIdentity`
kept out of it for the reason above.

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

## D37. Concrete syntax is lossless; surface Syntax is semantically shaped

**Decision.** Two representations below Term rather than one.

```
CST       every token, including trivia, comments, redundant parentheses, and
          input that does not parse; spans into the source text
Syntax    names, spans, and binding forms, shaped for elaboration
```

```
source -> tokens -> CST -> Syntax -> resolution and elaboration -> Term (Layer A)
```

**Why one representation cannot serve both.** Exact source preservation and
convenient semantic structure pull against each other at every node. A redundant
parenthesis is semantically absent and source-significant. A comment is what a
formatter and a diagnostic are for and is nothing to elaboration. Malformed input
has to produce a tree for a REPL and an editor, and must never reach elaboration
at all. Hygienic macros (D29) need lexical scope and provenance that a Term must
never carry, which is the same argument D26 makes one level up and for the same
reason.

**What each one is for.** The CST serves the formatter, diagnostics with spans,
error recovery, incremental reparse, and the source view a macro sees. Syntax
serves elaboration, with binding forms visible and names still names.

**The acceptance property, and it is not D12's.** Printing a CST reproduces the
source byte for byte, and reparsing that output yields the same CST, for
well-formed and malformed input alike. D12's `parse(canonicalPrint(t)) == t` is
about Terms and says nothing about trivia or grouping. Both round trips exist,
they test different layers, and the syntax one is the first milestone of
`vieta-syntax`. The Term-level statement that follows it is
`elaborate(parse("x + 2")) == elaborate(parse("2 + x"))`, which is where the two
layers meet: Syntax keeps what was written and the elaborated terms enter one
Layer A class (D36).

**What byte-exact covers.** Vieta source is valid UTF-8, checked once before
lexing. Invalid UTF-8 is rejected with the offset of the first bad sequence
rather than carried as raw error bytes, so every span is a character boundary and
a mis-encoded file produces one diagnostic instead of a cascade of them.
Losslessness is about malformed *syntax*, and over accepted input it preserves
the byte-order mark, CRLF against LF, tabs, comments, every run of whitespace,
the spelling of numbers, the spelling of identifiers with no normalization
applied, and redundant parentheses.

**The invariant that makes it cheap.** Trivia are leaves of the tree rather than
attachments to neighbouring tokens, so printing is the concatenation of every
leaf's source text in order, and the spans of the non-synthetic leaves tile the
source exactly. The round trip is then a corollary of an invariant a test can
check directly, instead of a property to be hoped for.

**Three constraints that follow.**

- *No permanent trivia attachment.* `x + /* why */ y` has the leaf sequence
  identifier, whitespace, plus, whitespace, comment, whitespace, identifier. An
  API may present trivia as leading or trailing, and that presentation is a view.
  Baking an attachment rule into the representation makes it wrong the first time
  a formatter or an edit disagrees with the rule.
- *Recovery never fabricates printable bytes.* A missing token is a synthetic
  leaf with an empty span, so `f(x` prints back as `f(x` and not as `f(x)`. The
  lexer is total: every step consumes at least one character or reaches the end,
  and an unrecognized character becomes an error token rather than a trap.
- *Provenance crosses into Syntax.* Every Syntax node carries an origin: a source
  span, a recovery synthesis, or a macro expansion once D29 exists. Diagnostics,
  hygiene, derivations, and pointing a user at the source expression that produced
  a term all need it, and none of them can recover it afterwards.

**Origin does not stop at `Syntax`.** D33 carries the same requirement through
elaboration and into the compiled artifact, whose origins live in an optional side
table from instruction range to origin. Two consequences for the representation
chosen here. An origin is a chain rather than a single span, since macro-generated
code has a definition site and an invocation site and a user needs both. And a
transformation composes origins rather than copying one, so elaborating infix
syntax into a call keeps the whole source expression rather than the operator
token. §13.6.

**What this does not gate.** D6. The parser recognizes binding *forms* without
deciding the alpha-invariant encoding, emitting named surface forms such as
`Lambda(name, body)` and `Let(name, value, body)`. Elaboration converts them
later. D6 gates elaboration and the store's traversal API, and it does not gate
the parser, so `vieta-syntax` can start while the enumeration is still open.

**Alternatives rejected.** One Syntax type doing both jobs, which is the shortcut
that costs the formatter and hygiene. Reparsing from source whenever trivia is
needed, which makes every diagnostic and every macro expansion depend on the
source text still being available and identical.

**Working assumption, not part of the decision.** Syntax owns its own nodes.
A lossless green tree with a typed view layered over it (the Roslyn and
rust-analyzer shape) is the same two layers with shared storage and stays
available; it is a representation choice behind the boundary this entry fixes.

**Reversal cost.** Retrofitting losslessness after the elaborator, the printer,
and the macro expander exist means re-auditing every consumer of Syntax and
re-deriving trivia from source for anything that needs it. The formatter and
hygienic macros are the two features that stop being reachable.

**Status: Decided (two layers, the pipeline, the byte-exact round trip, and origin
as a composable chain that outlives Syntax), Open (whether Syntax owns nodes or is
a view, the concrete origin representation, and the concrete surface grammar).**

---

## D38. Elaboration precedes the instruction set, and the first instruction set is evidence

**Decision.** The bytecode instruction vocabulary is derived from the resolved
executable form, not from surface syntax. The dependency runs one way.

```
surface Syntax
  -> name resolution, binding, quotation, world capture
  -> resolved executable form
  -> instruction set
```

The first instruction set to exist is provisional and expected to be discarded. It
is compiled, run, and read as evidence. The durable contract registers after
several representative programs compile cleanly through it.

**Why the order is not a preference.** An instruction set designed against the
surface grammar becomes a serialized AST: one opcode per surface form, with the
resolution work still to be done at run time. D33 already commits the artifact
format and the origin map (§13.6) to outliving individual programs, so the encoding
is the part of the machine that least tolerates a guess.

**What elaboration settles that the machine needs.** Each of these changes the
opcode list, and none is answerable from the grammar.

- Whether a call is lexical, world-bound (D20), or dynamic, since that is three
  instructions or one instruction with a resolution step at run time.
- How a closure captures its environment (D34).
- How binders are represented (D6), which is the question the store also asks and
  is not required to receive the same answer.
- Whether symbolic construction is a machine primitive or an ordinary call, which
  is D35 asked at the opcode level.
- How guarded results (D3) and matching (D14, D35) enter control flow.
- Where origin attaches once desugaring has moved code (§13.6).
- Which surface constructs disappear before execution and need no instruction at
  all.

**The first elaboration slice.** Literals, names and symbol resolution, calls,
`let`, `lambda`, quotation and term construction, and origin propagation. Nothing
else. It is sized to expose the machine model rather than to cover the language,
and it forces D6's encoding in a concrete setting instead of an abstract one. D6
gives the slice its binding shape: named binders become `BinderId`s, occurrences
resolve to a local reference or a global symbol, free-variable analysis over the
result gives the capture set the compiler needs, and quotation elaborates into a
symbolic binding context that builds a store term.

**No Core IR is registered.** The resolved form lives behind `vieta-syntax` or in a
`vieta-elab` crate, and whether it becomes an official Core IR stays open until it
has compiled something. Registering an intermediate representation before anything
runs through it is the same mistake as freezing the instruction set early, one
level down.

**The sequence.**

```
1. Resolve D6 far enough for lexical binding
2. Implement the first elaboration slice
3. Compile it into a small provisional bytecode
4. Run representative programs and observe
5. Register the durable instruction-set contract from what they show
```

Representative means at least a `let` with an arithmetic body, a named function, a
quotation, and a match on a variadic head.

**What this does not relax.** The irreversible machine properties are fixed already
and do not wait for evidence: origin propagation and the artifact origin map, the
logical frame model, the inert observation boundary (§13.6), fuel and cancellation
(D22), and compilation against versioned worlds (D20). A provisional encoding is
free to change. A provisional encoding that omits those is not provisional, it is a
restart.

**Alternatives rejected.** Designing the instruction set from the surface grammar,
which produces a serialized AST and moves resolution into the run-time path.
Registering a full Core IR now. Keeping the first encoding because it works, which
makes "provisional" a label rather than a plan.

**Reversal cost.** Asymmetric, which is why the order is registered rather than
left to taste. Deriving the instruction set from elaboration costs one throwaway
encoding and the time to write it. Freezing it first costs every artifact stored
under it plus the origin map format D33 commits to outliving programs, and that
cost stays invisible until the first non-trivial program is compiled.

**Status: Decided (the order, and that the first encoding is disposable), Open (the
resolved form's shape, and whether it becomes the Core IR).**

---

## Open items

| Item | Needed before | Reference |
|---|---|---|
| Binder encoding (indices, levels, locally nameless), separately on each side | With the first elaboration slice | D6, D38 |
| Shared traversal and substitution API both binder layers go under | With the first elaboration slice | D6 |
| Enumeration of what binds | Before choosing the encoding | D6 |
| Whether assumption contexts accept quantified propositions | With the enumeration | D6, §2.5, §13.1 |
| Tag-bit layout in the id space | Slice 1 | D7, measured in the store itself, §1.9 |
| Shape of the resolved executable form | With the first elaboration slice | D38 |
| Whether the resolved form becomes an official Core IR | After the provisional bytecode runs | D38 |
| Durable bytecode instruction set and calling convention | After representative programs run on the provisional one | D33, D38 |
| Origin representation as a chain, and the artifact origin map | With the instruction set | D33, D37, §13.6 |
| Logical frame model, and the inert observation boundary | With the instruction set | D33, D22, §13.6 |
| Resource limit at construction, or none | Before the store holds anything | D10, D22, `layer-a.md` §14 |
| Machine value representation, all of it | After the machine can be profiled | D34 |
| Match-tree representation and matcher handoff | With the compiler | D35 |
| Matching semantics contract content | Rule count in the low hundreds | D14 |
| Reification contract (totality, injectivity, value/term equality, `describe` versus `reify`) | Slice 1 ships | D26, §13.6 |
| Surface grammar | Slice 1 | D37; explicit `*` in core, implicit multiplication confined to a marked math-input mode |
| Whether Syntax owns nodes or is a typed view over the CST | With the parser | D37 |
| Strategy combinator vocabulary | Rule corpus past a few dozen | D28 |
| Objective vocabulary, and whether `convert` is a sixth operation | Before `simplify` has users | D27, §13.2 |
| Store GC algorithm | M1 | D8, epoch or region-based with compaction is the working assumption |
| Conformance suite format | M1 | Implementation-independent |
| FLINT `gr` capability audit | M2 | D15, no dependency on the spine, can start now |
| Coercion arrows: partiality, obligations, and the `RewriteResult` shape | M2 | D3, D15, §11.2 trap 5, §13.4 |
| Lower-compute-lift contract | Second domain family, by D16's rule | §5, D16, §13.5 |
| Native cancellation strategy | M2 | D22 |
| World invalidation versus pinning policy | M4 | D20 |
| Effects checked versus declared | M4 | D30 |
| Method versus rule boundary | M4 | D31 |
| Laziness mechanism in Vieta | M5 | D21 |
| Where numerically checked evidence lives | M7 | D3, D4, D24, §13.3 |
| Concurrent hash-cons strategy | When anything is parallel | Sharded tables or lock-free insert |
| License | Before any publication | Permissive maximizes survivability for a solo project |
