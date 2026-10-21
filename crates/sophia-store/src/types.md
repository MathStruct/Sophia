#implementation

> The store's record types. Source: `types.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]] §5 to §8. code: `types.rs`

The types are:
- `Target` (content or nominal);
- `NameEntry`;
- `LogOp` and `LogEntry`;
- `Nominal` (with a relation `version`);
- `RelationEntry`;
- `LineageEntry`;
- `Annotation` (its id hashes kind, subjects, payload and checker; status and inputs can change);
- `Input` (content, or a relation at a version);
- `Fact` (its id hashes everything, including its inputs);
- `SourceSpan` and `SOURCE_SPAN` (2026-10-21): the value of a `source-span` fact, which places a definition in a stored source record (source id, byte start, byte end). Ingests of Julia record one per definition, so a reader can show the text a definition was read from when its bytes are a foreign normal form (WP-66) that does not print back. A fact, not part of the record: it is store state, not identity;
- `Changes`.
