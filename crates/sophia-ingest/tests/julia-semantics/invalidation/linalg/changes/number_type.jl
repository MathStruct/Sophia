# A new number type with arithmetic, as a units or dual-number package defines.
struct MyNum <: Number
    x::Float64
end
Base.:*(a::MyNum, b::MyNum) = MyNum(a.x * b.x)
Base.:+(a::MyNum, b::MyNum) = MyNum(a.x + b.x)
Base.zero(::Type{MyNum}) = MyNum(0.0)
