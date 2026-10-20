module Macros
struct NoPivot end
fact!(A, p; check::Bool = true) = A
# v7: `check` here is a keyword argument, not a module-level unit.
@deprecate fact!(A, ::Val{false}; check::Bool = true) fact!(A, NoPivot(); check) false
# Definitions that a macro wraps whole are still units.
@inline twice(x) = 2x
Base.@propagate_inbounds @inline function thrice(x)
    3x
end
@inline(halve(x) = x / 2)
@static if true
    function picked(x)
        twice(x)
    end
end
# A top-level call's keywords aren't units either.
configure(; level::Int = 1) = level
configure(; level::Int = 2)
end
