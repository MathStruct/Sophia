# Include order: part 9 of the Julia conformance suite.
# See tests/includes.rs and [[Julia Conformance]].
module Inc

root_first() = 1

# An include in a function body, defined here and called at the end: Julia
# evaluates the file last, Sophia reads it here.
load_late() = include("late.jl")

# A literal include, a file in a subdirectory that includes its neighbour
# (relative to itself) and opens a submodule with its own include.
include("a.jl")
include("sub/b.jl")

# A submodule whose file is included inside it.
module Inner
include("inner.jl")
end

# One file included into two modules: Julia evaluates it in both.
include("twice.jl")
module Other
include("twice.jl")
end

# One file included twice into the same module: the second evaluation
# replaces the first one's methods.
include("again.jl")
include("again.jl")

# A path that is not in normal form.
include("sub/../norm.jl")

# Includes Julia evaluates and the literal-include rule does not see: on a
# qualified `include`, through `Base.include`.
Inc.include("qualified.jl")
Base.include(Inc, "baseinc.jl")
shared(x::Int) = 1
use_shared(x) = shared(x)

# Includes that are not literal (dynamic), in a module of their own.
module Dyn
include(joinpath(@__DIR__, "dyn", "joined.jl"))
for f in ("looped.jl",)
    include(joinpath("dyn", f))
end
include(identity, "dyn/mapped.jl")
end

# Includes Julia never evaluates at load: a branch `@static` removes, a
# function nobody calls, a quoted expression.
@static if false
    include("never_static.jl")
end
never_called() = include("never_called.jl")
const QUOTED = :(include("never_quoted.jl"))

load_late()

root_last() = 2

end
