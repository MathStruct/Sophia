#planning

> Open backlog tasks: the Moonkale edit loop and the folder store. Part of [[Backlog]], whose rules (rows, picking, closing) apply.

> Sources: [[Backlog]], [[Status]].

## Moonkale and the folder store (SR-125)

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-060 | `Interface::accept_text(namespace, text, token)`: apply a `propose_text` text in one transaction: names added, retargeted, renamed (same content) and **removed** (SR-155), memberships, the token checked inside; plus the proposal's guard against concurrent writes (Mistral's F1) | planned | Daniel: build together with B-010? (proposed: yes) | M | SPEC-010 §8; SR-155; [[Review 2026-11-17 Mistral B-003 (external)]] F1, F5 | SPEC-010 §8's acceptance 1 to 4 pass on four backends |
| B-005 | B-15, the rest of the edit loop: the change feed (Moonkale polls every 500 ms), the workspace's re-fetch, paint on Daniel's desktop | planned | Moonkale | M | SR-83, SR-125; [[Milestones]] order item 2 | a measured edit-to-paint latency in [[Benchmark Results]], with Moonkale's revision |
| B-006 | X-1, X-2: where the shared normaliser lives (its own repository or Sophia), and a registry of foreign normalisers instead of the `moonkale-wp66` prefix | proposed | Daniel, Moonkale | L | [[Review 2026-11-10 Fable (external)]] X-1, X-2, X-3 | a decision (S-ADR) Daniel and Moonkale accept; then rows for the move |
