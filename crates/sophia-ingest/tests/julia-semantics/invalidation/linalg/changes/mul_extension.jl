# A package's own five-argument `mul!` for its matrix type.
struct MyOp <: AbstractMatrix{Float64}
    n::Int
end
Base.size(m::MyOp) = (m.n, m.n)
Base.getindex(m::MyOp, i::Int, j::Int) = 0.0
LinearAlgebra.mul!(C::AbstractVecOrMat, A::MyOp, B::AbstractVecOrMat, α::Number, β::Number) = C
