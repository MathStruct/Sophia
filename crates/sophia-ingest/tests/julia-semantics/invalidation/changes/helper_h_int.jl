# A method inside the function an `invoke` call reaches: `use_helper` relies on it
# only through the instance `Helper.g(::Int64)`, reached by `invoke`.
Main.Helper.h(x::Int) = 2
