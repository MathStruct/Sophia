# `convert` to Float64 from a new type: an extension known to invalidate widely.
struct MyReal <: Real
    x::Float64
end
Base.convert(::Type{Float64}, r::MyReal) = r.x
