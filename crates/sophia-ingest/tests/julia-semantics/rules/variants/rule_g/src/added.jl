# A rule for `g`, counting its calls (the in-session check reads the count).
const G_RULE_CALLS = Ref(0)
function ChainRulesCore.rrule(::typeof(g), x)
    G_RULE_CALLS[] += 1
    return g(x), dy -> (NoTangent(), dy * (cos(x) * x + sin(x)))
end
