module Callable
struct S
    x::Int
end
struct Box{T}
    value::T
end
# A callable-type method extends whatever `Bi` is: it binds no name.
function (::Type{Bi})(x) where {Bi<:Box}
    Bi(x)
end
# An interpolated head (in `@eval` code) binds nothing either.
$S(a) = a
# `S` here is the type parameter, not the struct.
first_of(x::S) where {S} = x
uses_struct(s::S) = s.x
apply(f, x) = f(x)
apply(f, x, y) = f(x, y)
# `apply` is a local here, so not a use of the generic function.
twice(apply, x) = apply(apply(x))
calls(x) = apply(identity, x)
end
