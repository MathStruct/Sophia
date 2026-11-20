# LinearAlgebra with a small workload compiled on top of its package image;
# `invalidation.jl` records the compiled instances of LinearAlgebra's methods.
using LinearAlgebra
let A = rand(4, 4), B = rand(4, 4), b = rand(4), S = A + A'
    A * B; A * b; A \ b; A'; transpose(A); dot(b, b); norm(b); norm(A); det(A); inv(A); tr(A)
    lu(A); qr(A); cholesky(S + 4I); eigen(S); svd(A); kron(A, B); A + 2I
    Diagonal(b) * A; Symmetric(S) * b; UpperTriangular(A) \ b; mul!(similar(A), A, B)
    axpy!(2.0, b, copy(b)); normalize(b); cross(b[1:3], b[1:3]); rank(A); pinv(A)
end
const TARGET = LinearAlgebra
