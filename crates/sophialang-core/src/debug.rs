//! `Debug` output without recursion (SP-015): a value describes one level of
//! itself as a [`DNode`], children left unexpanded, and [`render`] walks the
//! description with an explicit stack, in the layout of `#[derive(Debug)]`
//! for both `{:?}` and `{:#?}`. See `debug.md`.

use std::fmt;

/// One level of a value's `Debug` form.
pub enum DNode<'a, T> {
    /// Already formatted (in the requested mode).
    Leaf(String),
    /// `Name(a, b)`; with no fields, just `Name`.
    Tuple(&'static str, Vec<DNode<'a, T>>),
    /// `Name { a: x, b: y }`.
    Struct(&'static str, Vec<(&'static str, DNode<'a, T>)>),
    /// `[a, b]`.
    List(Vec<DNode<'a, T>>),
    /// A value of the recursive type, expanded when reached.
    Child(&'a T),
}

/// A leaf in the current mode.
pub fn leaf<V: fmt::Debug>(v: &V, alt: bool) -> String {
    if alt { format!("{v:#?}") } else { format!("{v:?}") }
}

/// `Some(child)` or `None`, as derive prints an `Option`.
pub fn opt<'a, T>(o: Option<&'a T>) -> DNode<'a, T> {
    match o {
        Some(c) => DNode::Tuple("Some", vec![DNode::Child(c)]),
        None => DNode::Tuple("None", Vec::new()),
    }
}

enum Item<'a, T> {
    Node(DNode<'a, T>, usize),
    Text(String),
}

/// Writes `root` as derive would, expanding children with `expand`.
pub fn render<'a, T>(f: &mut fmt::Formatter<'_>, root: &'a T, expand: impl Fn(&'a T, bool) -> DNode<'a, T>) -> fmt::Result {
    let alt = f.alternate();
    let mut out = String::new();
    let mut stack = vec![Item::Node(DNode::Child(root), 0)];
    let pad = |n: usize| " ".repeat(n);
    while let Some(it) = stack.pop() {
        let (node, ind) = match it {
            Item::Text(s) => {
                out.push_str(&s);
                continue;
            }
            Item::Node(n, ind) => (n, ind),
        };
        // Pushed in reverse: the last pushed is written first.
        let mut seq: Vec<Item<'a, T>> = Vec::new();
        match node {
            DNode::Child(c) => seq.push(Item::Node(expand(c, alt), ind)),
            DNode::Leaf(s) => {
                // A multi-line leaf is indented like a nested value.
                seq.push(Item::Text(if alt && ind > 0 { s.replace('\n', &format!("\n{}", pad(ind))) } else { s }));
            }
            DNode::Tuple(name, fields) if fields.is_empty() => seq.push(Item::Text(name.to_string())),
            DNode::Tuple(name, fields) => {
                seq.push(Item::Text(format!("{name}(")));
                list(&mut seq, fields.into_iter().map(|x| (None, x)).collect(), ind, alt, ")");
            }
            DNode::Struct(name, fields) => {
                seq.push(Item::Text(if alt { format!("{name} {{") } else { format!("{name} {{ ") }));
                list(&mut seq, fields.into_iter().map(|(k, x)| (Some(k), x)).collect(), ind, alt, "}");
            }
            DNode::List(items) if items.is_empty() => seq.push(Item::Text("[]".into())),
            DNode::List(items) => {
                seq.push(Item::Text("[".into()));
                list(&mut seq, items.into_iter().map(|x| (None, x)).collect(), ind, alt, "]");
            }
        }
        stack.extend(seq.into_iter().rev());
    }
    f.write_str(&out)
}

/// The fields of a tuple, struct or list, then the closing bracket.
fn list<'a, T>(seq: &mut Vec<Item<'a, T>>, fields: Vec<(Option<&'static str>, DNode<'a, T>)>, ind: usize, alt: bool, close: &str) {
    let n = fields.len();
    let struct_close = close == "}";
    for (i, (key, node)) in fields.into_iter().enumerate() {
        let key = key.map(|k| format!("{k}: ")).unwrap_or_default();
        if alt {
            seq.push(Item::Text(format!("\n{}{key}", " ".repeat(ind + 4))));
            seq.push(Item::Node(node, ind + 4));
            seq.push(Item::Text(",".into()));
        } else {
            seq.push(Item::Text(key));
            seq.push(Item::Node(node, ind));
            if i + 1 < n {
                seq.push(Item::Text(", ".into()));
            }
        }
    }
    if alt {
        seq.push(Item::Text(format!("\n{}{close}", " ".repeat(ind))));
    } else if struct_close {
        seq.push(Item::Text(" }".into()));
    } else {
        seq.push(Item::Text(close.into()));
    }
}
