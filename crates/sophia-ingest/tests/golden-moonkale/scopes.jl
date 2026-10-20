struct Band
    band::Int
    index::Int
end

offset(b::Band) = b.band + b.index

function total(xs; scale=1, rest...)
    s = 0
    for (i, x) in enumerate(xs), j in 1:i
        s += x * j * scale
    end
    s
end

function widths(bands::Band...)
    map(bands) do b
        (; band, index) = b
        band - index
    end
end

function shifted(bs, ::Type{T}) where {T}
    m, n = size(bs)
    [T(offset(b)) for b in bs if offset(b) > m + n]
end

function guarded(b::Band)
    try
        let k = offset(b)
            x -> x + k
        end
    catch err
        err
    finally
        nothing
    end
end

function countdown(n)
    while n > 0
        n -= 1
    end
    n
end
