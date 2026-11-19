# What a change to the world (a method or an import added after the
# package is loaded) does to Sophia's facts. Part 4 of the Julia
# conformance suite: see tests/world.rs and [[Julia Conformance]]. Each file
# in `changes/` (evaluated into `World`) and `../outside/` (into `Main`)
# is one change; the package is never edited.
module World

struct Point
    x::Float64
end

# One method: callers refer to it by content.
solo(x) = x + 1
call_solo(x) = solo(x)
# Refers to `call_solo` by content: its id contains `call_solo`'s.
twice(x) = call_solo(x) + 1

# Several methods: callers use the function.
duo(x::Int) = 1
duo(x::String) = 2
call_duo(x) = duo(x)

# Base's functions, used through the implicit `using Base`.
call_sum(v) = sum(v)
call_max(v) = maximum(v)

module Inner
h(x) = 1
call_h(x) = h(x)
end

end
