#implementation #algorithm

> Turns a batch of drafts into hashed records. Source: `normalise.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]] §4; [[component]]; [[merkle]]. code: `normalise.rs`

1. Collect each definition's `Draft` references.
2. Find strongly connected components, dependency-first.
3. For each component:
   - order it canonically (references inside the group become canonical indices; references to earlier components become their finished digests);
   - compute the component digest and the member ids;
   - build the records (with `refs`, `nominal_refs` and status).

`LANG = "sophialang"` and `NORMALISER = "sophialang-core@0"` enter every id. Changing the encoding means bumping the normaliser's version (SPEC-001 §4.5).

**Difficulty:** a batch must contain every member of a recursive group. The bench generator keeps recursive pairs inside even-sized batches. A frontend that streams definitions has to buffer until each group is closed.
