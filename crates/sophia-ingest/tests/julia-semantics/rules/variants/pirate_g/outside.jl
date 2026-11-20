# A rule for `Rules.g` defined outside the package (another package in the same session).
module PirateG
import ChainRulesCore
using ChainRulesCore: NoTangent
import ..Rules
ChainRulesCore.rrule(::typeof(Rules.g), x) = (Rules.g(x), dy -> (NoTangent(), dy * (cos(x) * x + sin(x))))
end
