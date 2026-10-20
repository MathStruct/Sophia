struct Point
    x::Float64
    y::Float64
end

norm2(p::Point) = p.x * p.x + p.y * p.y

function dist(a::Point, b::Point)
    d = Point(a.x - b.x, a.y - b.y)
    sqrt(norm2(d))
end

function iseven2(n)
    if n == 0
        return true
    end
    isodd2(n - 1)
end

function isodd2(n)
    if n == 0
        return false
    end
    iseven2(n - 1)
end

# Same shape as `dist`, different names and spacing: same id.
function distance(p::Point,  q::Point)
    v = Point(p.x - q.x, p.y - q.y)
    sqrt(norm2(v))
end
