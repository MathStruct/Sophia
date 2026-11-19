# An import, then a method: extends `Base.maximum`, which `call_max`
# already used.
import Base: maximum
maximum(p::Point) = p.x
