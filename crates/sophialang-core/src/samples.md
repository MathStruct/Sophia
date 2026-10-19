#implementation

> The sample program: 16 definitions covering every kind. It is used by the tests, the conformance suite, `sophia sample` and the Moonkale sample store (SPEC-004 §9). Source: `samples.rs`.
>
> Sources: [[SPEC-003 Sophialang Core v0]] §6. code: `samples.rs`

The sample contains:
- `Nat` (data);
- `add` (self-recursive);
- `even` and `odd` (mutually recursive);
- `todo` (a hole);
- `norm` (an unresolved `math.sqrt`);
- the relations `parent` and `ancestor`, with two rules (one weighted);
- a test;
- `double` on `Int64`, dispatching through the built-in generic `+`;
- `peek` (an `Unsafe(memory)` section);
- a postulate;
- a struct layout;
- a constant.

`builtin(name)` derives intrinsic nominal ids for built-ins (`Int64`, `+`, `==`, …), as Unison does with `##Nat`.
