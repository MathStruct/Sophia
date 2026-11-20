# A rule for `Base.sin` at Float64 defined outside the package, more specific than ChainRules's.
module PirateSin
import ChainRulesCore
using ChainRulesCore: NoTangent
ChainRulesCore.rrule(::typeof(sin), x::Float64) = (sin(x), dy -> (NoTangent(), dy * cos(x)))
end
