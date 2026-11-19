# Mutable globals against constants: part 7 of the Julia conformance suite.
# See tests/globals.rs and [[Julia Conformance]].
module Glob

# A constant: every use reads this value (until a `const` redefinition,
# which is a new world, like a new method).
const LIMIT = 10
use_limit(v) = v + LIMIT

# A mutable global nothing in the package assigns again.
scale = 2
use_scale(v) = v * scale

# A mutable global a function assigns.
counter = 0
read_counter() = counter
set_counter(n) = (global counter = n)
function reset_counter()
    global counter
    counter = 0
end

# A typed global: mutable, its type fixed.
level::Int = 1
use_level() = level + 1
set_level(n) = (global level = n)
closure_level(v) = map(x -> x + level, v)

# A typed global assigned twice at module level.
mode2::Symbol = :a
mode2::Symbol = :b
use_mode2() = mode2

# A global assigned twice at module level.
mode = :a
mode = :b
use_mode() = mode

# Constants holding mutable objects: the binding is constant, the object is not.
const CACHE = Dict{Int,Int}()
use_cache(k) = get(CACHE, k, 0)
const TICKS = Ref(0)
tick() = (TICKS[] += 1)

# Declared at module level, assigned only by a function.
global later
make_later() = (global later = 5)
use_later() = later

# Assigned only by a function, with no declaration at module level.
make_fresh() = (global fresh = 1)
use_fresh() = fresh

# A local named like a global: in a function, `scale = 3` makes a new local.
shadow_scale(scale) = scale + 1
local_scale() = (scale = 3; scale)

# A global read inside a closure.
closure_scale(v) = map(x -> x * scale, v)

# A constant redefined (Julia ≥ 1.12: a new binding partition, a new world).
const STEP = 1
const STEP = 2
use_step() = STEP

# A callable object whose argument is named like a constant: inside the
# method, `UNIT` is the argument.
struct Scale
    k::Int
end
const UNIT = Scale(1)
(UNIT::Scale)(x) = UNIT.k * x

# A constant computed from another constant (no method: Julia evaluates it
# once, when the module is loaded).
const DOUBLE = 2 * LIMIT

# Constants an `@enum` makes (macro output).
@enum Colour RED GREEN
use_red() = RED

# Another module's globals, by a qualified name, and assigned from here.
module Inner
depth = 3
const WIDTH = 4
end
use_depth() = Inner.depth
use_width() = Inner.WIDTH
set_depth(d) = setproperty!(Inner, :depth, d)

# A parameter and a local named like a constant (not a callable object).
shadow_limit(LIMIT) = LIMIT + 1
local_limit() = (LIMIT = 3; LIMIT)

# Review B-078 (Codex), 2: two definitions using a constant on one line.
amb_a() = LIMIT; amb_b() = LIMIT
# 3: two definitions starting on one line, neither inside the other.
amb_f() = LIMIT; function amb_g()
    LIMIT
end
# 4: a generic use on the last line of another definition.
function gen_f(mode2)
    mode2
end; gen_g() = mode2
# 5: initializers that call a refused definition.
const BAD = set_counter(1)
badlevel::Int = set_counter(1)
# 8: a constant holding a mutable string type.
mutable struct Text <: AbstractString
    value::String
end
const TEXT = Text("a")
read_text() = TEXT

end
