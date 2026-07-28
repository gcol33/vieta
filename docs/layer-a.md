# Layer A: construction-time normalization

Specification, version 1. Required by D10 before Layer A has an implementation.
Written against D2 (four meanings of equality), D7 (the store), D11 (construction
is pure), D12 (two printers), D22 (resource limits), and D36 (canonical-shape
laws are part of the operator identity).

---

## 1. What Layer A is

Layer A is a function

```
normalize(head: ExprId, args: &[ExprId]) -> ExprId
```

applied by `Store::app` to every application before it is interned. It is the
only thing construction does (D11).

**What it computes.** A signature of operators with arities freely generates the
term algebra `T(X)`, in which `(x+2)+y` and `x+(2+y)` are different trees. The
declared laws generate a congruence on it, and a Vieta term denotes an element of
the quotient. `normalize` picks the representative of a class, so `ExprId`
equality is equality in the quotient rather than equality of trees. Everything
below is the working out of that one sentence: §3 and §7 say which laws generate
the congruence, §5 says how the representative is chosen, and §10 proves that the
choice is well defined. The theory is a function of the operators appearing in a
term, because each operator carries its laws (D36), so no context is needed to
know which quotient a term lives in.

**Normal-form closure.** Every `ExprId` a store hands out is in Layer A normal
form. By induction: atoms are normal, and `normalize` returns a normal id when
given normal arguments. Construction is bottom-up, so the arguments `normalize`
receives are already normal, and it never has to renormalize a subterm.

This is the structural fact the rest of the specification rests on. Layer A is
not a rewriting relation run to a fixpoint. It is a single function applied once
per node, and the obligations that come with that are termination of one call
(§9) and canonicity of the result (§10), both of which are far smaller than
confluence of a rewrite system over arbitrary terms. It is also why Layer A can
sit on the store's hot path.

**Four things Layer A may not do.**

| Prohibition | Reason |
|---|---|
| Fail | D10 requires totality. `Store::app` returns an `ExprId`, not a `Result` |
| Consult a world | The laws are read through the head id, which carries them (D36) |
| Emit a condition | The return type is an id. There is nowhere to put a guard, a `Status`, or a second branch |
| Recurse without bound | §9 |

---

## 2. The unconditional-validity criterion

Layer A returns an id and nothing else. A law it applies is therefore asserted
with no side condition, in every context the term will ever be read in. That
single criterion decides most of the boundary questions, and it decides them the
same way every time.

Three worked cases:

- `x^1 -> x` is in. It holds for every `x`, including `0`.
- `x^0 -> 1` is out. It fails at `x = 0`, where `0^0` is contested.
- `x * x^(-1) -> 1` is out. It fails at `x = 0`, and item 9 of the acceptance
  demo (§9.1 of `architecture.md`) is the requirement that `(a^2-1)/(a-1)`
  reduce to `a+1` **with `a != 1` recorded**. Silent cancellation at
  construction would delete the condition the whole thesis is about. Layer A
  never cancels.

**Declared laws are taken as true.** When an operator's signature says
associative, Layer A flattens without checking. The obligation belongs to
whoever declared the operator. This is not an exception to the criterion, it is
where the criterion is discharged: at declaration, once, rather than at every
construction.

**The domain the laws are asserted over** is the kernel's arithmetic domain,
which contains exact numbers and symbols standing for elements of it. It
contains no infinities and no indeterminate forms. A consequence worth writing
down before it is found as a bug: `x - x` is the term `0` from the moment it is
constructed, so substituting an undefined value for `x` afterwards does not
agree with substituting first and normalizing after. Every computer algebra
system behaves this way, and the alternative is to abandon construction-time
normalization entirely.

---

## 3. The canonical signature

Every operator identity carries one, immutable from declaration (D36):

```rust
struct CanonicalSignature {
    associative: bool,
    commutative: bool,
    idempotent: bool,
    unit: Option<ExprId>,
    zero: Option<ExprId>,
}
```

| Field | Equation | Effect on the stored form |
|---|---|---|
| `associative` | `f(a, f(b, c)) = f(a, b, c)` | Arguments headed by the same operator are spliced into the parent. `f(a)` is `a`. `f()` is the unit when one is declared |
| `commutative` | `f(a, b) = f(b, a)` | Arguments are sorted into the canonical order (§5) |
| `idempotent` | `f(a, a) = f(a)` | Duplicate arguments are dropped. Without `commutative`, only adjacent duplicates |
| `unit` | `f(a, e) = a` | Arguments equal to `e` are dropped |
| `zero` | `f(a, z) = z` | An argument equal to `z` makes the whole application `z` |

**The empty signature is the default.** An ordinary symbol head has every flag
false and both elements absent, so `normalize` takes one branch and interns.
This is the common case and it must stay the cheap one.

**`unit` and `zero` are terms.** A signature therefore refers to terms whose
heads are operators. Declaration order makes that well-founded, and a signature
naming a unit or zero headed by the operator being declared is rejected at
declaration time (D36). The reference is to an interned id, so the check at
construction is a `u32` comparison.

**What is deliberately absent from the vocabulary.**

- `OneIdentity`. It is a pattern-matching convention about `f(x)` standing for
  `x` while matching, it changes nothing about what is stored, and it belongs to
  the world's matching policy (D36). The variadic collapse `f(a) -> a` above is
  a different fact, it follows from associativity, and it is not named after
  Mathematica's attribute.
- Distributivity. It does not preserve size in either direction, it does not
  terminate in the expanding direction, and choosing a direction at construction
  is exactly the automatic simplification D11 forbids.
- Inverses. `f(a, inv(a)) = e` is the cancellation §2 rules out.
- User equations. `sin(x)^2 + cos(x)^2 == 1` is an equational theory for the
  simplifier and for D13's rule sets. The vocabulary is closed (D36), and that
  is what keeps §9 and §10 finite arguments rather than open-ended ones. The
  deeper reason it cannot be opened is in §0.7 of `architecture.md`: the laws
  admitted here are the ones with a known cheap canonical form, and the word
  problem for a general finitely presented algebra is undecidable, so no
  constructor can accept an arbitrary presentation and return representatives.

---

## 4. The two sources of normalization

**Declared laws** are the table above. They are generic: one implementation
reads the signature through the head id and applies to every operator equally.
Any operator may declare them.

**Kernel arithmetic** is a fixed set of rules attached to three specific
operator identities, `Core.Plus`, `Core.Times`, and `Core.Power` (§7). It is not
declarable, not extensible, and not derivable from any signature. `x + x -> 2*x`
needs `Plus` and `Times` to be related to each other, and no per-operator law
expresses that.

The line between them is the answer to "can a user get this behaviour for their
own operator?" Declared laws, yes. Kernel arithmetic, no. Adding a rule to
kernel arithmetic is a kernel change and bumps the version (§11). The
requirement on any such addition is §2: unconditionally valid, exact, and
terminating.

Stated in the terms of §1, the congruence is generated by the declared signatures
together with a fixed set of relations *among* the kernel operators. A declared
law is a relation an operator has with itself. A kernel rule may relate two
operators, and that is the capability held back from declaration.

**The closure is on term identity, and on nothing else.** Cross-operator
relations are most of the subject. `D(f + g) = D(f) + D(g)`,
`transpose(A + B) = transpose(A) + transpose(B)`, and a user product distributing
over `Plus` are all statable, and they live above Layer A in rule sets (D13),
domain normalizers (D15), and guarded transformations (D3), where a relation
determines what a term rewrites to and leaves what a term *is* alone. A user may
also declare a new operator family carrying its own laws, which is a genuine
quotient of its own and is not restricted here. What is unavailable is enlarging
the quotient that governs operators already in shared use, because that changes
which existing terms are equal and invalidates representatives already interned.

---

## 5. The canonical order

Sorting a commutative operator's arguments needs a total order on ids. Three
requirements fix it.

**It must be content-determined, not id-determined.** Sorting by the raw
`ExprId` word is the cheap and wrong choice, for three separate reasons:

- ids are renumbered by a compacting collector at safepoints (D8), so a stored
  argument list sorted by old ids is no longer sorted afterwards, and the next
  construction of the same term would intern a second, duplicate node;
- ids differ between processes, so a store segment on the wire (D23) and the
  store that receives it would disagree about canonical form;
- canonical printed output would depend on the order symbols happened to be
  interned in, which makes replayed sessions print differently and makes an
  implementation-independent conformance suite impossible.

**It need not be mathematically conventional.** The canonical printer prints
whatever this order says (D12), and the pretty printer is free to present a sum
in whatever arrangement reads well, because it only owes alpha-equivalence under
reparsing. Contorting the canonical order for appearance would buy nothing that
D12 has not already bought.

**The order.**

1. By kind: numbers, then symbols, then applications.
2. Two numbers: by numeric value. Each exact number has exactly one id, because
   rationals are stored in lowest terms with a positive denominator, a unit
   denominator yields an integer, and a value takes the inline tag when it fits
   (`id.rs`). Two numbers compare by value across tags.
3. Two symbols: by name, comparing UTF-8 bytes. Not by symbol-table index, which
   is insertion-ordered. Not locale-dependent.
4. Two applications: by head, then by arity, then argument by argument left to
   right.

Totality follows from interning: two distinct nodes differ in head, arity, or
some argument, so the comparison finds a difference. Well-foundedness follows
from arguments being constructed strictly before the node that holds them.

**Cost.** The comparison begins with `a.bits() == b.bits()`, which returns
Equal in one instruction. Hash-consing makes that the case for every shared
subterm, so the recursive descent only runs where two terms genuinely differ,
and it stops at the first difference. The remaining cost is symbol name
comparison, which may later be replaced by a cached rank in the symbol table.
Any such cache must reproduce this order exactly.

---

## 6. The procedure

Given `head` and normal `args`.

```
0.  signature = signature_of(head)
    if signature is empty and head is not a kernel arithmetic head:
        intern(head, args)                          // fast path

1.  if signature.associative:
        splice arguments whose head is `head`
2.  if head is a kernel arithmetic head:
        fold exact numeric arguments                 // §7
3.  if signature.zero is Some(z) and z occurs in args:
        return z
4.  if signature.unit is Some(e):
        drop arguments equal to e
5.  if signature.commutative:
        sort args by the canonical order             // §5
6.  if signature.idempotent:
        drop duplicates (adjacent only when not commutative)
7.  if head is a kernel arithmetic head:
        collect like terms                           // §7
8.  if signature.associative:
        args.len() == 1  ->  return args[0]
        args.len() == 0  ->  return signature.unit when present
9.  intern(head, args)
```

**Why this order.** Each pass establishes a precondition the next one needs, and
no pass recreates work for an earlier one.

| Pass | Precondition it needs | Established by |
|---|---|---|
| Fold numerics | Nested same-head arguments already lifted | 1 |
| Zero | A zero buried in a nested node already exposed | 1 |
| Dedupe | Duplicates adjacent | 5 |
| Like terms | Coefficients are single folded numbers | 2 |
| Arity collapse | The list is final | 1 through 7 |

Checking the other direction, that no pass invalidates an earlier one: unit
removal and dedupe only delete, so they cannot produce a same-head argument
needing another flatten, and cannot disturb the sort; zero returns immediately;
like-term collection builds `Times` nodes, whose head differs from `Plus`, so
nothing needs reflattening; arity collapse returns an argument that was already
normal. This is the content of local confluence, discharged by inspection over a
fixed, closed set of passes rather than argued in general.

---

## 7. Kernel arithmetic

Three operator identities, with these signatures:

| Operator | associative | commutative | idempotent | unit | zero |
|---|---|---|---|---|---|
| `Core.Plus` | yes | yes | no | `0` | none |
| `Core.Times` | yes | yes | no | `1` | `0` |
| `Core.Power` | no | no | no | none | none |

`Power` is not associative: `(x^y)^z` and `x^(y^z)` differ. Its whole
normalization is kernel arithmetic.

**Numeric folding.** Exact numbers among the arguments of `Plus` and `Times` are
combined into one, which the sort then places first. Exact arithmetic is
associative and commutative, so the fold order does not affect the result.

`Power` folds when the exponent is an integer:

- `2^3 -> 8`, `2^(-1) -> 1/2`, `(-2)^3 -> -8`.
- `0^(-1)` does not fold. There is no exact result, and §8 says what happens.
- A non-integer exponent does not fold, even when an exact value exists.
  `4^(1/2)` has the value `2`, and `(-8)^(1/3)` has three cube roots of which
  the real one and the principal complex one differ. Folding one and not the
  other would put a branch choice in the constructor. Root extraction belongs to
  a layer that can name the branch.
- `x^1 -> x` for any `x`, per §2.

**Like terms in `Plus`.** Each argument splits into a coefficient and a
monomial: a `Times` node whose first argument is a number splits into that
number and the `Times` of the rest; anything else has coefficient `1`. Arguments
are grouped by monomial id, coefficients are summed, groups summing to zero are
dropped, and a group with coefficient `1` is emitted as the bare monomial. The
grouping key is an id, so it is a `u32` comparison.

This is what makes `x - x` the term `0` and `x/2 + x/2` the term `x`, since `-x`
is `Times(-1, x)` and both fall out of coefficient arithmetic followed by the
unit and zero laws on `Times`.

**Like bases in `Times`.** Arguments are grouped by base, where `Power(b, e)`
contributes `(b, e)` and anything else contributes `(a, 1)`. A group whose
exponents are **all positive integers** is combined into one `Power` with their
sum. Any other group is left alone.

The restriction is §2 again. `x^m * x^n = x^(m+n)` for positive integers holds
at `x = 0`, where both sides are `0`. With mixed signs it does not: `x^2 *
x^(-1)` and `x` disagree at `0`, where the left side is undefined. Combining
only the positive submultiset of a mixed group is still a function of the
multiset, so canonicity (§10) is unaffected.

Consequences: `x * x -> x^2`, `x^2 * x^3 -> x^5`, and `x * x^(-1)` stays a
product, which is the uncancelled form the acceptance demo requires.

---

## 8. Totality

Layer A never fails. When a fold has no exact result, the fold is not performed
and the node is interned as written. `Power(0, -1)` is a term. What it means is a
question for evaluation, which has `Status::OutsideDomain` (D4) and can answer
it, and for the assumption engine, which can refuse to.

This is the general policy: **Layer A declines rather than deciding.** Anything
it cannot settle unconditionally and exactly, it leaves standing.

---

## 9. Termination

One `normalize` call performs nine passes over an argument list that is finite
and only shrinks after step 1. No pass loops. The only question is nested
`normalize` calls, which arise in step 7 when like-term collection builds a
`Times` node, and in `Times` when like-base collection builds a `Power` node.

Fix the rank `Power < Times < Plus`. Every nested call from kernel arithmetic
goes to a strictly lower rank: `Plus` builds `Times` nodes, `Times` builds
`Power` nodes, `Power` builds nothing. The chain is fixed and has length three,
so nesting depth is bounded by three, and each nested call terminates by the
same argument. Steps 1 through 6 and 8 build no nodes at all.

Hence every `normalize` call terminates, and the store's construction of a term
of `n` nodes performs `n` terminating calls.

The vocabulary being closed (§3) is what makes this argument finite. A
user-extensible law set would reopen it at every declaration.

---

## 10. Canonicity

Let `≡_A` be the congruence generated by the declared laws of the operators
appearing in a term, together with exact arithmetic and the kernel rules of §7.
The theorem Layer A owes:

> For all terms `t` and `u`, `normalize(t) = normalize(u)` if and only if
> `t ≡_A u`.

Stated through the quotient: let `q : T(X) -> T(X)/≡_A` be the quotient map.
Layer A fixes a canonical-representative function `s : T(X)/≡_A -> T(X)` with
`q ∘ s = id`, which is a section of `q`, and `normalize` is the induced map
`s ∘ q` on raw terms. It sends a term to the chosen representative of its class.
Interning then gives that representative a word-sized name, which is where
`ExprId` equality comes from. The section is what §5 and §6 construct.

Sections exist without any order at all, since a representative may be chosen
from each class arbitrarily. What §5's total order supplies is *this* section: a
choice rule that is deterministic, computable, and reproducible across processes,
which is the only kind a machine can implement. It does not supply minimality.
The canonical form is what §6's passes produce and is not the least element of
its class, since flattening changes arity and §5 compares arity, so
`Plus(a, Plus(b, c))` precedes the `Plus(a, b, c)` it normalizes to.

**Left to right (soundness).** Every pass replaces a term by an `≡_A`-equal one:
steps 1 through 6 are the declared equations read left to right, step 7 is
kernel arithmetic, step 8 is associativity at arity one and the unit law at
arity zero. Composition of `≡_A`-preserving steps preserves `≡_A`. Two terms
with the same id are therefore `≡_A`-equal, and hence mathematically equal
whenever the declarations are true. This is the direction that must never break,
because it is what D2's structural equality claims.

**Right to left (completeness for `≡_A`).** The normal form is a canonical
representative of its `≡_A` class, established one layer at a time:

| Laws declared | Normal form is | Why it is canonical |
|---|---|---|
| A | the left-to-right sequence of non-unit leaves | elements of the free semigroup are exactly such sequences |
| A, C | that sequence sorted | equal multisets, one total order, one sequence |
| A, C, U | with units removed | the free commutative monoid is finite multisets over non-unit generators |
| A, C, U, I | deduplicated | idempotence collapses the multiset to its underlying set |
| plus a zero | the zero itself | every term containing it is `≡_A` to it |
| plus exact folding | one number for the whole numeric submultiset | exact `+` and `*` are associative and commutative, so the submultiset has one value |
| plus like terms | a map from distinct monomials to nonzero coefficients | two terms `≡_A` under the linear rules have the same map, and monomials are compared as ids |

Each layer's form is stable under the ones before it, which §6 established by
inspecting the passes.

**Where completeness stops, deliberately.** `≡_A` is a strict subtheory of
mathematical equality. `(x+1)^2` and `x^2 + 2x + 1` have different ids, because
expansion is not in `≡_A` and D11 forbids it at construction. Domain equality
and provable equality are separate predicates over the store (D2), and the
equivalence structure that relates ids without merging them is a context-scoped
layer, never a mutation of store identity (D9).

So the honest statement of what `x + 2 === 2 + x` means: **same id implies
mathematically equal; mathematically equal does not imply same id.**

---

## 11. Versioning

One monotone integer, `LAYER_A_VERSION`, currently `1`. It is stamped on
serialized store segments (D23), on sessions (D19), and on derivations (D24),
and it is what makes a replayed trace trustworthy across releases (D10).

It bumps when the normal form of any term changes: a law added to or removed
from the vocabulary, a change to the canonical order, a change to the fold
rules, or a change to a kernel operator's signature. It does not bump for
performance work that leaves every normal form identical.

Loading a segment stamped with a different version is safe, because
deserialization replays construction bottom-up through `app` and therefore
recanonicalizes under the current rules. What the stamp buys is the knowledge
that ids recorded inside that segment's derivations and caches may no longer
name the same terms.

---

## 12. Not in Layer A

Expansion, factoring, common-denominator collection, trigonometric identities,
logarithm and exponential rules, radical simplification, cancellation of any
kind, domain-dependent rewriting, and anything that needs an assumption. All of
it is Layer C, reached through `simplify`, returning guarded results (D3) with
conditions recorded.

`Hold` and `Inert` (D11) suppress Layer A for faithful representation of input,
which is how a term that Layer A would otherwise normalize can be shown as
written. Item 3 of the acceptance demo is exactly this.

---

## 13. What this hands to the implementation

1. An operator table in the store, keyed on `(module path, name)`, holding a
   `CanonicalSignature` per entry (D36). `Store::symbol` becomes the degenerate
   single-module case of interning into it.
2. `Store::app` keeps its signature. It stays infallible, per §1 and §8.
3. A canonical comparison over ids implementing §5, content-determined, opening
   with the identity short-circuit.
4. A prelude of kernel operators interned at store construction, so that
   `Core.Plus`, `Core.Times`, and `Core.Power` and the ids of `0` and `1` are
   available before any arithmetic term is built.
5. Tests, of which four are the load-bearing ones:
   - `x + 2` and `2 + x` intern to the same id (acceptance demo item 4);
   - a property test over random terms: any permutation of a commutative
     operator's arguments interns to the same id, and no permutation of a
     non-commutative one does;
   - two stores that intern symbols in different orders produce byte-identical
     canonical printed output for the same term, which is what §5's
     content-determined requirement actually means;
   - `x * x^(-1)` does not become `1`, and `(a^2-1)/(a-1)` does not become
     `a+1`, at construction.

---

## 14. Open

**Construction has no resource limit.** D22 puts fuel and cancellation in the
evaluation context, and Layer A does not run there. `2^(10^9)` folds to a
gigabyte-scale integer inside `Store::app`, with nothing to stop it, and no
Ctrl-C path reaches it. Four answers:

| Answer | Cost |
|---|---|
| Leave it | A single expression can hang the kernel with no interrupt |
| Decline folds above a size ceiling | `app` stays infallible, and `≡_A` acquires a dependency on a budget that §10 would have to state |
| Make `app` fallible on the fold | Same budget problem, plus an API change at every construction site |
| Give the store D22's cancellation token, and let a long fold observe it | `app` becomes fallible with exactly one failure mode, and `≡_A` is untouched because a cancelled construction yields no term at all |

The fourth is the recommendation. Cancellation is not a normalization choice, so
it does not belong in the equational theory, and D22 already requires the token
to reach everything long-running. Taking this before the store holds anything is
cheaper than retrofitting a fallible constructor later.

**Like bases in `Times` (§7) is the specification's own call**, not something
D10 named. D10's scope says "collect like terms", which is the additive rule.
The multiplicative one is included because leaving `x * x` unnormalized while
`x + x` normalizes is an asymmetry users would hit immediately. It can be
dropped without touching anything else in this document.

**A cached symbol rank** for §5, if name comparison shows up in a profile. It
must reproduce the specified order exactly.
