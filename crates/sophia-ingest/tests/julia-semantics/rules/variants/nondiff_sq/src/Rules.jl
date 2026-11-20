module Rules
import ChainRulesCore
using ChainRulesCore: NoTangent

sq(x) = x * x
g(x) = sin(x) * x
h(x) = exp(x) + one(x)
ChainRulesCore.rrule(::typeof(h), x) = (h(x), dy -> (NoTangent(), dy * exp(x)))

f1(x) = g(x) + sq(x)
f2(x) = g(sq(x))
f3(x) = sq(x) + one(x)
f4(x) = 2 * h(x)
unused(x) = 3 * x
# Mutually recursive: one closure for both, two different derivatives.
p(x) = x > 1 ? q(x - one(x)) : x
q(x) = x > 1 ? p(x - one(x)) : 2 * x
# A call Julia cannot infer: the walk sees `Any` (runtime dispatch).
dyn(x) = Base.inferencebarrier(g)(x)
include("added.jl")
end
