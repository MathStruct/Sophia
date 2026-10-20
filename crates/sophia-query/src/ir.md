#implementation

> The query IR: sorts, values, terms, the base relations, atoms, literals, rules, the select, and `Names` (display data). Source: `ir.rs`.
>
> Sources: [[SPEC-007 Sophia Query Language]] §2–§3. code: `ir.rs`

- `Base` lists the eight base relations (`def`, `refs`, `name`, `member`, `nominal`, `uses`, `annotation`, `fact`) with their column sorts. `Base::SCHEMA` (`store-base-v0`) is hashed into every query id.
- A `Query` is parameters (sorts), local predicates (arities), `Rule`s, and one `Select` whose outputs are variables or `count(v)`.
- Variables are numbered per rule; names live in `Names`, never in the IR, exactly as sophialang keeps names out of hashed terms.
