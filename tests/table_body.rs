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
fn rows_get_a_body() {
    assert_eq!(
        parse("<table><tr><td>a<tr><td>b</table>"),
        "body(table(tbody(tr(td(t)),tr(td(t)))))"
    );
}

#[test]
fn cells_get_a_body_and_a_row() {
    assert_eq!(
        parse("<table><td>a<td>b</table>"),
        "body(table(tbody(tr(td(t),td(t)))))"
    );
}

#[test]
fn cells_in_a_section_get_a_row() {
    assert_eq!(
        parse("<table><thead><th>a<th>b</thead></table>"),
        "body(table(thead(tr(th(t),th(t)))))"
    );
}

#[test]
fn explicit_sections_are_kept() {
    assert_eq!(
        parse("<table><thead><tr><td>a</thead><tbody><tr><td>b</tbody></table>"),
        "body(table(thead(tr(td(t))),tbody(tr(td(t)))))"
    );
}

#[test]
fn rows_after_a_section_start_a_new_body() {
    assert_eq!(
        parse("<table><thead><tr><td>a</thead><tr><td>b</table>"),
        "body(table(thead(tr(td(t))),tbody(tr(td(t)))))"
    );
}

#[test]
fn nested_tables_get_their_own_body() {
    assert_eq!(
        parse("<table><tr><td><table><tr><td>a</table></table>"),
        "body(table(tbody(tr(td(table(tbody(tr(td(t)))))))))"
    );
}

#[test]
fn rows_outside_tables_are_left_alone() {
    assert_eq!(parse("<div><tr><td>a</div>"), "body(div(tr(td(t))))");
}
