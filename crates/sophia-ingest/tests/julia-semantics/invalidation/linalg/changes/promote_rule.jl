# A promotion rule between a new type and Float64.
struct MyFloat <: AbstractFloat
    x::Float64
end
Base.promote_rule(::Type{MyFloat}, ::Type{Float64}) = MyFloat
