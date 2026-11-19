function outer(x)
    inner(y) = y + x
    inner(x)
end
@doc "A macro doc wrapper" documented_macro(x) = x
const typed_const::Int = 1
if true
    conditional(x) = x
end
@deprecate legacy(x; check::Bool=true) replacement(x; check)
Base.:(==)(a::Bits, b::Bits) = true
positional_default(x, k::Integer = 0) = x + k
