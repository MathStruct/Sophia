//! Recursive groups: strongly connected components of the reference graph,
//! and the canonical order of a component's members. See `component.md`.
//!
//! The ordering is Moonkale's WP-66 normaliser v1 rule (Code Graph Schema,
//! "WP-66 normalizer v1 → Recursive components"):
//!
//! - For each candidate root, a first-visit traversal over references in
//!   stream order assigns member indices.
//! - The lexicographically smallest complete serialisation wins.
//! - If several roots tie, each member takes its smallest index across them.
//!   Members that share it are related by an automorphism of the group, so
//!   they compute the same thing; they become **one class** (the quotient).
//!   Classes are numbered densely in order of that index, and the component
//!   is serialised over its classes: `f() = g(); g() = f()` is stored as one
//!   member, identical to `h() = h()` (review of 2026-10-20, P0).
//! - All candidates share one work budget.

/// Strongly connected components of a graph with nodes `0..n`, in
/// **dependency-first** order: every component comes after all components it
/// references. Within a component, nodes are listed in ascending order.
///
/// `edges(v)` lists the nodes `v` references (duplicates allowed). Iterative
/// Tarjan, so deep graphs cannot overflow the stack.
pub fn strongly_connected<F, I>(n: usize, mut edges: F) -> Vec<Vec<usize>>
where
    F: FnMut(usize) -> I,
    I: IntoIterator<Item = usize>,
{
    const UNVISITED: usize = usize::MAX;
    let adj: Vec<Vec<usize>> = (0..n).map(|v| edges(v).into_iter().collect()).collect();
    let mut index = vec![UNVISITED; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut next = 0usize;
    // (node, position in its adjacency list)
    let mut call: Vec<(usize, usize)> = Vec::new();

    for root in 0..n {
        if index[root] != UNVISITED {
            continue;
        }
        call.push((root, 0));
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on_stack[root] = true;

        while let Some(&mut (v, ref mut pos)) = call.last_mut() {
            if *pos < adj[v].len() {
                let w = adj[v][*pos];
                *pos += 1;
                if index[w] == UNVISITED {
                    index[w] = next;
                    low[w] = next;
                    next += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                call.pop();
                if let Some(&(parent, _)) = call.last() {
                    low[parent] = low[parent].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut comp = Vec::new();
                    loop {
                        let w = stack.pop().expect("tarjan stack");
                        on_stack[w] = false;
                        comp.push(w);
                        if w == v {
                            break;
                        }
                    }
                    comp.sort_unstable();
                    out.push(comp);
                }
            }
        }
    }
    out
}

/// A shared work budget, in serialised bytes, for canonical ordering.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    pub remaining: u64,
}

impl Default for Budget {
    fn default() -> Self {
        Budget::DEFAULT
    }
}

impl Budget {
    pub const DEFAULT: Budget = Budget { remaining: 1 << 28 };

    fn spend(&mut self, n: usize) -> Result<(), OrderError> {
        self.remaining = self.remaining.checked_sub(n as u64).ok_or(OrderError::BudgetExhausted)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrderError {
    #[error("canonical ordering exceeded its work budget")]
    BudgetExhausted,
    #[error("member {0} is not reachable from the root: not a strongly connected component")]
    NotStronglyConnected(usize),
}

/// The canonical order of one component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ordered {
    /// The canonical class of each member, by position in the input slice.
    /// Automorphic members share a class.
    pub index: Vec<u32>,
    /// Each member's canonical bytes, with references to members written as
    /// their classes, by position in the input slice. Members of one class
    /// have identical bytes.
    pub bytes: Vec<Vec<u8>>,
    /// The concatenation of one member's bytes per class, in class order:
    /// the component's serialisation.
    pub serialisation: Vec<u8>,
    /// The number of classes: the component's size as stored.
    pub classes: u32,
}

/// Orders the members `0..n` of one component.
///
/// - `refs(m)` lists, **in stream order**, the members that member `m`
///   references (indices into `0..n`).
/// - `encode(m, index)` writes member `m`'s canonical bytes, with every
///   reference to a member `k` written as `index[k]`.
pub fn canonical_order<R, E>(
    n: usize,
    refs: R,
    encode: E,
    budget: &mut Budget,
) -> Result<Ordered, OrderError>
where
    R: Fn(usize) -> Vec<usize>,
    E: Fn(usize, &[u32]) -> Vec<u8>,
{
    assert!(n > 0, "a component has at least one member");
    if n == 1 {
        let bytes = encode(0, &[0]);
        budget.spend(bytes.len())?;
        return Ok(Ordered { index: vec![0], serialisation: bytes.clone(), bytes: vec![bytes], classes: 1 });
    }

    let refs: Vec<Vec<usize>> = (0..n).map(&refs).collect();
    let mut best: Option<(Vec<u8>, Vec<Vec<u8>>)> = None;
    let mut ties: Vec<Vec<u32>> = Vec::new();

    for root in 0..n {
        // First-visit traversal in stream order (breadth-first by member,
        // stream order within a member).
        let mut index = vec![u32::MAX; n];
        let mut order = Vec::with_capacity(n);
        index[root] = 0;
        order.push(root);
        let mut head = 0;
        while head < order.len() {
            let m = order[head];
            head += 1;
            for &t in &refs[m] {
                if index[t] == u32::MAX {
                    index[t] = order.len() as u32;
                    order.push(t);
                }
            }
        }
        if let Some(missing) = index.iter().position(|&i| i == u32::MAX) {
            return Err(OrderError::NotStronglyConnected(missing));
        }

        let bytes: Vec<Vec<u8>> = (0..n).map(|m| encode(m, &index)).collect();
        let mut ser = Vec::new();
        for &m in &order {
            ser.extend_from_slice(&bytes[m]);
        }
        budget.spend(ser.len())?;

        match &best {
            Some((b, _)) if ser > *b => {}
            Some((b, _)) if ser == *b => ties.push(index),
            _ => {
                ties.clear();
                ties.push(index);
                best = Some((ser, bytes));
            }
        }
    }

    let (serialisation, bytes) = best.expect("n > 0");
    let min_index: Vec<u32> = (0..n).map(|m| ties.iter().map(|ix| ix[m]).min().expect("one root")).collect();
    let mut distinct: Vec<u32> = min_index.clone();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() == n {
        // No symmetry: the winning traversal is the canonical order.
        return Ok(Ordered { index: min_index, bytes, serialisation, classes: n as u32 });
    }
    // The quotient: dense class numbers, and the serialisation over classes.
    let class: Vec<u32> = min_index.iter().map(|i| distinct.binary_search(i).unwrap() as u32).collect();
    let bytes: Vec<Vec<u8>> = (0..n).map(|m| encode(m, &class)).collect();
    let mut serialisation = Vec::new();
    for c in 0..distinct.len() as u32 {
        let rep = class.iter().position(|&k| k == c).expect("every class has a member");
        serialisation.extend_from_slice(&bytes[rep]);
    }
    budget.spend(serialisation.len())?;
    Ok(Ordered { index: class, bytes, serialisation, classes: distinct.len() as u32 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scc(n: usize, edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
        strongly_connected(n, |v| edges.iter().filter(move |e| e.0 == v).map(|e| e.1).collect::<Vec<_>>())
    }

    #[test]
    fn components_come_dependency_first() {
        // 0 -> 1 <-> 2 -> 3
        let c = scc(4, &[(0, 1), (1, 2), (2, 1), (2, 3)]);
        assert_eq!(c, vec![vec![3], vec![1, 2], vec![0]]);
    }

    #[test]
    fn self_loop_is_its_own_component() {
        assert_eq!(scc(2, &[(0, 0), (1, 0)]), vec![vec![0], vec![1]]);
    }

    #[test]
    fn deep_chain_does_not_overflow() {
        let n = 200_000;
        let c = strongly_connected(n, |v| if v + 1 < n { vec![v + 1] } else { vec![] });
        assert_eq!(c.len(), n);
        assert_eq!(c[0], vec![n - 1]);
    }

    /// A toy member: a label and the members it references, in order.
    fn order_of(labels: &[&str], refs: &[Vec<usize>]) -> Ordered {
        let encode = |m: usize, ix: &[u32]| {
            let mut v = labels[m].as_bytes().to_vec();
            for &t in &refs[m] {
                v.push(b'@');
                v.extend_from_slice(&ix[t].to_le_bytes());
            }
            v
        };
        canonical_order(labels.len(), |m| refs[m].clone(), encode, &mut Budget::default()).unwrap()
    }

    #[test]
    fn order_does_not_depend_on_input_order() {
        // even/odd-like pair, given in both orders.
        let a = order_of(&["even", "odd"], &[vec![1], vec![0]]);
        let b = order_of(&["odd", "even"], &[vec![1], vec![0]]);
        assert_eq!(a.serialisation, b.serialisation);
        assert_eq!(a.index, vec![b.index[1], b.index[0]]);
        assert_ne!(a.index[0], a.index[1]);
    }

    #[test]
    fn automorphic_members_form_one_class() {
        // Two identical members referencing each other: the quotient is one
        // member referencing itself.
        let o = order_of(&["f", "f"], &[vec![1], vec![0]]);
        assert_eq!(o.index, vec![0, 0]);
        assert_eq!(o.classes, 1);
        let alone = order_of(&["f"], &[vec![0]]);
        assert_eq!(o.serialisation, alone.serialisation, "f→g→f is h→h");
    }

    #[test]
    fn a_symmetric_triangle_collapses_and_an_asymmetric_one_does_not() {
        let sym = order_of(&["f", "f", "f"], &[vec![1], vec![2], vec![0]]);
        assert_eq!((sym.index.clone(), sym.classes), (vec![0, 0, 0], 1));
        let asym = order_of(&["f", "g", "f"], &[vec![1], vec![2], vec![0]]);
        assert_eq!(asym.classes, 3);
        let mut seen = asym.index.clone();
        seen.sort();
        assert_eq!(seen, vec![0, 1, 2], "a permutation");
    }

    #[test]
    fn partial_symmetry_keeps_the_rest_distinct() {
        // a ↔ b symmetric, both calling c, c calling a: a and b are one class.
        let o = order_of(&["s", "s", "c"], &[vec![1, 2], vec![0, 2], vec![0]]);
        assert_eq!(o.classes, 3, "c reaches a but not b, so a and b differ structurally");
        let o2 = order_of(&["s", "s"], &[vec![1, 1], vec![0, 0]]);
        assert_eq!(o2.classes, 1);
    }

    #[test]
    fn bisimilar_members_that_are_not_automorphic_stay_apart() {
        // a = s(b, c), b = s(a, a), c = s(a, a) unfolds to the same infinite
        // tree as h = s(h, h), but swapping b and c changes a's bytes, so no
        // automorphism relates them: the quotient is by automorphism, not by
        // bisimilarity (RQ-001).
        let three = order_of(&["s", "s", "s"], &[vec![1, 2], vec![0, 0], vec![0, 0]]);
        let alone = order_of(&["s"], &[vec![0, 0]]);
        assert_eq!((three.classes, alone.classes), (3, 1));
        assert_ne!(three.serialisation, alone.serialisation);
    }

    #[test]
    fn budget_is_enforced() {
        let mut tiny = Budget { remaining: 3 };
        let r = canonical_order(2, |m| vec![1 - m], |_, _| vec![0; 8], &mut tiny);
        assert_eq!(r, Err(OrderError::BudgetExhausted));
    }
}
