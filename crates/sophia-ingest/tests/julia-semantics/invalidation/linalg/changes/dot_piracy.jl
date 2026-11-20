# Type piracy: a more specific `dot` for Base's own vector type.
LinearAlgebra.dot(x::Vector{Float64}, y::Vector{Float64}) = sum(x .* y)
