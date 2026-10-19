#implementation

> Rendering profiles stored in the database. Source: `profiles.rs`.
>
> Sources: [[SPEC-005 Rendering Profiles]] §2; SR-28 ("store in database not in .json"). code: `profiles.rs`

Rows of the store's meta table:
- `profile/<name>/<setting>`, with `based_on` for inheritance;
- `reader/<reader>/<scope>`, where the scope is `default`, `origin:<lang>` or `namespace:<path>`.

The most specific scope wins: the longest namespace prefix, then the language of origin, then the default, then `julia`. Built-in profiles are written at `open` if missing. Every change goes through `Store::set_settings`, one logged transaction.

**Difference from SPEC-005 §2:** the rows live in the generic meta table rather than dedicated tables, so that every backend has them with no schema change. A query view over them can be added for SQLite.
