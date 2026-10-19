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
- `Changes`.
