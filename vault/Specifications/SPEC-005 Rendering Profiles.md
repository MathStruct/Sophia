#design #definition

> **Draft, 2026-10-20 (Claude), from Daniel's message of 2026-10-20.** How code in the store is displayed. Display depends **entirely** on the reader's declared settings, and those settings are **stored in the Sophia database**, not in JSON (SR-28). The default is a Julia-flavoured profile (SR-27). It replaces §6 of the SPEC-001 of 2026-10-17, and takes up Moonkale's North Star §11 ("syntax is a user setting") and WP-30.
>
> Sources: Moonkale `North Star.md` §11 (declarative rendering settings); projectional editors (JetBrains MPS, Hazel, Lamdu); bidirectional transformations (Foster et al., *Combinators for Bidirectional Tree Transformations*, TOPLAS 2007). code: `crates/sophialang-syntax`, `crates/sophia-store` (profile tables) (planned).
>
> Theory (CT-ML wiki): [Relational Lens](https://mathstruct.org/CategoryTheory-ML-Wiki/Relational-Lens). This vault: [[Theory of the v0 Store]] §4.

## 1. What a profile is

A **rendering profile** is a named set of settings that turns stored definitions into text and text back into definitions. It is a *view*:
- changing a profile changes no definition, no id and no history;
- two readers with different profiles edit the same definitions.

## 2. Stored in the database

```sql
profile(id TEXT PRIMARY KEY, name TEXT, lang TEXT, based_on TEXT, seq INT)       -- lang: the language it renders
profile_setting(profile TEXT, key TEXT, value TEXT, PRIMARY KEY(profile, key))
reader(id TEXT PRIMARY KEY, display_name TEXT)
reader_profile(reader TEXT, scope TEXT, profile TEXT, PRIMARY KEY(reader, scope))
  -- scope: 'default' | 'origin:<lang>' (code that came from that language) | 'namespace:<path>'
```

- **Resolution.** To print definition *d* for reader *r*, take the most specific `reader_profile` row: `namespace:` (longest prefix) beats `origin:`, which beats `default`. A profile inherits every key it does not set from `based_on`, up to a built-in profile.
- **Built-in profiles** are rows written at `init`: `julia` (the default for sophialang), `rust`, `python`, and `core` (a debugging view of the core term itself). Users copy one and change keys.
- **Changes are logged** like names (SPEC-001 §7), so a profile change reaches Moonkale through `changes_since`.
- **Where the rows live.** The project store holds the readers who work on it. A user-level store holding a person's profiles across projects is a later option (SPEC-001 Q7).
- **No file is read.** The only text form of a profile is the `sophia:Settings` view ([[SPEC-004 Moonkale Interface]] §4), and it is parsed back into rows.

## 3. The settings (v0 keys)

| Key | Values | `julia` | `rust` | `python` |
|---|---|---|---|---|
| `kw.function` | `function`, `fn`, `def` | `function` | `fn` | `def` |
| `kw.let` | `none`, `let`, `local` | `none` | `let` | `none` |
| `blocks` | `end`, `braces`, `indent` | `end` | `braces` | `indent` |
| `index.base` | `0`, `1` | `1` | `0` | `0` |
| `type.annotation` | `::`, `:` (postfix), `prefix` (C-style) | `::` | `:` | `:` |
| `type.show` | `declared`, `inferred-inline`, `hidden` | `declared` | `declared` | `declared` |
| `operators` | `unicode`, `ascii` | `unicode` | `ascii` | `ascii` |
| `implicit.show` | `hidden`, `shown` | `hidden` | `hidden` | `hidden` |
| `generic.call` | `f(x, y)`, `x.f(y)` (display a method call receiver-first) | `f(x, y)` | `x.f(y)` | `x.f(y)` |
| `comments` | `#`, `//` | `#` | `//` | `#` |
| `unsafe.show` | `keyword`, `gutter`, `hidden` | `keyword` | `keyword` | `keyword` |
| `proofs.show` | `inline`, `gutter`, `hidden` | `gutter` | `gutter` | `gutter` |
| `names.locale` | a locale, for localised names later | `en` | `en` | `en` |

The `rust` and `python` profiles render **sophialang** in Rust or Python dress. They do not make it Rust or Python: semantics stay sophialang's, and they follow Julia when in doubt (SR-29). Code that *came from* Rust keeps the language tag `rust`, and is rendered by the Rust frontend's own printer.

## 4. Index base, worked out

The hard case from North Star §11. Printing `a[0]` as `a[1]` is right only where the core *knows* the expression is a position in a sequence. So:
- Positions have their own type, **`Index`**, separate from `Int64`.
- An array access takes an `Index`. Arithmetic `Index ± Int64 → Index` is base-independent: "the next element".
- A literal position is stored as an **offset from the first element** (`Index(0)` is the first) and printed as `offset + base`.
- Conversions between `Int64` and `Index` are explicit calls, `index(k; base=…)` and `ordinal(i; base=…)`. They print honestly, with their base.
- An `Int64` used as a position without a conversion is a type error in the checked fragment. In unchecked code it is displayed with a "base unknown" marker and never shifted.

The cost is a distinct type for positions. Julia already has `CartesianIndex` and Rust has `usize`, so the idea is not alien, but it is a real design decision ([[SP-007 Index base needs an Index type]]). v0 has no arrays, so v0 only reserves the key and the type.

## 5. The rules every editable profile obeys

1. **A view, never a rewrite.** No setting changes a stored definition.
2. **Round trip.** For every definition *d* and editable profile *p*: `parse(print(d, p), p) = d`. A property test runs it over every stored definition and every built-in profile.
3. **Semantic, not textual.** Sugar is chosen per typed core node (§4), never by string replacement.
4. **Lossy means read-only.** A setting that loses information (`type.show = hidden` on a definition whose type cannot be re-inferred) makes the affected part display-only and marked.
5. **Stated profile.** Shared artefacts (diffs, copies, published pages) say which profile printed them.

## 6. The parser

`parse(text, projection, namespace) → Result<[Core definition], [Diagnostic]>`.
- Names resolve against the current namespace. A name that fails becomes `Unresolved` with a warning, not an error, so you can save a definition before its dependencies exist (§4.4).
- Syntax errors are diagnostics with ranges, and nothing is stored.
- **The parser is bounded** (Moonkale Security.md, "bounded parsers"). It has an explicit limit on input size (default 512 KiB per text), nesting depth (default 128) and the number of definitions per text (default 4,096), matching Moonkale's WP-66 limits. Exceeding a limit is a diagnostic, never a crash or a partial store.
- Nothing in the core records which projection wrote it. A definition parsed from a Rust-flavoured projection and one parsed from a Julia-flavoured projection are the same definition if they elaborate to the same term.



## 7. The canonical `julia` profile

```julia
function add(x::Int64, y::Int64)::Int64
    x + y
end

area(c::Circle)::Float64 = π * c.r^2          # short method form

data Vec(A::Type, n::Nat)
    nil :: Vec(A, zero)
    cons(x::A, xs::Vec(A, n)) :: Vec(A, succ(n))
end

relation parent(::Person, ::Person)
rule ancestor(X, Z) :- parent(X, Y), ancestor(Y, Z)
```

The same `add` in the `rust` profile is `fn add(x: Int64, y: Int64) -> Int64 { x + y }`, and in the `python` profile it is `def add(x: Int64, y: Int64) -> Int64:` followed by an indented `return x + y`. In the `python` profile a final expression prints as `return`, which is printing sugar and parses back to the same term.

## Implementation status (2026-10-20)

- **Profiles:** `julia` (default) and `rust` are editable and round-trip ([[sophialang_syntax]]: tested on the sample and on 20 000 random definitions). `python` is display-only, because its indentation blocks have no parser yet. `core` is a debug view.
- **Storage:** profiles are stored as meta rows in the database (`profile/<name>/<key>`, `reader/<reader>/<scope>`), not in dedicated tables ([[profiles]]), so every backend has them without a schema change. Resolution follows §2.
- **Keys honoured:** `kw.function`, `blocks`, `type.annotation`, `operators`.
- **Not yet:** `type.show`, `implicit.show`, `generic.call`, `index.base`, and the `Index` type of §4 (no arrays yet).
- **`let` and `match`** follow the profile's `blocks` (2026-10-21): `end` in `julia`, braces in `rust` (`let x = v {` … `}`, `match s {` … `}`), a colon and indentation in `python`. Until then they printed with `end` in every profile, so Rust text mixed both styles.

## Related

[[SPEC-002 Sophialang Principles]] · [[SPEC-003 Sophialang Core v0]] · [[SPEC-004 Moonkale Interface]] · [[LLMs and Mixed-Language Code]]
