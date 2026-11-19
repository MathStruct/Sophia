module Pars
public long_where
function long_where(x::T; scale=1)::T where T
    x * scale
end
short_where(x::T; scale=1)::T where T = x * scale
kw_semicolon(x) = identity(x; atol=x)
kw_comma(x) = identity(x, atol=x)
kw_shorthand(x) = identity(; x)
x::Int = 1
const a, b = 1, 2
+(a::Nothing, b::Nothing) = a
 a ⊕ b = a
Base.:+(x::Nothing) = x
struct Callable end
(x::Callable)(n) = n
function (::Type{T})(x) where T
    x
end
juxtapose(x) = 2x
adjoint_case(x) = x'
end_index(a) = a[end]
function dotted(x)
    x .+= 1
end
var"odd name"(x) = x
interpolated(x) = """value: $x"""
#= outer #= nested =# comment =#
strings(x) = (r"a.*", raw"$literal", x)
do_case(xs) = map(xs) do x
    x + 1
end
lambda_case(x) = map(y -> y + x, [1])
@kwdef struct Options
    n::Int = 1
end
mutable struct Mutable
    n::Int
end
primitive type Bits 8 end
abstract type Abstract end
baremodule Bare
    bare(x) = x
end
module Inner
    nested(x) = x
end
@inline function inlined(x)
    x
end
"""The documented function."""
function documented(x)
    x
end
macro m(x)
    x
end
function refused(x)
    global state = x
end
include("More.jl")
end
