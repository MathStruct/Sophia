# A parametric argument type: only the `Vector{Int}` call reaches it.
Inval.scale(v::Vector{Int}) = 3.0
