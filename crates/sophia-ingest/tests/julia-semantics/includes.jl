# Ground truth from Julia itself for part 9 of the conformance suite
# (include order): which files Julia evaluates when it loads the package,
# in which order and into which module, and where each method it then holds
# was defined. Compared with Sophia's include discovery (`module::discover`)
# by `cargo run -p sophia-ingest --example julia_includes` (and
# `tests/includes.rs` on the fixture). See includes.md.
#
# Usage: julia includes.jl PKG SRCDIR
#
# Prints two kinds of line, tab-separated:
#   include  seq  Module  file
#     one per file Julia evaluates, in the order it starts evaluating them
#     (`Base.include_callbacks` run before a file's code; seq counts from
#     0, the root file); Module is the module the file's code runs in, the
#     package's name for the root module, `-` for the root file (evaluated
#     in Main); file is relative to SRCDIR, or `outside:<absolute path>`;
#   method  file  line  Module  name
#     one per (file, line, module, function name) of a method of a file
#     Julia evaluated above (file written as there) that `which` still
#     dispatches to: where Julia says it was defined (`m.file`, `m.line`)
#     and the module it was defined in (`m.module`, `-` for Main).
pkg = ARGS[1]
srcdir = rstrip(abspath(ARGS[2]), '/') * "/"

name_of(m::Module) = join(string.(filter(!=(:Main), collect(fullname(m)))), ".")
relfile(file) = startswith(file, srcdir) ? String(chopprefix(file, srcdir)) : nothing
key(file) = something(relfile(file), "outside:" * file)

events = Tuple{Module,String}[]
push!(Base.include_callbacks, (mod, path) -> push!(events, (mod, String(path))))
include(joinpath(srcdir, pkg * ".jl"))

for (i, (mod, path)) in enumerate(events)
    m = mod === Main ? "-" : name_of(mod)
    println(join(("include", i - 1, m, key(path)), '\t'))
end

current(m) = try
    Base.which(m.sig) === m
catch
    true
end

# Methods of the files Julia evaluated, also those outside SRCDIR.
evaluated = Set(path for (_, path) in events)
seen = Set{Tuple{String,Int,String,String}}()
Base.visit(Core.methodtable) do m
    String(m.file) in evaluated || return
    file = key(String(m.file))
    current(m) || return
    row = (file, Int(m.line), m.module === Main ? "-" : name_of(m.module), string(m.name))
    row in seen && return
    push!(seen, row)
    println(join(("method", row...), '\t'))
end
