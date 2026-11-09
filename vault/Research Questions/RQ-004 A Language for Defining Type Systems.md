#open-problem

> **Status: open (2026-11-09).** Area: type theory, logical frameworks · Prerequisites: sophialang core (SPEC-003), annotations and checkers (SPEC-001 §8) · Related: SR-154, SR-53, [[SPEC-002 Sophialang Principles]] §3, [[Computational Trinitarianism]] §2.3, §3.
>
> Sources: Harper, Honsell, Plotkin, *A Framework for Defining Logics*, JACM 40(1), 1993 (LF); Pfenning, Schürmann, *System Description: Twelf*, CADE 1999; Cousineau, Dowek, *Embedding Pure Type Systems in the Lambda-Pi-Calculus Modulo*, TLCA 2007; Assaf et al., *Dedukti: a Logical Framework based on the λΠ-Calculus Modulo Theory*, arXiv:2311.07185, 2023. The question comes from Daniel's message of 2026-11-09 (SR-154) and from the external reviews of that message ([[Review 2026-11-09 Scope (external)]]).

## Question

sophialang is meant to have a third part: a language in which arbitrary type systems can be stated, beside the programming and query parts (SR-154). Today the specifications have one fixed dependent core (SPEC-003) and "exchangeable checkers" (SPEC-002 §3), but a checker is an external tool with a version. Nothing says how its type system is written down. **Can a logical framework inside sophialang state a type system's judgments and rules as content-addressed definitions? Can checking under that system then be a versioned derivation that Sophia can check, without the core learning each system? And what does the fixed core have to contain for that to work?**

## Setting

- **Representing a system:** a type system is given by its syntax, its judgment forms (such as Γ ⊢ e : A) and its inference rules. In a logical framework these become signatures: constants whose types encode the rules, so that a derivation is a term (LF, judgments as types).
- **Checking:** a claim "e : A under system T" is an annotation whose evidence is a derivation in T's signature. Checking it is type-checking in the framework, which stays one small checker whatever T is.
- **Computation:** for systems whose rules compute (definitional equality, as in dependent or cubical types), a framework needs rewriting. λΠ-modulo (Dedukti) adds rewrite rules to LF for this.

## What is known

- LF and Twelf encode many logics and type systems as signatures and check derivations with one checker (Harper, Honsell, Plotkin; Pfenning, Schürmann).
- λΠ-modulo embeds every functional pure type system (Cousineau, Dowek). Dedukti checks proofs translated from several provers in one framework (Assaf et al.).
- **Not checked here:** whether sophialang's planned core (SPEC-003: Π, universes, `Prop`, inductive types, quantities) is enough to serve as such a framework, or whether it needs rewrite rules as in λΠ-modulo.
- **Open in the design:** SPEC-002 treats checkers as tools, so this question could also be answered "keep them as tools, and record only their rules as documentation". That would be a choice, not a result.

## What would count as an answer

- A design that states at least two different systems in one framework, for example simply typed λ-calculus and a linear type system, with derivations checked by one framework checker, and with stored ids. It should also say what the core needs.
- Or an argument that a framework inside sophialang costs more than it gains, compared with external checkers plus documentation, with the reason.
