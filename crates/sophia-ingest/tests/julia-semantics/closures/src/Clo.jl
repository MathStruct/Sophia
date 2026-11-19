# Fixture for part 6 of the Julia conformance suite (closures and captured
# variables): what each closure captures, what is its own, and which module
# names it uses, as Julia lowers it (`closures.jl`) against Sophia's
# encoding (`sophia_ingest::conformance::closure_ledger`, `tests/closures.rs`).
# Line numbers are asserted by `tests/closures.rs`: add cases at the end.
module Clo

const scale = 3
const k = 10
z = 0
total = 0
solo(x) = x + 1

# A parameter captured.
adder(n) = x -> x + n
# A module constant used inside a closure.
scaled(v) = map(x -> x * scale, v)
# A parameter named like a module constant: captured, not the constant.
shadows(k) = x -> x + k
# A captured variable the closure assigns: boxed.
counter() = (c = 0; () -> (c += 1))
# A local assigned after the closure that reads it: still the outer local.
late() = (f = () -> z; z = 3; f())
# The closure assigns a name the function has nowhere else: its own local.
fresh() = () -> (y = 1; y)
# The closure assigns a module global's name: in a function body that makes
# a local of the closure, not the global.
assigns_global() = () -> (total = 2; total)
# `global` inside a closure: the module's binding.
bumps() = () -> (global total += 1)
# Nested closures: the outer one captures what the inner one needs.
curry(a) = b -> c -> a + b + c
# A `do` block.
withdo(v, a) = map(v) do x
    x + a
end
# Named inner functions.
function outer(a)
    inner(x) = x + a
    function inner2(y)
        y + solo(a)
    end
    inner(1) + inner2(2)
end
# A static parameter used in a closure.
sp(x::T) where {T} = () -> T
# A loop variable captured.
function loops(v)
    fs = []
    for i in v
        push!(fs, () -> i)
    end
    fs
end
# `local` in a closure shadows the outer variable.
function shadowed(x)
    f = () -> (local x = 2; x)
    f() + x
end
# A closure's parameter shadows an outer local of the same name.
function param_shadow(x)
    g = x -> x + 1
    g(x)
end
# A closure that calls a module function and captures a local.
calls_solo(a) = () -> solo(a)
# A closure in a `let` that captures the `let` variable.
in_let(a) = let b = a + 1
    () -> b
end
# Generators and comprehensions are closures too.
gen(v, a) = sum(x + a for x in v)
comp(v, a) = [x + a for x in v]
# A closure with a keyword argument whose default is a captured local.
kw(a) = (x; d = a) -> x + d
# The outer local is assigned only after the closure is made, and a module
# global of that name exists: `late` with a definition to resolve to.
late_solo() = (f = () -> solo; solo = 2; f())
# An inner function in short form only (an inner `function … end`, as in
# `outer`, makes the definition refused).
outer_short(a) = (inner(x) = x + a; inner(1))
# A multi-line anonymous function.
multi(a) = x -> begin
    y = x + a
    y * 2
end
# A closure inside a closure assigns the outermost variable.
nested_assign() = (c = 0; () -> (() -> (c += 1))())
# A variable local to the function only through a branch taken later.
branch(b) = (f = () -> w; if b
    w = 1
end; f)
# One closure's local is not another's: `q` in `h` is the module's.
two_closures() = (g = () -> (q = 1); h = () -> q; h)
# A captured variable passed as a keyword argument of the same name.
kwname(atol) = () -> isapprox(1.0, 1.0; atol = atol)
# Keyword arguments in a call (no closure). Julia: `kw_a(0.5)` is false,
# `kw_r(0.5)` true; `kw_x()` false, `kw_y()` true.
kw_a(atol) = isapprox(1.0, 2.0; atol = atol)
kw_r(rtol) = isapprox(1.0, 2.0; rtol = rtol)
kw_x() = isapprox(1.0, 2.0; atol = 0.5)
kw_y() = isapprox(1.0, 2.0; rtol = 0.5)
# Two closures on one line, one with destructured arguments.
pairs(v) = mapreduce(x -> (x, x), ((a, b), (c, d)) -> (a + c, b * d), v)
# Three on one line: `_` is unnamed, two take `j`.
ranges(u, K) = u == 'F' ? _ -> (1:K) : u == 'U' ? j -> (1:min(j, K)) : j -> (j:K)
# Review B-076 (Mistral), 1: a `do` block whose body line starts another
# closure (Julia files the `do` closure at its body's line).
do_inner(v, a, b) = map(v) do x
    (x -> x + a)(x) + b
end
# Review B-076, 2: two definitions with closures on one line.
two_h(a) = (x -> x + a); two_k(b) = x -> x + b
end
