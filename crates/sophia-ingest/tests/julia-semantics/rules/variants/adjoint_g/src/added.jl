# A rule for `g` in Zygote's own system: a method of `ZygoteRules._pullback`, made by a macro.
import Zygote
Zygote.@adjoint g(x) = g(x), dy -> (dy * (cos(x) * x + sin(x)),)
