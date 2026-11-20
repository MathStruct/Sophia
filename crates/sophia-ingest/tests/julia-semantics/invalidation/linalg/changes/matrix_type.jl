# A new matrix type with the array interface, as a structured-matrix package defines.
struct MyMat <: AbstractMatrix{Float64}
    n::Int
end
Base.size(m::MyMat) = (m.n, m.n)
Base.getindex(m::MyMat, i::Int, j::Int) = Float64(i == j)
