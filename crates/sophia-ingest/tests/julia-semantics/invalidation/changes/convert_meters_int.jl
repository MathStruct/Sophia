# A `Type{…}` argument: `convert(Meters, ::Int)` is new, the `Float64` call is not affected.
Base.convert(::Type{Inval.Meters}, x::Int) = Inval.Meters(x)
