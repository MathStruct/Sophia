# Which function, and which method, does a call reach? Part 3 of the Julia
# conformance suite (dispatch): see tests/dispatch.rs and [[Julia Conformance]].
module Disp
import Base: length

struct Point
    x::Float64
end

# One method in the module, and nothing adds another: the call reaches it.
solo(x) = x + 1
call_solo(x) = solo(x)

# Several methods: which one depends on the argument's type.
duo(x::Int) = 1
duo(x::String) = 2
call_duo(x) = duo(x)

# One method written here, a second added by a qualified head below.
module Inner
h(x) = 1
call_h(x) = h(x)
end
Inner.h(x::Int) = 2

# One method written here, a second added from another file after `import`.
lone(x) = 0
call_lone(x) = lone(x)
include("ext.jl")

# A function of Base, extended here: the call is Base's `length`.
length(p::Point) = 1
call_length(v) = length(v)

# A function of Base, not extended.
call_sum(v) = sum(v)

# A parameter named like a function: the call is the argument.
shadow(duo) = duo(1)
shadow_solo(solo) = solo(1)

# A type: the call is its constructor.
make(x) = Point(x)

# A qualified call into a submodule.
call_qualified(x) = Inner.h(x)

# Inside a closure: the closure is a method of its own in Julia.
call_in_closure(xs) = map(y -> duo(y), xs)

# A function passed as a value, not called.
pass_solo(xs) = map(solo, xs)

# Default arguments: one definition, two methods in Julia, both its own.
defarg(x, y = 1) = x + y
call_defarg(x) = defarg(x)

# A generator: Julia files its body at no line.
call_in_generator(xs) = sum(duo(x) for x in xs)

# A loop variable spliced into an `@eval` template, named like a function:
# `$duo` is `:(:U)`, not the function `duo`.
for (T, duo) in ((:Int, :(:U)),)
    @eval function tagged(x::$T)
        return (x, $duo)
    end
end

# A multi-line definition.
function multi(x)
    y = solo(x)
    return duo(y)
end

# Two definitions on one line: Julia gives only the line, so neither is compared.
same_a(x) = 1; same_b(x) = solo(x)

# One name, two bindings in one body: `h` here and `Inner.h`.
h(x) = 5
both(x) = h(x) + Inner.h(x)
end
