#overview #open-problem

> Open research questions that Sophia raises, each stated so that it can be worked on without
> reading the rest of the vault: the setting, the question, what is known, and what would count as
> an answer. Numbered `RQ-nnn`; the template is `vault/templates/Research Question Template.md`.
> Asked for by Daniel on 2026-10-25 (SR-147), after the section in Lenticulum.jl's vault.

This is not the [[Problem Log]]. A problem (`SP-nnn`) is something in the code or the project that
needs a decision or an implementation. A research question is mathematics, computer science or an
experiment whose answer nobody here knows, and which could be a paper, a thesis chapter, or a long
afternoon. Questions come up during implementation and while discussing the theory; when one is
self-contained, it is lifted here and the note it came from links to it.

A question gets an answer by a proof, a counterexample or a measurement, recorded in its note with
its source. Then its status changes, and it stays listed.

| # | Question | Area | Status |
|---|---|---|---|
| [[RQ-001 Recursive Groups up to Bisimilarity]] | Should a recursive group's ids respect behavioural equivalence (bisimilarity), and is there a canonical form that does, cheaply and compatibly with Merkle hashing? | coalgebra, graph canonisation | open |
| [[RQ-002 When Incremental Collection Equals a Fresh Build]] | Under which conditions on the files does collecting after each batch leave exactly the records of a store built fresh from the current files? | garbage collection, incremental computation | open (the known counterexample is refused) |
| [[RQ-003 Core Terms or Surface Syntax for Language Models]] | Does a language model write correct code more reliably in a canonical core form than in a familiar surface syntax that a parser checks? | machine learning, experiment design | open |
