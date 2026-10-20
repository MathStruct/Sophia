module Dyn
include("types.jl")
include(joinpath(@__DIR__, "extra.jl"))
f(x) = origin()
g(x) = Point(x, x)
end
