The incoming context tool does not return the advertised callers or dependants because it traverses incoming edges from the name rather than its definition. Existing tests exercise only outgoing context.

Review comment:

- [P2] Resolve the definition before traversing incoming references — /home/daniel/Code/CategoryTheory/Sophia/crates/sophia-mcp/src/sophia_mcp.rs:192-196
  For `context` with `direction: "in"`, traversal starts at a name node, whose incoming edge is `Contains` from its namespace; the `Names` edge to its definition is outgoing. Consequently, this tool walks namespace ancestors instead of finding definitions that reference the requested definition. Resolve the name's target first and traverse incoming references from that content node, retaining the name in the response. Add an incoming-direction test using `sample.add`, which should include `sample.add_test`.