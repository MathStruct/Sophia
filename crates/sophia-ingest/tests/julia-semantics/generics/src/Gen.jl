# Which generic function does each method belong to? A generic function is
# (module, name): see tests/generics.rs.
module Gen
import Base: show
import Base.length

include("sub.jl")

module A
f(x) = x + 1
f(x, y) = x + y
end

module B
f(x) = x - 1
g(x) = f(x) * 2
end

struct Point
    x::Float64
end
Point() = Point(0.0)
Point(a, b) = Point(a + b)

show(io::IO, p::Point) = print(io, p.x)
length(p::Point) = 1
Base.:+(p::Point, q::Point) = Point(p.x + q.x)
Base.getindex(p::Point, i) = p.x
A.f(p::Point) = p.x

f(p::Point) = 2
f(p::Point, q) = 3
use(p) = f(p) + f(p, p)
end
