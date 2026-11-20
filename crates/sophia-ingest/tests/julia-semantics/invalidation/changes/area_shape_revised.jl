# Replaces `area(s::Shape)` at its own file and line, as Revise does after an
# edit: the new method has the old one's location.
let file = joinpath(@__DIR__, "..", "Inval.jl")
    line = findfirst(l -> startswith(l, "area(s::Shape)"), readlines(file))
    include_string(Inval, "\n"^(line - 1) * "area(s::Shape) = 5.0", file)
end
