# JuliaSyntax source oracle for parser agreement; see parse.md.
const J = Base.JuliaSyntax
children(n) = something(J.children(n), J.SyntaxNode[])
kind(n) = string(J.kind(n))
firstbyte(n) = first(J.byte_range(n)) - 1
lastbyte(n) = last(J.byte_range(n)) - 1
spelling(n, text) = String(Vector{UInt8}(codeunits(text)[J.byte_range(n)]))
striphead(n) = kind(n) in ("where", "::", "curly", "<:", "quote", "parens") && !isempty(children(n)) ? striphead(children(n)[1]) : n
function nametoken(n)
    c = children(n)
    kind(n) == "." && !isempty(c) && return nametoken(c[end])
    kind(n) in ("quote", "curly", "<:", "parens") && !isempty(c) && return nametoken(c[1])
    n
end
function callparts(n)
    c = children(n)
    if J.is_infix_op_call(n)
        return c[2], [c[1]; c[3:end]]
    end
    c[1], c[2:end]
end
paramname(n, text) = spelling(nametoken(striphead(kind(n) in ("=", "...", "kw") ? children(n)[1] : n)), text)
function callkeywords(n)
    kind(n) in ("function", "macro", "module", "struct", "abstract", "primitive") && return 0
    total = 0
    if kind(n) == "call"
        _, args = callparts(n)
        for a in args
            total += kind(a) == "parameters" ? length(children(a)) : kind(a) == "=" ? 1 : 0
        end
    end
    total + sum(callkeywords, children(n); init=0)
end
function emit(n, file, text)
    k = kind(n)
    c = children(n)
    form = k
    head = isempty(c) ? n : c[1]
    npos, kws = "-", "-"
    bodies = c[2:end]
    if k in ("function", "macro")
        form = k == "macro" ? "macro" : J.flags(n) & J.SHORT_FORM_FUNCTION_FLAG != 0 ? "short" : "function"
        sig = striphead(head)
        if kind(sig) == "call"
            head, args = callparts(sig)
            npos = string(count(a -> kind(a) != "parameters", args))
            names = sort!([paramname(p, text) for a in args if kind(a) == "parameters" for p in children(a)])
            kws = isempty(names) ? "-" : join(names, ',')
        else
            head = sig
            npos = "0"  # function f end: generic declaration
        end
    elseif k == "struct"
        form = J.flags(n) & J.MUTABLE_FLAG != 0 ? "mutable_struct" : "struct"
    elseif k == "="
        lhs = c[1]
        kind(lhs) == "::" && kind(children(lhs)[1]) == "Identifier" || return
        head, form = children(lhs)[1], "typed_global"
    elseif k == "const"
        lhs = kind(head) == "=" ? children(head)[1] : head
        # One row per binding, even in a destructuring const declaration.
        if kind(lhs) == "tuple"
            for h in children(lhs)
                token = nametoken(striphead(h))
                println(join(("def", file, firstbyte(token), firstbyte(n), lastbyte(n), "const", spelling(token,text), "-", "-", callkeywords(n)), '\t'))
            end
            return
        end
        head = striphead(lhs)
        bodies = c
    elseif !(k in ("abstract", "primitive", "module"))
        return
    end
    token = nametoken(head)
    name = kind(head) == "::" ? "-" : spelling(token, text)
    println(join(("def", file, firstbyte(token), firstbyte(n), lastbyte(n), form, name, npos, kws,
                  sum(callkeywords, bodies; init=0)), '\t'))
end
function walk(n, file, text, scope=:module)
    k = kind(n)
    statement = n.parent === nothing || kind(n.parent) in ("block", "toplevel", "macrocall")
    (k != "=" || (scope == :module && statement)) && emit(n, file, text)
    # Keep scope while traversing source wrappers and nested method bodies.
    # Quotes and strings are data; macro expansion is outside this measurement.
    k in ("quote", "inert", "string", "cmdstring", "macro_name", "const") && return
    if k in ("function", "macro")
        foreach(c -> walk(c, file, text, :local), children(n)[2:end])
    else
        nextscope = k == "module" ? :module : k in ("for", "while", "let", "struct", "->", "do", "call", "parameters") ? :local : scope
        foreach(c -> walk(c, file, text, nextscope), children(n))
    end
end
src = abspath(ARGS[1])
files = sort!([joinpath(root, f) for (root, _, fs) in walkdir(src) for f in fs if endswith(f, ".jl")])
for path in files
    text, file = read(path, String), relpath(path, src)
    tree = J.parseall(J.SyntaxNode, text; filename=path, ignore_errors=true)
    stream = J.ParseStream(text)
    J.parse!(stream; rule=:all)
    for d in stream.diagnostics
        d.level == :error && println(join(("error", file, d.first_byte - 1, d.last_byte - 1), '\t'))
    end
    walk(tree, file, text)
end
