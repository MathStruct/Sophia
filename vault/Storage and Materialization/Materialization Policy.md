#design #decision-input

> When Sophia keeps an object's bytes. Default rules per category now; a cost model later, once workloads have been measured. **No automatic caching system is to be built yet** (SR-126). Part of [[Storage and Materialization]]. 2026-10-20.
>
> Sources: [[Object Categories]]; the external review of 2026-10-20 (the inequality below is its proposal); Harinarayan, Rajaraman and Ullman, *Implementing Data Cubes Efficiently*, SIGMOD 1996 (greedy view selection under a space budget); Megiddo and Modha, *ARC: A Self-Tuning, Low Overhead Replacement Cache*, FAST 2003.

## Hard rules (never overridden by a cost estimate)

1. **Authored content is persisted.** It cannot be rebuilt (category 1).
2. **Decisions are persisted.** Names, nominal collections, lineage and the log (category 3).
3. **Non-reproducible evidence that something cites is persisted**: a test result a release relied on, a measurement a decision quotes.
4. **Every reported or cited result has a provenance record**, whether or not its bytes are kept.
5. **Identity is never a function of the policy.**
6. **Transient working sets are budgeted and released** (SR-120). A computation may peak high; it may not leave residue.

## Default rules per category

| Category | Default |
|---|---|
| 1 authored | persist (source blobs content-addressed and compressed) |
| 2 canonical forms | persist id, kind, edges; **bytes cached** for ingested code, persisted for sophialang-authored code |
| 3 decisions | persist; compact the log |
| 4 evidence | persist certificates and cited results; keep provenance of the rest |
| 5 derived structures | indexes persisted as physical layout; results cached; the rest on demand |
| 6 compiled artifacts | cached by A; persisted when pinned or when expensive and reused |
| 7 intermediates | transient; promoted only explicitly |

## The cost model, for later

For a derived object x:
- C(x): expected cost to recompute it;
- R(x): cost to retrieve a stored copy;
- F(x): expected number of future uses within the retention horizon;
- S(x): storage cost (bytes × time);
- I(x): cost of keeping it valid (for snapshot-keyed objects this is zero: they never become wrong, only unreachable).

Keep x when

```text
F(x) · (C(x) − R(x))  >  λ · S(x) + I(x)
```

λ is the exchange rate between storage and compute, one knob per store. The review proposed this form; it is the standard view-selection benefit per unit space.

**Why not now.** F is unknown until real workloads run (Moonkale browsing, agent queries, builds). Guessing F builds a complicated predictor on no data. The order is:
1. measure (log every derived-object request with its cost and size, [[Storage Measurements]] M-3);
2. apply the default rules above;
3. fit λ and an F estimator from the log;
4. only then, if the log shows it pays, an adaptive cache (ARC or a cost-aware LRU variant such as GreedyDual-Size).

## Situations the defaults must handle

- **Cheap to rebuild, used constantly:** reverse edges. Persist as an index; their cost is storage, which SP-011 reduces.
- **Expensive, used rarely:** the closure of everything below `fta`. Cache with a short horizon.
- **Expensive and reused:** a compiled derivative of a large model; a Reactant/XLA executable. Persist by artifact id A. This is the case that justifies artifact caching at all ([[AD as a Materialization Case]]).
- **Cheap, used once:** most query results. Do not keep; their provenance is the query text and the snapshot, both already stored.
- **Non-reproducible but cheap to repeat:** benchmark timings. Keep the summary a decision cites; drop the raw samples.

## What the policy is not

- It is not part of the store's semantics. Two stores with different policies answer every query identically; one is faster or smaller.
- It is not per-backend. LMDB and SQLite hold the same logical objects ([[S-ADR-0010 Primary backend and an analytics replica]]).
