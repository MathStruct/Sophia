#implementation

> Rendering profiles as values, and their text form. Source: `profile.rs`.
>
> Sources: [[SPEC-005 Rendering Profiles]] §2, §3. code: `profile.rs`

- **`Profile`** is built from settings rows. Unknown or invalid values fall back to Julia's defaults. Built-ins: `julia`, `rust`, `python` (display-only), `core` (debug view).
- **`parse_settings` and `print_settings`** are the text form shown in Moonkale's `sophia:Settings` node: `key = value` lines.

The rows themselves live in the database: [[profiles]].
