# Macro expansions as transformation records (experiment E-4): what each
# module-level macro call expands to, whether Sophia finds the methods in the
# expansion, and whether the expansion is reproducible from its recorded
# inputs. See tests/expansions.rs and [[Semantic Boundary Experiments]] §E-4.
module Exp

newer(x) = x + 1

# `@eval` loops: a qualified name spliced twice; types as values; a splat.
struct Wrap
    x::Float64
end
for f in (:sin, :cos)
    @eval Base.$f(w::Wrap) = Wrap($f(w.x))
end
for T in (Int, Float64)
    @eval half(x::$T) = x / 2
end
for args in ((:a, :b), (:a, :b, :c))
    @eval arity($(args...)) = $(length(args))
end

# A value whose printed form does not parse back: a closure.
for g in (x -> x + 1,)
    @eval applyg(x) = $g(x)
end

# A macro with hidden state: its output depends on how often it ran.
const COUNTER = Ref(0)
macro counted(name)
    COUNTER[] += 1
    :($(esc(Symbol(name, COUNTER[])))() = 1)
end
@counted c

# A macro that makes two functions, with hygienic local names.
macro twice(name)
    quote
        $(esc(name))(x) = 2x
        $(esc(Symbol(name, :_neg)))(x) = -2x
    end
end
@twice tw

# Macros of Base that make what Sophia cannot see as written.
@deprecate older(x) newer(x)
@enum Fruit apple banana
Base.@kwdef struct Opts
    n::Int = 3
end

# A macro around a definition.
@inline inl(x) = x + 1

# A docstring on a macro call, and on definitions an `@eval` loop makes.
"Documented."
@inline docd(x) = x
for n in (:d1, :d2)
    @eval begin
        "Made by a loop."
        $n() = 0
    end
end

# `@static` around an `@eval` loop: a form with `@eval` gives eval records only.
@static if Sys.iswindows()
    win() = 1
else
    for n in (:s1, :s2)
        @eval $n() = 1
    end
end

# `@eval M ex`: a definition made in another module.
module Inner end
for n in (:i1,)
    @eval Inner $n(x) = x
end

# A macro inside a block that is not expanded: Julia's methods, record 0.
if true
    @inline in_if(x) = x
end

# Cases from the review of B-025 (Codex).
# State changed between two identical calls: one address, two texts.
const STATE = Ref(1)
macro stateful()
    esc(:(stateful_f() = $(STATE[])))
end
@stateful
STATE[] = 2
@stateful
# A splice with a side effect: evaluated once, as Julia does.
const SIDE = Ref(0)
@eval sidefx() = $(SIDE[] += 1)
# A gensym inside a quoted symbol.
macro qgen()
    esc(:(qgen_f() = $(QuoteNode(gensym(:x)))))
end
@qgen
# A macro whose expansion calls `@eval`.
macro mkeval(n)
    esc(:(@eval $n() = 5))
end
@mkeval five
# A file included into two modules, with a macro call that is not expanded.
module A2
include("shared.jl")
end
module B2
include("shared.jl")
end

end
