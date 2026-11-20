# A loose equality method, less specific in its second argument.
struct Tag
    s::Symbol
end
Base.:(==)(a::Tag, b) = false
