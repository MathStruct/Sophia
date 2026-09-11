# Datalog

Datalog is a declarative query language: function-free Horn clauses over relations, with recursion and without negation-as-failure in its pure form. A rule reads "head holds if all body atoms hold":

```datalog
depends(A, B) :- direct_dep(A, B).
depends(A, C) :- depends(A, B), direct_dep(B, C).
```

Evaluation computes the least fixpoint of the rules, typically by **semi-naive evaluation**, which on each iteration joins only against facts derived in the previous round rather than against everything — the difference between a usable and an unusable transitive closure.

Datalog is the standard language of program analysis: Doop (points-to analysis for Java), Soufflé (a compiled, high-performance engine), CodeQL and Glean's Angle are all in this family. The reason is that most static analyses *are* least fixpoints over program facts, and writing them as rules rather than as hand-rolled worklist algorithms is dramatically shorter and automatically incrementalisable.

For [[Compilation as Query|Sophia]], Datalog is the natural way to express a compiler pass as a rule that *derives* new nodes and equivalences rather than mutating an IR in place. **egglog** takes this further by fusing Datalog with [[E-Graph|e-graphs]], so rules can assert equalities and the engine maintains congruence closure — which is very close to a complete description of what this project wants its optimiser to be.

## Related

- [[E-Graph]]
- [[Equality Saturation]]
- [[Property Graph]]
- [[Compilation as Query]]
- [[Query Cookbook]]
- [[State of the Art - Graph Databases for Code]]
