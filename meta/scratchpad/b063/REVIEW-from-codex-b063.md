Stored-query name parameters reject valid nominal entities, and ordinary additions return misleading path or version information. These issues affect the newly exposed tools' normal workflows.

Full review comments:

- [P2] Resolve nominal names as valid Id parameters — /home/daniel/Code/CategoryTheory/Sophia/crates/sophia-mcp/src/sophia_mcp.rs:267-268
  When an Id parameter names a nominal entity, such as a sophialang relation, `run` rejects it even though `Sort::Id` explicitly includes nominal entities. The same query works when given that entity's hex digest, so the advertised name-path alternative is inconsistent. Convert `Target::Nominal(d)` to `Val::Id(d)` as well.

- [P2] Return a usable version token after adding a definition — /home/daniel/Code/CategoryTheory/Sophia/crates/sophia-mcp/src/sophia_mcp.rs:385-387
  For ordinary sophialang definitions, `Interface::create_text` returns `Written.version == 0` because its `apply` call has no edited name. Forwarding that value makes the successful `add` response contain an invalid version: passing it to `edit` immediately produces a conflict. Obtain the created name's `text_token` before returning the response, as the query-creation branch already does.

- [P2] Use the actual created key in the add response — /home/daniel/Code/CategoryTheory/Sophia/crates/sophia-mcp/src/sophia_mcp.rs:385-386
  If `name` is `foo` but the text defines `bar`, `create_text` successfully creates `bar` and returns its key; this wrapper discards that key and reports `namespace.foo` instead. The reported path may not exist or may identify an unrelated existing definition. Either validate that the text defines the requested name before writing or derive the response path from the returned key.