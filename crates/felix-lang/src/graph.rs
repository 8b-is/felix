//! `felix graph` — the dataflow a checked program actually has.
//!
//! One node per statement, one edge per name that flows from a `let` into a
//! later statement. Linear bindings are drawn with a double border and their
//! single edge is the `consume`, so the picture is the linear rule's proof:
//! a linear node with anything other than exactly one bold out-edge cannot be
//! drawn, because it would not have checked.
//!
//! Two renderings of the same graph: Graphviz DOT, and Mermaid for places
//! (GitHub, the constellation docs) that render it inline. Output is
//! deterministic; the golden files hold it.

use std::collections::HashMap;

use crate::ast::{Expr, Program, Stmt};
use crate::check::Checked;

/// A node: what to call it and how to draw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub label: String,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Let,
    LinearLet,
    Print,
    Consume,
}

/// An edge from the statement that binds a name to one that uses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    /// Uses of the name in the target statement (`a <~> a` is two).
    pub uses: usize,
    pub consume: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

/// Build the graph of a program that has already passed [`crate::check::check`].
pub fn build(program: &Program, checked: &Checked) -> Graph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut bound_at: HashMap<&str, usize> = HashMap::new();

    for (index, statement) in program.statements.iter().enumerate() {
        let (node, value, consume) = match statement {
            Stmt::Let {
                name,
                linear,
                value,
                ..
            } => {
                let ty = checked
                    .type_of(name)
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "?".into());
                let node = Node {
                    label: format!(
                        "{}let {name}: {ty} = {}",
                        if *linear { "linear " } else { "" },
                        value.summary()
                    ),
                    kind: if *linear {
                        NodeKind::LinearLet
                    } else {
                        NodeKind::Let
                    },
                };
                (node, value, false)
            }
            Stmt::Print { value, .. } => (
                Node {
                    label: format!("print({})", value.summary()),
                    kind: NodeKind::Print,
                },
                value,
                false,
            ),
            Stmt::Consume { value, .. } => (
                Node {
                    label: format!("consume({})", value.summary()),
                    kind: NodeKind::Consume,
                },
                value,
                true,
            ),
        };

        // Names in first-use order, with their counts.
        let mut names: Vec<(&str, usize)> = Vec::new();
        collect(value, &mut names);
        for (name, uses) in names {
            if let Some(&from) = bound_at.get(name) {
                edges.push(Edge {
                    from,
                    to: index,
                    uses,
                    consume,
                });
            }
        }

        if let Stmt::Let { name, .. } = statement {
            bound_at.insert(name.as_str(), index);
        }
        nodes.push(node);
    }
    Graph { nodes, edges }
}

fn collect<'a>(expr: &'a Expr, names: &mut Vec<(&'a str, usize)>) {
    match expr {
        Expr::Int { .. } | Expr::Ratio { .. } => {}
        Expr::Var { name, .. } => match names.iter_mut().find(|(n, _)| n == name) {
            Some((_, count)) => *count += 1,
            None => names.push((name, 1)),
        },
        Expr::List { items, .. } | Expr::Call { args: items, .. } => {
            items.iter().for_each(|item| collect(item, names))
        }
        Expr::Contract { left, right, .. } => {
            collect(left, names);
            collect(right, names);
        }
    }
}

impl Graph {
    /// Graphviz DOT.
    pub fn to_dot(&self) -> String {
        let mut out = String::from("digraph felix {\n  node [fontname=\"monospace\"];\n");
        for (i, node) in self.nodes.iter().enumerate() {
            let shape = match node.kind {
                NodeKind::Let => "shape=box",
                NodeKind::LinearLet => "shape=box, peripheries=2",
                NodeKind::Print => "shape=note",
                NodeKind::Consume => "shape=octagon",
            };
            let label = node.label.replace('\\', "\\\\").replace('"', "\\\"");
            out.push_str(&format!("  s{i} [{shape}, label=\"{label}\"];\n"));
        }
        for edge in &self.edges {
            let mut attrs = Vec::new();
            if edge.consume {
                attrs.push("style=bold".to_string());
                attrs.push("label=\"consume\"".to_string());
            } else if edge.uses > 1 {
                attrs.push(format!("label=\"×{}\"", edge.uses));
            }
            let attrs = if attrs.is_empty() {
                String::new()
            } else {
                format!(" [{}]", attrs.join(", "))
            };
            out.push_str(&format!("  s{} -> s{}{attrs};\n", edge.from, edge.to));
        }
        out.push_str("}\n");
        out
    }

    /// Mermaid `flowchart`.
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("flowchart TD\n");
        for (i, node) in self.nodes.iter().enumerate() {
            let label = node
                .label
                .replace('"', "#quot;")
                .replace('<', "#lt;")
                .replace('>', "#gt;");
            let (open, close) = match node.kind {
                NodeKind::Let => ("[\"", "\"]"),
                NodeKind::LinearLet => ("[[\"", "\"]]"),
                NodeKind::Print => ("([\"", "\"])"),
                NodeKind::Consume => ("{{\"", "\"}}"),
            };
            out.push_str(&format!("  s{i}{open}{label}{close}\n"));
        }
        for edge in &self.edges {
            let arrow = if edge.consume {
                "==>|consume|".to_string()
            } else if edge.uses > 1 {
                format!("-->|×{}|", edge.uses)
            } else {
                "-->".to_string()
            };
            out.push_str(&format!("  s{} {arrow} s{}\n", edge.from, edge.to));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{check, parser};

    fn graph(source: &str) -> Graph {
        let program = parser::parse(source).unwrap();
        let checked = check::check(&program).unwrap();
        build(&program, &checked)
    }

    #[test]
    fn edges_follow_names_and_count_repeated_uses() {
        let g = graph("let a = pack([1], 1); let b = 2; print(a <~> a); print(b);");
        assert_eq!(
            g.edges,
            vec![
                Edge {
                    from: 0,
                    to: 2,
                    uses: 2,
                    consume: false
                },
                Edge {
                    from: 1,
                    to: 3,
                    uses: 1,
                    consume: false
                },
            ]
        );
        assert_eq!(g.nodes[0].label, "let a: Word = pack([1], 1)");
    }

    #[test]
    fn a_linear_binding_has_exactly_one_edge_and_it_is_the_consume() {
        let g = graph("linear let v = new_vault(); let n = 1; print(n); consume(v);");
        let from_v: Vec<&Edge> = g.edges.iter().filter(|e| e.from == 0).collect();
        assert_eq!(from_v.len(), 1);
        assert!(from_v[0].consume);
        assert_eq!(g.nodes[0].kind, NodeKind::LinearLet);
    }

    #[test]
    fn dot_and_mermaid_render_the_same_graph() {
        let g = graph("let w = pack([1], 1/2); print(w <~> w);");
        assert_eq!(
            g.to_dot(),
            "digraph felix {\n  node [fontname=\"monospace\"];\n  \
             s0 [shape=box, label=\"let w: Word = pack([1], 1/2)\"];\n  \
             s1 [shape=note, label=\"print(w <~> w)\"];\n  \
             s0 -> s1 [label=\"×2\"];\n}\n"
        );
        assert_eq!(
            g.to_mermaid(),
            "flowchart TD\n  s0[\"let w: Word = pack([1], 1/2)\"]\n  \
             s1([\"print(w #lt;~#gt; w)\"])\n  s0 -->|×2| s1\n"
        );
    }

    #[test]
    fn long_lists_are_elided_in_labels() {
        let g = graph("let w = pack([1, 2, 3, 4, 5], 1);");
        assert_eq!(g.nodes[0].label, "let w: Word = pack([…5], 1)");
    }
}
