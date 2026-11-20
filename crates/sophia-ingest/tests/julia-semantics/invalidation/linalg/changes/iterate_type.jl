# A new iterator type.
struct Count3 end
Base.iterate(::Count3, s = 1) = s > 3 ? nothing : (s, s + 1)
Base.length(::Count3) = 3
