#design #open-problem

> Language models are trained on existing languages, not on sophialang's canonical form. So models reach the store through a **frontend**: a projection in a language they know. Daniel asked whether a model can handle the **mixed-language content** this yields (SR-62). **Answer: yes, if each definition is shown in a surface the model knows, if the cross-language edges are explicit, and if every write goes through a parser that refuses what it cannot normalise.** This note says how, what the risks are, and how training on sophialang directly would work later.
>
> Sources: Moonkale `markdown/architecture/LLM and RAG.md`, `Claude Code Extension.md`; the Model Context Protocol; Chen et al., *Evaluating Large Language Models Trained on Code*, 2021 (models are strongest in high-resource languages); [[Cross-Language Semantic Hazards]]. code: `crates/sophia-mcp` (planned, M3).

## 1. How a model sees the store

- **Through a rendering profile** ([[SPEC-005 Rendering Profiles]]). Sophialang printed in the `julia` profile is close to a language models know well. A model may also be given the `rust` or `python` profile if that makes it more accurate for a task. The store does not care, because every profile parses back to the same core.
- **Definitions of other languages appear in their own syntax**, printed by their own frontends. A Rust function stays Rust.
- **Cross-language relations are explicit text, never implied:**
  - a call into another language is shown as a call to a name with its language tag;
  - an equivalence is shown as an annotation (`# ≡ rust:crate::f, proved by sophia-egraph`).

  The model never has to guess that two things "are the same".
- **Identity is never typed by the model.** Content ids appear only as short prefixes in comments. A model refers to names, and the parser resolves them.

## 2. The tool interface

A Model Context Protocol server, `sophia-mcp`, gives agents the same operations Moonkale has ([[SPEC-004 Moonkale Interface]]):
- `search(query)` and `sql(query)`, read-only;
- `show(name, profile)`: the printed text with its source map;
- `context(name, depth)`: the definition plus its dependencies and dependants, each in its own language;
- `edit(name, text, expected_version)`: parse, store and move the name, or return diagnostics;
- `add(namespace, text)`;
- `facts(name)`: typings, test results, proofs, `unsafe` sections.

Every write goes through the parser and the optimistic version check, so a model cannot corrupt the store. At worst it adds definitions nobody names.

## 3. Mixed content: what is hard

- **Semantic drift between surfaces.** In the `python` profile, sophialang integers still wrap like Julia's (SR-29), not like Python's unbounded integers. A model thinking in Python may assume otherwise. *Mitigation:* the profile header states the semantics ("sophialang semantics, Python dress"), and tests catch the rest.
- **Cross-language calls.** A model editing Julia code that calls a Rust definition needs the Rust signature in context. `context()` includes it in Rust syntax.
- **Index base.** In the `python` profile, positions print 0-based. A model's off-by-one habits follow the dress, which is the point of SPEC-005 §4. The `Index` type makes a mismatch a type error rather than a silent bug.
- **Token budget.** Mixed context costs more tokens. `context(depth)` lets agents fetch exactly what they need, because the closure is exact (Minimal Binaries §1).

## 4. Training on sophialang directly (later)

Once existing languages are parsed into the store (SR-03), it contains **parallel corpora**:
- the same definition printed in several profiles;
- for ingested code, the original source paired with the normalised core.

That supports:
1. **Profile-agnostic training:** the model learns that the `julia`, `rust` and `python` prints are one program.
2. **Training on the core directly**, with a tree vocabulary from mined abstractions ([[Compression and Canonical Form]] §3.5), so the model writes core terms and reuses abstractions.
3. **Cross-language translation**, verified: a model proposes a translation, and the store checks it by tests or by an equivalence proof before it is accepted.

The open question (SR-73) is whether a model writing core terms is more reliable than one writing a familiar surface that a parser checks. That is an experiment to run, not an assumption.

## Related

[[SPEC-005 Rendering Profiles]] · [[SPEC-004 Moonkale Interface]] · [[Compression and Canonical Form]] · [[Cross-Language Semantic Hazards]]
