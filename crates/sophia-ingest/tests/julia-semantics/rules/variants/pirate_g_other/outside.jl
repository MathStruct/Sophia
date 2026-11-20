# `pirate_g`'s rule with another pullback body (a probe: a wrong derivative on
# purpose, so a program that misses the body shows as equal programs with
# different gradients).
module PirateG
import ChainRulesCore
using ChainRulesCore: NoTangent
import ..Rules
ChainRulesCore.rrule(::typeof(Rules.g), x) = (Rules.g(x), dy -> (NoTangent(), dy * x))
end
