#overview #planning

> The open tasks of Sophia, one note per section in this folder (indexed below), how a task is picked and closed, and where finished tasks go. One task per agent session; the handoff to the next session is the top section of [[Status]].

> Sources: original to this repository; [[Milestones]], [[Status]].

**Rows.** `| id | task | status | needs | size | source | done when |`. Status: *planned* (specified or asked for), *proposed* (an idea with a reason, not yet accepted by Daniel), *partial*, *done*. Size: **S** about one session, **M** two to four, **L** many (split before starting). Ids are `B-nnn`, never reused. Priorities are proposals until Daniel sets them.

**Picking a task.** Take the task named under "Next session" in [[Status]]; otherwise the first *planned* row, in the order of the section notes below, whose needs are done. A specification, requirement (SR-nn), problem (SP-nnn) or review that asks for work gets a row in the section note it belongs to.

**Closing a task.** When a task is finished: its gates pass; its row moves to `Backlog Done <yyyy-mm>.md` in this folder (one note per month) with the date, the commit and what was verified and how; the "Next session" section of [[Status]] is **overwritten** (date, what was finished, the next task's id, which notes and files it needs, what the next session must know); if the change reaches Moonkale, an entry goes to Moonkale's `HANDOFF-from-sophia.md` (AGENTS.md, "Moonkale"); then the agent reports to Daniel and suggests a fresh session.

**Unattended runs.** When a task's design is written in its row (files, acceptance, nothing left for Daniel to decide), the maintainer's session may queue it for Codex or Mistral: one line `ready  B-002  codex` (or `mistral`, `glm`) in the untracked `QUEUE.md` of the main checkout, then `dispatch-ready sophia` (`--dry-run` first; `--max N`, `--hours H`). The script runs the queued tasks one after another, each on a fresh branch in the agent's worktree, and leaves `HANDOFF-from-<agent>-<id>.md` and a summary in `HANDOFF-from-dispatch.md` in the main checkout. It never merges or edits Status or the backlog: the next Opus session reviews each branch (and has the other model review it), merges, and closes the row as above.

## Open

The order below is a proposal (2026-11-17, B-001), following SR-125 (Moonkale first) and the order of [[Milestones]]; Daniel sets the priorities. Rows marked *proposed* wait for his decision before work starts. The sections follow that order, one note each in this folder; a new row goes into the note it belongs to.

| note | section | open rows |
|---|---|---|
| [[Backlog Moonkale]] | Moonkale and the folder store (SR-125) | 3 |
| [[Backlog Fable Review]] | Open findings of Fable's review | 5 |
| [[Backlog Query Engine]] | Query engine and agents | 8 |
| [[Backlog Semantic Experiments]] | Semantic boundary experiments ([[Semantic Boundary Experiments]], SR-124, SR-139) | 12 |
| [[Backlog Storage]] | Storage (measure first, layout changes need Daniel, SR-126) | 4 |
| [[Backlog Sophialang]] | Language: sophialang (M4 to M6, SR-154) | 16 |
| [[Backlog Store Semantics]] | Store semantics, later | 5 |
| [[Backlog Vault]] | Vault and documentation | 3 |
| [[Backlog Long-term]] | Long-term directions (not planned; listed so they have a row) | 6 |
