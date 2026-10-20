module Geo
using LinearAlgebra: norm
export Point, dist
include("types.jl")
include("ops.jl")
module Inner
helper(x) = x
scale(p) = helper(p)
end
end
