#implementation

> A 256-bit BLAKE3 digest. Source: `digest.rs`.
>
> Sources: O'Connor et al., *BLAKE3*, 2020; [[Block Size and Hash Overhead]] §4 (why 256 bits). code: `digest.rs`

`Digest` is the id type for definitions, components, nominal entities, annotations and facts.
- `to_hex` and `FromStr` round-trip, accepting an optional `#` prefix.
- `short(n)` gives display prefixes, the way git and Unison show hashes.
- Debug prints `#` plus 12 hex characters.

**Not done:** base32 display (Unison uses base32hex); short-prefix *input* is resolved by the CLI, not here.
