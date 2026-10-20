//! Prints the tree-sitter S-expression of a snippet (for writing resolvers).
fn main() {
    let lang = std::env::args().nth(1).unwrap();
    let text = std::fs::read_to_string(std::env::args().nth(2).unwrap()).unwrap();
    let tree = moonkale_content_id::parse(&lang, &text, &mut moonkale_content_id::Budget::new()).unwrap();
    println!("{}", tree.root_node().to_sexp());
}
