# Naming and Change Propagation

Content addressing gives immutable identity. Humans need mutable names. This note is about the layer between them, and about what happens when someone edits a function that a thousand other functions depend on.

## Names are a separate, mutable relation

A `Name` node maps `(namespace, symbol) → hash`, with a validity interval. It is the only mutable thing in the store ([[Graph Schema]]). Everything else is append-only.

```
name(ns="Base.Math", sym="sqrt", target=b3:9f2a…, valid_from=t1, valid_to=∞)
```

Consequences, all inherited from [[Unison Codebase|Unison]]:

- **Renaming is free and non-breaking.** Dependents reference hashes. A rename touches one row.
- **Nothing is ever overwritten.** The old definition still exists at its hash; it simply stops being reachable by that name.
- **History is structural, not textual.** `git log` shows diffs of text; the name table shows which hash a name pointed at over time, and the hashes themselves show the actual dependency change.
- **Two definitions can share a name in different namespaces** with no ambiguity in the store, only at the UI layer.

## The update propagation problem

This is the part of the Unison model that is genuinely hard, and it is unavoidable here too.

Suppose `f` is edited to `f'`. `f ≠ f'` by hash, so `g` — which references `f` by hash — still references the *old* `f` and is unchanged. That is correct and is the source of all the good properties above. But the user's intent was usually "everything that used `f` should now use `f'`".

Propagating that intent means: for every `g` in the transitive dependents of `f`, construct `g'` by substituting `f'` for `f`, then `g''` for dependents of `g`, and so on — a *cascade* through the dependency DAG, where each rebuilt definition gets a new hash. The cascade is:

1. **Mechanical when types are unchanged.** If `type(f') ≡ type(f)`, substitution is sound and every dependent rebuilds automatically. A pure hash-substitution pass over the DAG, bottom-up.
2. **Semi-mechanical when types changed compatibly** (a widened parameter, a new default). Dependents may need adaptation; the system can propose and typecheck candidates.
3. **Manual when types changed incompatibly.** Each dependent is a separate edit that a human must make. Unison calls this the "todo list" after a patch, and in practice it is the main ergonomic cost of the model.

A `Patch` node records the set of replacements `{(h_old, h_new)}` and its propagation frontier, so the process is resumable and auditable.

## Where equivalence edges help

An [[Equivalence and Witnesses|EQUIV]] edge at level `defeq` or `rewrite` between `f` and `f'` makes case (1) *provable* rather than assumed. More interestingly, a `REFINES` edge lets propagation proceed for dependents that only rely on the refined behaviour — the cascade can be scoped by what each dependent actually observes, rather than being all-or-nothing.

This is the first place where the equivalence machinery pays for itself inside a single language, without any cross-language ambition. It is a good early demonstration target.

## Versioning and releases

There is no "version number" in the store — a version is just a `Name` binding at a point in time, or a named set of bindings (a `Namespace` snapshot, itself hashable). Semantic versioning becomes derivable rather than declared:

- patch-level change ⟺ new hash, `EQUIV` at `observational` or better
- minor change ⟺ new hash, `REFINES` old
- major change ⟺ new hash, no relation

Whether that derivation is trustworthy depends entirely on the strength of the available witnesses, which is the honest version of what semver pretends to guarantee today.

## Interaction with Git

The repository still holds text files that humans edit. Two models:

- **Git as source of truth, store as derived cache.** Safe, conventional, and means the store can always be rebuilt. Recommended for the whole early roadmap.
- **Store as source of truth, text as a projection** (the real Unison model). Stronger properties, much more tooling required, and it fights every existing editor, reviewer and CI system.

Starting with the first and keeping the second possible is the pragmatic path; the schema does not need to change between them, only which artifact is authoritative.

## Related

- [[Unison Codebase]]
- [[Content-Addressed Code]]
- [[Graph Schema]]
- [[Equivalence and Witnesses]]
- [[Content-Addressed Precompilation]]
