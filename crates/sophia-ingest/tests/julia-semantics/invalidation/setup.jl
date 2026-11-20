# Loads the fixture and compiles its callers; `Inval` is the module whose
# compiled code `invalidation.jl` records.
# Code outside the target module, reached from it only through `invoke`.
module Helper
h(x) = 1
@noinline g(x::Number) = h(x)
end
include(joinpath(@__DIR__, "Inval.jl"))
Inval.workload()
const TARGET = Inval
