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
fn paragraph_ends_at_the_next_paragraph() {
    assert_eq!(parse("<p>a<p>b"), "body(p(t),p(t))");
}

#[test]
fn paragraph_ends_at_blocks() {
    assert_eq!(parse("<p>a<div>b</div>"), "body(p(t),div(t))");
    assert_eq!(parse("<p>a<ul><li>b</ul>"), "body(p(t),ul(li(t)))");
    assert_eq!(parse("<p>a<h1>b</h1>"), "body(p(t),h1(t))");
}

#[test]
fn paragraph_closes_through_inline_elements() {
    assert_eq!(parse("<p><b>a<div>b</div>"), "body(p(b(t)),div(t))");
}

#[test]
fn paragraph_stays_open_for_inline_content() {
    assert_eq!(parse("<p>a<span>b</span>c"), "body(p(t,span(t),t))");
}

#[test]
fn list_items_close_each_other() {
    assert_eq!(
        parse("<ul><li>a<li>b<li>c</ul>"),
        "body(ul(li(t),li(t),li(t)))"
    );
}

#[test]
fn nested_lists_keep_their_items() {
    assert_eq!(
        parse("<ul><li>a<ul><li>b<li>c</ul><li>d</ul>"),
        "body(ul(li(t,ul(li(t),li(t))),li(t)))"
    );
}

#[test]
fn paragraph_inside_an_item_ends_with_it() {
    assert_eq!(parse("<ul><li><p>a<li>b</ul>"), "body(ul(li(p(t)),li(t)))");
}

#[test]
fn definition_terms_and_descriptions_close_each_other() {
    assert_eq!(
        parse("<dl><dt>a<dd>b<dt>c<dd>d</dl>"),
        "body(dl(dt(t),dd(t),dt(t),dd(t)))"
    );
}

#[test]
fn table_cells_and_rows_close_implicitly() {
    assert_eq!(
        parse("<table><tr><td>a<td>b<tr><td>c</table>"),
        "body(table(tbody(tr(td(t),td(t)),tr(td(t)))))"
    );
    assert_eq!(
        parse("<table><tr><th>a<th>b<tr><td>c</table>"),
        "body(table(tbody(tr(th(t),th(t)),tr(td(t)))))"
    );
}

#[test]
fn table_sections_close_each_other() {
    assert_eq!(
        parse("<table><thead><tr><td>a<tbody><tr><td>b<tfoot><tr><td>c</table>"),
        "body(table(thead(tr(td(t))),tbody(tr(td(t))),tfoot(tr(td(t)))))"
    );
}

#[test]
fn nested_tables_are_not_closed() {
    assert_eq!(
        parse("<table><tr><td><table><tr><td>a</table><td>b</table>"),
        "body(table(tbody(tr(td(table(tbody(tr(td(t))))),td(t)))))"
    );
}

#[test]
fn paragraph_inside_a_cell_does_not_close_outside() {
    assert_eq!(
        parse("<p>x<table><tr><td><p>a<td>b</table>"),
        "body(p(t),table(tbody(tr(td(p(t)),td(t)))))"
    );
}

#[test]
fn options_close_each_other() {
    assert_eq!(
        parse("<select><option>a<option>b<optgroup><option>c<optgroup><option>d</select>"),
        "body(select(option(t),option(t),optgroup(option(t)),optgroup(option(t))))"
    );
}

#[test]
fn explicit_end_tags_still_work() {
    assert_eq!(parse("<p>a</p><p>b</p>"), "body(p(t),p(t))");
    assert_eq!(
        parse("<ul><li>a</li><li>b</li></ul>"),
        "body(ul(li(t),li(t)))"
    );
}
