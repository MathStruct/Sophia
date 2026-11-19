# What does a macro call at module level define, and does Sophia's id see
# the macro? Part 5 of the Julia conformance suite (macros): see
# tests/macros.rs and [[Julia Conformance]].
module Mac

# A macro that makes two functions from one name.
macro twice(name)
    quote
        $(esc(name))(x) = 2x
        $(esc(Symbol(name, :_neg)))(x) = -2x
    end
end

# A macro that returns its argument unchanged.
macro same(ex)
    esc(ex)
end

solo(x) = x

# A macro around a definition: the method is made, and the macro changes it.
@inline inl(x) = x + 1
@noinline function noinl(x)
    x + 1
end
Base.@propagate_inbounds getit(v, i) = v[i]
@generated function gen(x)
    :(x)
end

# A macro around a definition that changes nothing.
@same kept(x) = x + 1

# A macro around a definition with a default argument.
@inline defarg(x, y = 1) = x + y

# Macros that make definitions Sophia cannot see.
@twice tw
@deprecate old(x) solo(x)
@enum Color red green

# A struct whose macro adds a keyword constructor.
Base.@kwdef struct K
    a::Int = 1
end

# `@eval` loops: an interpolated name, and a literal name with an
# interpolated signature (one template, two methods).
for (n, v) in ((:one, 1), (:two, 2))
    @eval $n() = $v
end
for T in (:Int, :Float64)
    @eval tmpl(x::$T) = 1
end

# `@eval` on a literal definition.
@eval evald(x) = 3

# One branch is compiled, on this platform.
@static if Sys.isapple()
    plat() = 1
else
    plat() = 2
end

# A docstring.
"Documented."
documented(x) = x

# Cases from the review of B-074 (Mistral).
# Two forms on one line.
@inline same_a(x) = 1; @noinline same_b(x) = 2
# A macro call whose arguments before the definition are not ASCII.
macro same2(s, ex)
    esc(ex)
end
@same2 "αβ" uni(x) = x + 1
@inline uni_after(x) = x + 2
# A parenthesised macro argument.
@same(paren(x) = x + 1)
# A method of a type parameter's constructor.
struct Box end
@inline (::Type{T})(c::Char) where {T<:Box} = T()
# A docstring on a module, then a form.
"Inner doc."
module Dm
@inline dm(x) = x
end
@inline after_dm(x) = x
# A baremodule.
baremodule Bm
using Base: @inline, +
@inline bm(x) = x + 1
end
# `@eval` with a splice, not in a loop.
@eval $(Symbol("direct"))(x) = 4

end
