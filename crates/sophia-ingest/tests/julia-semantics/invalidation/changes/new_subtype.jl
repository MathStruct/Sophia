# A new subtype with its own method: calls on the abstract `Shape` may reach it,
# calls on `Circle` or `Sq` cannot.
struct Tri <: Inval.Shape
    b::Float64
end
Inval.area(t::Tri) = t.b / 2
