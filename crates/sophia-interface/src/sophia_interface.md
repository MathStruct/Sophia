#implementation

> Crate root of `sophia-interface`: what Moonkale's `sources/sophia` adapter wraps. Source: `sophia_interface.rs`.
>
> Sources: [[SPEC-004 Moonkale Interface]]; Moonkale `packages/core/src/source/`. code: `sophia_interface.rs`, [[edit]], [[profiles]]

## Shape

`Interface<B>` is a store opened for **one reader**; the reader id is given at `open` (SPEC-004 §3a). Its public types are:
- `Key`: `Content(digest)` → Moonkale's `NodeId::from_content`; `Derived(s)` → `NodeId::derive(source, s)`;
- `NodeView`, `EdgeView`, `Subgraph`;
- `Direction`, `Change`, `Risk`;
- `Written`, `EditError`, and `Diagnostic` with byte spans.

None of them is a Moonkale, driver or async type.

## Operations, mapped to Moonkale's `Source`

| Moonkale | Interface |
|---|---|
| `Query::Node` | `node(key)` |
| `Query::Children` | `children(key)`: sub-namespaces and names; the root also holds `sophia:Settings` |
| `Query::Neighbours` | `neighbours(key, depth, dir, limit)` |
| `Query::All` | `all(limit)`: namespaces, names, definitions and named nominal entities, with `Contains`, `Names` and the dependency edges among them; `truncated` when the cap cuts |
| `Query::Text { "sql" }` | `sql(query, limits)` (SQLite): table plus subgraph |
| `fetch_text` | `fetch_text(key)`: names print their whole recursive group in the reader's profile; definitions print read-only; settings print the reader's profile |
| `apply(WriteText)` | `write_text(key, expected, text)` |
| `apply(CreateText)` | `create_text(ns, name, text)` |
| `apply(CreateDir)` | `create_dir(ns, name)` |
| `apply(Rename)` | `rename(key, to)` (`/` ↔ `.`) |
| `apply(Delete)` | `delete(key)` |
| `changes_since` | `changes_since(seq, limit)`, with the keys affected and `reprint_all` for profile changes |
| `classify` | `Interface::classify(dialect)` |

## Edit tokens

`fetch_text` returns, as a name's version, the edit token from `text_token(path)`: a hash of every name the text covers, with versions and targets (SPEC-004 §3b). `write_text` requires the same token. The profile is deliberately not part of it, because the parser reads every editable profile.

## Tests

`tests/acceptance.rs` covers SPEC-004 §9 item by item, on SQLite and redb (11 tests):
- browsing, including relations as nominal entities with `MethodOf` members;
- editing, including a no-op re-save, a parse error, a conflict, and an edit inside a recursive group;
- a rename in the change feed;
- switching profiles (reprint, edit in Rust, another reader still sees Julia);
- creating definitions, a relation and a rule from text;
- `sql`.

## Difficulties met

- **A deadlock:** `neighbours` called `children` inside an open read, and SQLite's connection mutex is not reentrant. Fixed with `children_in(tx, …)`. Rule kept for this crate: never call a public method from inside a `store.read` closure.
- **Not atomic across steps:** an edit is several store transactions (add records, update names, attach relation members). A failure halfway leaves a consistent but partial state, never a corrupt one. A single-transaction batch API is a later improvement ([[SP-008 Edits span several transactions]]).

**The `query` dialect** (2026-10-20, [[SPEC-007 Sophia Query Language]] §8): `query(text, limits)` runs Sophia's own query language on any backend and returns the table (ids as hex text) plus a node for every id cell. `classify("query")` is `ReadOnly`. A stored query's name prints in the query syntax (`print_target`).
