# Vieta: Measurements

Numbers taken from the shipping components rather than from prototypes, per
`architecture.md` §1.9. Every entry records the machine, the date, the command,
and the corpus. A number without its N is not a measurement.

---

## Store, 2026-07-28

Windows 11, i9-14900K, 64 GB. rustc 1.95.0, release profile, single thread.

```
$env:VIETA_MEASURE_ATTEMPTS = '20000000'
cargo test -p vieta-store --release --test measure -- --nocapture
```

Corpus: applications of seven heads at arity one to three, built in eight rounds.
Within a round the set of available arguments is frozen at 44 terms, half leaves
and half a spread sample of the previous round's output. Freezing is what bounds
the reachable combinations and so forces the repeats; a churning pool produces a
corpus that shares nothing, which measures the wrong thing. Three of the seven
heads are `Plus`, `Times`, and `Power`, so the corpus runs Layer A on roughly
three applications in seven.

| | |
|---|---|
| applications issued | 20,000,000 |
| nodes interned | 2,158,672 |
| symbols | 12 |
| argument words | 6,440,792 |
| intern table slots | 4,194,304 |
| sharing | 9.26 applications per node |
| memory per node | 31.7 bytes used, 46.6 bytes reserved |
| heap | 65.3 MB used, 96.0 MB reserved |
| construction | 4.16 M applications/s |
| intern lookup | 3.13 M applications/s |
| structural equality | 1.98 G comparisons/s |
| whole-store walk | 88.7 M nodes/s |

Where the 31.7 bytes go, per node: 12 for the node record, 11.9 for arguments at
a mean arity of 2.98, 7.8 for the intern table at 51 percent occupancy. Reserved
exceeds used because `Vec` growth doubles; resident memory pays the reserved
figure and the layout costs the used one.

**What Layer A costs and buys, on this corpus.** The same run against the store
before Layer A existed, interning only, gave 9.43 M applications/s and 6.39
applications per node. Normalization at construction therefore costs a factor of
2.3 in construction rate and returns 45 percent more sharing, since permutations
of a commutative head now reach one node and units and annihilators collapse
before anything is interned. At 4 M applications/s construction is not a
bottleneck for anything the language does next.

Intern lookup is slower than construction here, which reverses the earlier run.
The repeat pass sees the table at its final 4.2 M slots while the first pass grew
into it, so the repeat pays cache misses the first pass did not.

**What this settles for D7.** Three tag bits leave a 29-bit payload, so the node
index saturates at 536 million. At 31.7 bytes per node that is roughly 17 GB of
store before the id space runs out, so memory binds first by a wide margin and
the payload width is not under pressure. Six of the eight tags are assigned. The
layout stands.

**What it says about the intern table.** The table is 26 percent of resident
memory at 51 percent occupancy, growing at a 70 percent load factor, which makes
the growth threshold the largest single memory knob in the store and a cheap
thing to revisit under measurement. It is not a tag-layout question and does not
block anything.

**Not measured here.** The candidate index under rule load (§1.9 item 2), which
has nothing to run against until the rule representation exists, and the FLINT
arena round-trip (§1.9 item 3), which belongs with the `gr` capability audit and
sizes M2.
