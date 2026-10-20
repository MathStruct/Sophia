struct Point
    x::Float64
    y::Float64
end
origin() = Point(0.0, 0.0)
struct Wrap
    parent::Point
end
parent(w) = w.parent
Base.show(io::IO, w::Wrap) = print(io, w.parent)
const SCALE = 2.0
