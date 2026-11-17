#overview #planning

> The open tasks of Sophia in one list, how a task is picked and closed, and where finished tasks go. One task per agent session; the handoff to the next session is the top section of [[Status]].

> Sources: original to this repository; [[Milestones]], [[Status]].

**Rows.** `| id | task | status | needs | size | source | done when |`. Status: *planned* (specified or asked for), *proposed* (an idea with a reason, not yet accepted by Daniel), *partial*, *done*. Size: **S** about one session, **M** two to four, **L** many (split before starting). Ids are `B-nnn`, never reused. Priorities are proposals until Daniel sets them.

**Picking a task.** Take the task named under "Next session" in [[Status]]; otherwise the first *planned* row whose needs are done. A specification, requirement (SR-nn), problem (SP-nnn) or review that asks for work gets a row here.

**Closing a task.** When a task is finished: its gates pass; its row moves to `Backlog Done <yyyy-mm>.md` in this folder (one note per month) with the date, the commit and what was verified and how; the "Next session" section of [[Status]] is **overwritten** (date, what was finished, the next task's id, which notes and files it needs, what the next session must know); then the agent reports to Daniel and suggests a fresh session.

**Unattended runs.** When a task's design is written in its row (files, acceptance, nothing left for Daniel to decide), the maintainer's session may queue it for Codex or Mistral: one line `ready  B-002  codex` (or `mistral`, `glm`) in the untracked `QUEUE.md` of the main checkout, then `dispatch-ready sophia` (`--dry-run` first; `--max N`, `--hours H`). The script runs the queued tasks one after another, each on a fresh branch in the agent's worktree, and leaves `HANDOFF-from-<agent>-<id>.md` and a summary in `HANDOFF-from-dispatch.md` in the main checkout. It never merges or edits Status or the backlog: the next Opus session reviews each branch (and has the other model review it), merges, and closes the row as above.

## Open

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-001 | Fill this backlog: one row per open item of [[Milestones]] (M2 onwards), the "waiting" items of [[Status]], open SP-nnn problems and unanswered SR-nn requirements | planned | — | S | Daniel, 2026-11-17 | every open item has a row with its source; Status names the next task |
