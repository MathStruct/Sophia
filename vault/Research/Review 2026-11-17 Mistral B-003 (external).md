#reference

> A correctness review of B-003 (`Interface::propose_text`, [[SPEC-010 Store-Truth Folders and Generated Files]] §5 and acceptance 3; named relation signatures as export aliases, §4) by Mistral Large 4 through the `vibe` CLI, 2026-11-17, in the helper worktree `../Sophia-mistral` (branch `mistral/b003-review`). Codex's quota was used up. Mistral could not run cargo in its session, so it reviewed by reading and wrote 13 probe tests without running them. Claude ran them. Two failed because of mistakes in the probes, which Claude corrected. After that, all pass. One finding is a real limitation, already documented. One is recorded as not reproduced. The probes are now `crates/sophia-interface/tests/propose_text.rs`.
>
> Sources: the review's answer file, kept outside the repository at `/home/daniel/.cache/agent-usage/vibe/2026-11-17-2351-b003-review/ANSWER.md` (cost: $0, Mistral Large 4). code: `propose.rs`, `edit.rs`, `sophia_interface.rs`, `tests/acceptance.rs`, `tests/propose_text.rs`.

## How it was run

```sh
vibe -p --prompt-file review-brief.md --agent review --trust --workdir ../Sophia-mistral --max-price 1 --max-turns 100
```

The brief gave the requirement (SPEC-010 §4, §5, acceptance 3) and the diff, and asked for defects only, with probes as tests. The probes were run with `cargo test -p sophia-interface --test probe_b003`. After Claude's two corrections, the result was 14 passed, 0 failed.

## Findings

| # | finding | right? | what was done |
|---|---|---|---|
| F1 | `propose_text` reads the export at one log position, but then diffs through many separate reads, so a concurrent write could give a proposal that mixes two store states and is not reported as a conflict | plausible, **not reproduced** (needs a concurrent writer; no test made it fail) | not changed (rule: no fix without a failing test). [[propose]] states that the proposal takes no lock. A cheap guard would be to retry while `last_seq` moved during the diff, as `export` does. It is noted for B-060, whose accept has to check the token in its own transaction anyway |
| F2 | a new relation's id in the proposal, and the ids of records referring to it, are stand-ins: `new_nominal` draws a random id | **right**, confirmed by the probe `new_relation_reports_provisional_ids` | already documented in [[propose]] and SPEC-010 §6.6; the probe stays as a test |
| F3 | `propose.md` missing, Code Map entry missing, `sophia_interface.md` says "not built yet" | partly: the diff was taken before the notes were written. The stale "not built yet" line was **right** | line removed |
| F4 | a name outside the file is reported as `NameClaimed` with an empty owner, while an edit raises `EditError::Refused` | not a defect: a `Proposal` carries a `StoreError`, and `NameClaimed`'s documented meaning includes "empty when no file owns it" | none |
| F5 | no accept for a whole sophialang file | **right**, a spec gap rather than a defect of this change | backlog row B-060 |

**Probe mistakes corrected by Claude.** (1) `reorder_only_changes_no_name` joined every span. A recursive group's names share one span, so the group appeared twice, and the parser rightly refused `odd` defined twice. Duplicate spans are now dropped. (2) `new_relation_reports_provisional_ids` took the first membership of `sample.likes` as the signature, but that was the rule. It now looks for the signature among the stored relation's methods.

## What the probes cover

They run on SQLite: the round trip, which writes nothing; empty text, where every name is removed; a name defined twice, which is a parse error; a name outside the file; a name from another namespace; a namespace that does not exist; a stale token, which conflicts but still diffs; reordering, which changes no name; a recursive group and its alias; rules and signatures as memberships; a new relation; a laid-out name; a derived store; and a conflict reported together with a refusal.
