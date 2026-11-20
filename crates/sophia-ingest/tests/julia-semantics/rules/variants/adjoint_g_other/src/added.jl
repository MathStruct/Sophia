# `adjoint_g`'s adjoint with another pullback body (a probe: a wrong
# derivative on purpose, as in `pirate_g_other`).
import Zygote
Zygote.@adjoint g(x) = g(x), dy -> (dy * x,)
