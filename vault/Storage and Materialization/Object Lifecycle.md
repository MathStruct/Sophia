#design

> What happens to an object from the moment it is computed: discarded, cached or persisted, and how it moves between these. Part of [[Storage and Materialization]]. 2026-10-20.
>
> Sources: [[Object Categories]]; [[SPEC-006 Worlds, Artifacts and Evidence]] §3, §6a; Nix (store paths, roots, `nix-collect-garbage`); Bazel's action cache and content-addressed storage; Salsa (red-green revalidation).

## The states

```text
                    ┌─────────────── compute (operation, inputs, params, env) ───────────────┐
                    ▼                                                                        │
            ┌──────────────┐  release   ┌─────────┐                                          │
 transient  │ working set  │──────────▶ │  gone   │   (default for runtime intermediates)    │
            └──────┬───────┘            └─────────┘                                          │
                   │ result                                                                   │
                   ▼                                                                          │
            ┌──────────────┐  record provenance (always, for anything reported or cited)     │
            │   result     │──────────────────────────────────────────────┐                  │
            └──┬────┬────┬─┘                                              ▼                  │
     discard   │    │    │ persist                                 ┌──────────────┐          │
               ▼    │    ▼                                         │  provenance  │──────────┘
          ┌───────┐ │ ┌───────────┐  unpin / unreachable           │  record      │  rebuild on demand
          │ gone  │ │ │ persisted │──────────────────────┐         └──────────────┘
          └───────┘ │ └───────────┘                      ▼
                    │ cache            ┌──────────┐   ┌──────────┐
                    └────────────────▶ │  cached  │──▶│ evicted  │  (provenance stays)
                                       └──────────┘   └──────────┘
```

- **Transient:** inside one operation. Bounded by its budget, released when it returns (SR-120).
- **Result:** what the operation returns. If it is reported to a user or cited by anything, its **provenance record** is written: operation and version, input ids, parameters, environment. That record is small and is the object's identity when it is input-addressed.
- **Discarded:** used once and dropped. Its provenance (if written) remains, so it can be asked for again.
- **Cached:** kept in a bounded cache, keyed by its id; evictable at any time without loss of meaning.
- **Persisted:** kept until no root reaches it ([[Garbage Collection]]).
- **Evicted:** a cached object whose bytes are gone. Asking for it again recomputes it (invariant 3 of [[Storage and Materialization]]): same id, or a visible "not reproducible".

## Promotion and demotion

- **Promotion** (transient → result → cached → persisted) happens when an object proves worth keeping: it is cited by a decision or an artifact, pinned by a user, or the policy's cost estimate says so ([[Materialization Policy]]).
- **Demotion** (persisted → cached) happens when the last root that pinned it goes, and it is still reproducible. A non-reproducible object is never demoted to a cache: it is either kept or explicitly deleted.
- **Promoting an intermediate** gives it an identity for the first time: an AD tape, an MLIR module, a query plan. The promotion records the provenance that the intermediate never had.

## What must survive when bytes are dropped

| Kept | Why |
|---|---|
| the object's **id** | facts and artifacts cite it |
| its **provenance** (operation, version, inputs, parameters, environment) | to rebuild it and to judge whether a rebuild is the same object |
| its **reproducibility class**: deterministic, deterministic-modulo-environment, non-reproducible | to know whether rebuilding is allowed to stand in for it |
| for evidence: **what it established** and with which trust (SPEC-006 §5) | "was this ever checked?" must stay answerable |

## Examples from Sophia today

| Object | Today | Proposed |
|---|---|---|
| tree-sitter parse trees during ingest | transient, released | same |
| WP-66 canonical bytes of an ingested definition | persisted | cached (rebuild from source), see [[Current Inventory]] |
| semi-naive working relations of a query | transient, budgeted | same |
| a complete query result | persisted in `t_meta`, never evicted | cached with eviction ([[SP-014 The query result cache is never evicted]]) |
| the magic-set rewrite of a query | transient (rebuilt per `prepare`) | transient; promoted with a transformation record when an artifact relies on it (SPEC-006 §6a) |
| a world snapshot | persisted on `snapshot_world` | persisted only when cited |
| Metamath proof records | persisted | persisted as **authored** objects (they are the author's proofs), but rebuildable from the `.mm` blob if that is kept |
