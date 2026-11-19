#implementation #experiment

> E-2 (B-023): do two gemm implementations share structure a query can find without names? Ingests Rust `matrixmultiply`, Octavian.jl and LinearAlgebra's `matmul.jl` into one SQLite store, computes name-free loop features for every definition, and scores two structural predicates against hand-made role labels, on the source and in the store. Source: `gemm_structure.rs`; labels: `gemm_structure.tsv`. Result: [[Cross-Language Gemm Structure]].
>
> Sources: [[Semantic Boundary Experiments]] §E-2; code: `gemm_structure.rs`, `gemm_structure.tsv`, `../src/code.rs`, `../src/module.rs`.

**Run.**

```sh
E=$SOPHIA_CORPORA/e2   # matrixmultiply-0.3.10 (crates.io), Octavian.jl 83f70f1b, Julia 1.13.1's matmul.jl
cargo run --release -p sophia-ingest --example gemm_structure -- $E/out \
    crates/sophia-ingest/examples/gemm_structure.tsv \
    rust:matrixmultiply:$E/dl/matrixmultiply-0.3.10/src \
    julia:Octavian:$E/dl/Octavian.jl/src/Octavian.jl \
    julia:matmul:$E/jlmatmul/matmul.jl
```

A corpus is `lang:name:path`: a Rust source directory (every `.rs` below it, per file, as `sophia ingest-code rust` does), or a Julia root file (module mode with its includes, as `sophia ingest-code julia` does). It writes `OUT/e2.sqlite` (fresh each run) and `OUT/definitions.tsv` (one row per definition: status, depth, loops, macro loops, roles, calls) and prints the tables.

**What it computes, per definition found by `code::definitions` (refused or not):**
- **status:** `ok` with the record id the store gives it, or the refusal reason (`code::file`'s first refused construct; `no id in module mode` for a Julia definition the per-file finder sees but module mode does not identify: mostly assignments module mode does not count as definitions);
- **depth:** the deepest loop nest in its syntax tree. Rust `for`, `while`, `loop`; Julia `while`, and `for` with one level per binding (`for i in a, j in b` is 2). Loops inside a macro call's arguments count when the parser parses them (Julia's `@turbo for …`), not when they are a token tree (Rust's `loop4!(…)`);
- **macro loops:** `for`/`while`/`loop` keywords inside Rust token trees, the loops the tree does not show;
- **calls:** the last name of each callee (`a.b.f(x)` → `f`, `K::kernel` → `kernel`) and Rust macro calls (`loop4!`).

**The predicates** (module containers, `module` and `mod`, are not candidates: their loops are their contents'):
- **nest-over-nest:** depth ≥ 2, and a definition reachable through calls has a loop: the BLIS driver, blocking loops over a callee that loops;
- **leaf-nest:** depth ≥ 2, and nothing reachable has a loop: kernels, packing, naive loops.

Each runs twice. **On the source**, calls resolve by name within the corpus, to every definition of that name (all methods, all trait impls). **In the store**, the program records facts `e2_depth_ge1` to `e2_depth_ge4` on each stored definition (kind = the bucket, subject = the record, input = the record), and runs the predicate as a Sophia query, calls being `refs(a, b)` or `uses(a, n) && member(n, b)`:

```julia
query e2_nest_over_nest()
    calls(a, b) = refs(a, b)
    calls(a, b) = uses(a, n) && member(n, b)
    reach(a, b) = calls(a, b)
    reach(a, c) = calls(a, b) && reach(b, c)
    below(a) = reach(a, b) && fact(_, :e2_depth_ge1, b)
    select a where fact(_, :e2_depth_ge2, a) && below(a)
end
```

`e2_leaf_nest` is the same with `!below(a)`. Depth is bucketed into fact kinds because a query cannot compare numbers yet (SPEC-007 §13, B-067).

**The labels** are the oracle: roles (driver, pack, macrokernel, microkernel, naive) given by hand from the sources before any predicate ran; `gemm_structure.tsv` says what each role means. A label that matches no definition stops the run.

**Where the store and the source could differ** although the store holds the definitions (Mistral's review of B-023, F4 to F6; not triggered by E-2's corpora): a call between members of one recursive component is in no base relation, so `below` can be false in the store; a Julia file included into two modules is read twice and both rows get the last module's id; a label names a file by basename, so it would match every file of that name (matrixmultiply has two `mod.rs` and two `macros.rs`; no label names them). Containers get no facts, so the store query excludes them as the source predicate does.

**What it does not do.** It does not resolve Rust paths, traits or macros (calls by name over-approximate), does not expand macros, and has no data flow, so it cannot tell a copy loop from a multiply-accumulate loop. The features are an experiment's, recorded as facts by `e2-gemm-structure@0`; nothing in Sophia's ingest computes them.
