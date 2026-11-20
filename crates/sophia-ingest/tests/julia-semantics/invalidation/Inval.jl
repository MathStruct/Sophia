# Dispatch facts and their invalidation (E-3, B-024). Each caller below is
# compiled by `workload()`; `invalidation.jl` records, for every call in every
# compiled caller, the argument types and the methods the call can reach.
# Each file in `changes/` adds one method (evaluated into `Main`, as another
# package would); the module itself is never edited.
module Inval

abstract type Shape end
struct Circle <: Shape
    r::Float64
end
struct Sq <: Shape
    a::Float64
end

# Two methods: callers with a concrete, an abstract and an untyped argument.
area(s::Shape) = 0.0
area(c::Circle) = c.r^2
use_circle(c::Circle) = area(c)
use_sq(s::Sq) = area(s)
use_shape(v::Vector{Shape}) = area(v[1])
use_any(v::Vector{Any}) = area(v[1])
total(v::Vector{Shape}) = sum(area, v)

# `invoke` chooses the method for `Tuple{Shape}` whatever the argument's type.
use_invoke(c::Circle) = invoke(area, Tuple{Shape}, c)
# `invoke` of a function outside the module (`Main.Helper`, defined by setup.jl).
use_helper(x::Int) = invoke(Main.Helper.g, Tuple{Number}, x)

# Another function with the same argument types.
perimeter(s::Shape) = 1.0
use_perimeter(c::Circle) = perimeter(c)

# Parametric arguments: `Vector{Float64}` and `Vector{Int}` are different types.
scale(v::Vector{Float64}) = 2.0
scale(v::AbstractVector) = 1.0
use_scale_f(v::Vector{Float64}) = scale(v)
use_scale_i(v::Vector{Int}) = scale(v)

# A method with a type parameter (`where`).
first_or(v::AbstractVector{T}, d::T) where {T} = isempty(v) ? d : v[1]
use_first(v::Vector{Int}) = first_or(v, 0)

# Base's functions called with the module's types.
struct Meters
    x::Float64
end
to_meters(x::Float64) = convert(Meters, x)
Base.convert(::Type{Meters}, x::Float64) = Meters(x)
len_of(v::Vector{Int}) = length(v)

function workload()
    use_circle(Circle(1.0))
    use_sq(Sq(1.0))
    use_shape(Shape[Sq(1.0)])
    use_any(Any[Sq(1.0)])
    total(Shape[Circle(1.0), Sq(2.0)])
    use_invoke(Circle(1.0))
    use_helper(1)
    use_perimeter(Circle(1.0))
    use_scale_f([1.0])
    use_scale_i([1])
    use_first([1])
    to_meters(1.0)
    len_of([1])
    nothing
end

end
