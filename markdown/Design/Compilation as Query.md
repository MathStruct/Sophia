# Compilation as Query

The README says the goal is to "build a compiler that queries the graph db and assembles IRs back into the graph database". This note works out what that means operationally, and why it is not merely a storage change.

## The reframing

| Classical compiler | Sophia |
| --- | --- |
| pass = function over an IR in memory | pass = rewrite rule = query producing new nodes/edges |
| pass pipeline, fixed order | rule set, run to fixpoint ([[Equality Saturation]]) |
| IR discarded after each pass | every intermediate persisted and addressable |
| incremental build = timestamp/file granularity | incremental = per-node hash granularity |
| cache = artifact keyed by filename | cache = node keyed by its own content |
| "what did the optimiser do?" — rerun with `-print-after-all` | a query over stored `LOWERS_TO` edges |

The last row is the one that is immediately, boringly useful even if the grand cross-language goal never lands: a persistent record of *why* the compiler produced what it produced.

## A pass as a rule

A rewrite rule is itself a node in the graph (so it is versioned, hashable and citable by a [[Equivalence and Witnesses|witness]]). Concretely a rule is a pattern, a guard, and a production — which is exactly a Datalog-with-generation clause:

```datalog
% constant folding, as a rule over core terms
folds(App, Const) :-
    term(App, "app"), child(App, 0, F), child(App, 1, X), child(App, 2, Y),
    prim(F, "add.wrap.i64"), lit(X, A), lit(Y, B),
    Const = mk_lit(wrap_add_64(A, B)).

equiv(App, Const, "rewrite", {}) :- folds(App, Const).
```

Note what the rule emits: not a *replacement* but an **equivalence**. Nothing is ever destroyed. This is the [[E-Graph|e-graph]] discipline, and it is forced on us anyway by the append-only store ([[Graph Schema]]).

## Compilation = saturate, then extract

1. **Seed.** Load the `Decl` reachable from the requested root, plus its transitive `DEPENDS_ON` closure, into an e-graph.
2. **Saturate.** Apply the rule set until no new equivalences are produced, or a node/time/iteration budget is hit. Equality saturation dissolves the [[LLVM Pass|phase-ordering problem]]: there is no "should I inline before or after constant folding", because both are recorded and the choice is deferred.
3. **Extract.** Choose one representative per equivalence class, minimising cost. As an integer program, with `x_n = 1` iff node `n` is selected:

$$ min sum_n c(n) dot x_n $$
$$ "s.t." quad ∀ "class" C : sum_(n ∈ C) x_n ≥ 1 $$
$$ ∀ n "selected", ∀ m ∈ "children"(n) : x_n ≤ x_m $$

  Cost `c` is target-dependent (instruction count, estimated latency, code size) and comes from the `Target` node. The acyclicity requirement — the extracted DAG must not contain a cycle through e-classes — is what makes extraction NP-hard in general; the standard practical answer is greedy bottom-up extraction with a cycle check, escalating to ILP only for small hot regions.
4. **Lower.** Emit [[MLIR Operation|MLIR ops]] for the extracted DAG, write them back as nodes with `LOWERS_TO` edges, then hand off to `mlir-opt` / LLVM for the parts we are explicitly not reimplementing ([[MLIR Lowering]]).
5. **Validate.** Optionally run per-instance equivalence checking on the lowering and store the result as a `Witness` ([[State of the Art - Program Equivalence Checking]]).

## Incrementality falls out

Because every input is content-addressed, every query result can be memoised on `(rule-set hash, query hash, input hash, target hash)`. A change to one function invalidates exactly the memo entries that transitively read it — this is the [[State of the Art - Incremental Computation|Salsa/Adapton]] model, except that the dependency graph does not have to be *tracked at runtime* because it is already the thing being stored.

Formally, a compiler becomes a function

$$ "build" : (h_"root", h_"rules", h_"target") → h_"artifact" $$

that is pure and total, which is the same property Nix gets for packages and Unison gets for definitions, at statement granularity.

## Why not just do this in memory?

The honest answer is that for a single build, you should. The database earns its place only when:

- the same definitions are compiled repeatedly across processes, machines or users (shared cache),
- you want to *query* the IR rather than regenerate it (tooling, search, analysis),
- you want to attach knowledge to intermediate nodes — tests, proofs, benchmarks, docs — that would otherwise have nowhere to live,
- multiple frontends must meet at a common representation.

If none of those apply, this is a slower LLVM. That trade-off is tracked in [[Open Problems and Risks]].

## Query cost is the real risk

Reconstructing a function's IR means a recursive traversal of possibly $10^4$–$10^6$ nodes. In a columnar store this is a recursive CTE with a hash join per level — fine at $10^4$, questionable at $10^6$ per function. Mitigations, in the order they should be tried:

1. **Chunked storage**: store whole subtrees as a single serialised blob keyed by the root hash, with individual node rows materialised only for nodes that are independently referenced. This is exactly what Git does with packfiles and what makes it fast.
2. **Materialised closures**: precompute and store `DEPENDS_ON*`.
3. **Hot-cache in memory**: [[Hash Consing|hash-consed]] arena in the process, database as the cold tier.
4. **Do not persist every IR layer by default**: treat `Op` and `Instr` layers as regenerable caches with an eviction policy, and persist only core terms as the source of truth.

(4) is probably the correct default and is a meaningful retreat from the most ambitious reading of the idea. It preserves everything that matters — core terms, types, equivalences, provenance — while letting the IR layers be a cache.

## Related

- [[Equality Saturation]]
- [[E-Graph]]
- [[Datalog]]
- [[Query Cookbook]]
- [[Graph Schema]]
- [[Content-Addressed Precompilation]]
- [[State of the Art - Incremental Computation]]
- [[State of the Art - Equality Saturation and E-Graphs]]
- [[egraph]] and [[sophia_emit]] — the Rust modules that would implement this
