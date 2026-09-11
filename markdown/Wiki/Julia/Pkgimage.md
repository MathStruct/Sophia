# Pkgimage

Since Julia 1.9, precompiling a package produces a **pkgimage** — a shared library containing *native machine code* for the package's compiled methods — alongside the older `.ji` file, which held only serialised lowered and type-inferred code. Before 1.9, native code was regenerated in every fresh process; pkgimages are the change that substantially reduced Julia's time-to-first-execution.

Mechanically, a pkgimage is a relocatable object built at precompile time, validated on load against the package's dependencies and their build IDs, and then `dlopen`ed. `PackageCompiler.jl` is the heavier-weight relative: it bakes chosen packages into a whole system image, trading flexibility for start-up speed.

The interesting limitation, and the one [[Content-Addressed Precompilation]] targets, is **granularity**. Validity is checked per package against a set of source file hashes and dependency build IDs. Editing one function invalidates the package image. Adding a method in a *dependent* package can invalidate compiled code in a dependency through [[World Age|backedge]] invalidation, at a granularity coarser than the actual dependency.

So pkgimages already establish two things this project needs: that caching native code across processes is worthwhile, and that the ecosystem accepts a cache keyed on content hashes of inputs. What they do not do is key on the *semantic* content of an individual definition, or persist the dispatch facts that would make invalidation exact. That gap is the concrete, measurable target of [[Roadmap|M3]].

## Related

- [[World Age]]
- [[Multiple Dispatch]]
- [[Content-Addressed Precompilation]]
- [[State of the Art - Julia Compilation and Precompilation]]
- [[State of the Art - Incremental Computation]]
