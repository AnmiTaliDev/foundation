// SPDX-License-Identifier: GPL-3.0-only

use foundation::dom::{Node, NodeType};
use foundation::html;

fn shape(node: &Node) -> String {
    match &node.node_type {
        NodeType::Text(_) => "t".to_string(),
        NodeType::Element(element) => {
            let children: Vec<String> = node.children.iter().map(shape).collect();
            if children.is_empty() {
                element.tag_name.clone()
            } else {
                format!("{}({})", element.tag_name, children.join(","))
            }
        }
    }
}

fn parse(body: &str) -> String {
    let root = html::parse(&format!("<html><body>{}</body></html>", body));
    shape(&root.children[0])
}

#[test]
fn caption_ends_at_the_first_row() {
    assert_eq!(
        parse("<table><caption>c<tr><td>a</table>"),
        "body(table(caption(t),tbody(tr(td(t)))))"
    );
}

#[test]
fn caption_ends_at_a_section() {
    assert_eq!(
        parse("<table><caption>c<tbody><tr><td>a</table>"),
        "body(table(caption(t),tbody(tr(td(t)))))"
    );
}

#[test]
fn colgroup_ends_at_the_first_row() {
    assert_eq!(
        parse("<table><colgroup><col><col><tr><td>a</table>"),
        "body(table(colgroup(col,col),tbody(tr(td(t)))))"
    );
}

#[test]
fn colgroup_ends_at_a_caption() {
    assert_eq!(
        parse("<table><colgroup><col><caption>c<tr><td>a</table>"),
        "body(table(colgroup(col),caption(t),tbody(tr(td(t)))))"
    );
}

#[test]
fn nested_table_inside_a_caption_keeps_its_rows() {
    assert_eq!(
        parse("<table><caption><table><tr><td>a</table>c<tr><td>b</table>"),
        "body(table(caption(table(tbody(tr(td(t)))),t),tbody(tr(td(t)))))"
    );
}

#[test]
fn explicit_end_tags_still_work() {
    assert_eq!(
        parse("<table><caption>c</caption><tr><td>a</td></tr></table>"),
        "body(table(caption(t),tbody(tr(td(t)))))"
    );
}
