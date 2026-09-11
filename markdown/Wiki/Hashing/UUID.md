# UUID

A UUID (RFC 4122 / RFC 9562) is a 128-bit identifier with a version field indicating how it was produced. The versions relevant to a content-addressed system:

- **v4** — random. Unique with overwhelming probability, but says nothing about what it identifies. Julia's `Project.toml` package UUIDs are these.
- **v5** — SHA-1 of a namespace UUID plus a name. Deterministic and reproducible, which is the right shape, but only 122 usable bits and built on SHA-1.
- **v7** — time-ordered random. Good for database keys, irrelevant here.

[[Start Here]] uses "UUID" for what this vault calls a **content hash**, and the distinction is worth keeping. A 256-bit [[BLAKE3]] digest is what Sophia actually stores; a UUID-shaped identifier is derivable from it by truncation for systems that insist on a 128-bit key.

That truncation costs real security margin: collision probability rises from roughly $n^2 slash 2^257$ to $n^2 slash 2^129$, which at $n = 2^40$ is about $2^(-49)$ instead of $2^(-177)$. Acceptable for a display label or a foreign key; not acceptable as the thing that decides whether two definitions are the same program. Hence: **full digest for identity, truncated UUID for interop only.**

## Related

- [[BLAKE3]]
- [[Hashing and Identity]]
- [[Content-Addressed Code]]
