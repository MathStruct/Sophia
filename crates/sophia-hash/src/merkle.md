#implementation

> Component, member and nominal digests, and `Record`: the language-agnostic row the store keeps for one hashed definition. Source: `merkle.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]] §4.1, §4.3. code: `merkle.rs`
>
> Theory: [[Theory of the v0 Store]] §1 (the Merkle DAG).

```
component_digest = BLAKE3(field("sophia","store-v0") ‖ field("lang",…) ‖ field("normaliser",…) ‖ field("component", members…))
member_digest    = BLAKE3(field("component", component_digest) ‖ field("index", u32))
nominal_id       = BLAKE3(field("sophia","nominal-v0") ‖ lang ‖ kind ‖ field("nonce", 32 bytes))
```

- **`Record`** carries the id, the language and its normaliser, the kind, the component and index, the canonical bytes, the display data (not hashed), the status, and the outgoing references (content and nominal).
- **`Record::verify`** recomputes the ids of a component's members from their stored bytes. The store is not trusted: `sophia verify` rehashes everything.

**What it does not do:** check that `refs` agrees with the references inside `bytes`. Only the language can decode the bytes, so that check belongs to the language's tests.
