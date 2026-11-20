# A more specific method for a `where` method: `first_or(::Vector{Int}, ::Int)` reaches it.
Inval.first_or(v::Vector{Int}, d::Int) = -1
