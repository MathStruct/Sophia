# A method of a Base function for the module's type: `length(::Vector{Int})` is not affected.
Base.length(c::Inval.Circle) = 1
