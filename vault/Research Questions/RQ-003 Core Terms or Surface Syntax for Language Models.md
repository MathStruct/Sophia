#open-problem

> **Status: open (2026-10-25).** Area: machine learning, experiment design · Prerequisites: sophialang core (SPEC-003), rendering profiles (SPEC-005) · Related: [[LLMs and Mixed-Language Code]], [[Compression and Canonical Form]], SR-62, SR-73.
>
> Sources: Chen et al., *Evaluating Large Language Models Trained on Code*, arXiv:2107.03374, 2021 (models are strongest in high-resource languages); the question is from [[LLMs and Mixed-Language Code]] (SR-73), lifted here.

## Question

Sophia stores code in a canonical core form and prints it in rendering profiles that look like Julia,
Rust or Python. A model can write a profile's surface syntax, which a parser checks and normalises,
or it can write core terms directly. **Which gives more correct definitions per attempt, at what
cost in tokens, and does the answer change when the core form is given a vocabulary of mined
abstractions or when the model is fine-tuned on core terms?**

## Setting

- **Surface:** the model writes a definition in a profile (`julia`, `rust`, `python`). The parser
  rejects what it cannot normalise, and the model may retry with the error.
- **Core:** the model writes the core term in its printed form (SPEC-003), with references as names
  that the store resolves.
- **Correct:** the stored definition passes the task's tests. The same tests, the same reference
  definitions and the same retry budget apply in both conditions.

## What is known

- **Expected, not measured:** models do best in languages common in their training data (Chen et al.).
  That favours surface syntax without fine-tuning.
- **Expected, not measured:** a canonical form has no stylistic variation, which could help a
  fine-tuned model. A vocabulary of abstractions could shorten outputs ([[Compression and Canonical
  Form]] §3.5).
- Nothing has been measured. `crates/sophia-mcp`, through which a model would reach the store, is
  planned (M3), not built.

## What would count as an answer

A measurement protocol and its result:
- a task set of at least a few hundred definitions with tests, drawn from code already in the store;
- pass@1 and pass@k per condition, tokens per correct definition, and the share of parser rejections;
- at least two models, one of them fine-tuned on core terms if the zero-shot result is close.

## Why it matters here

It decides what Sophia shows a model and what a model writes back through Moonkale's AI path. If
core terms win, the store's own form is the interface. If surface wins, profiles stay the interface,
and training on core terms is a later project (SR-62).

## Leads

- Start zero-shot with the `julia` profile against core, on LinearAlgebra functions with their tests.
- Constrained decoding with the core grammar removes syntax errors from the comparison, which isolates
  semantic errors.
