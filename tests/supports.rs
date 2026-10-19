// SPDX-License-Identifier: GPL-3.0-only

use foundation::css;
use foundation::supports::evaluate;

#[test]
fn declarations_are_tested_against_what_the_engine_implements() {
    assert!(evaluate("(display: flex)"));
    assert!(evaluate("(position: absolute)"));
    assert!(evaluate("(width: calc(100% - 10px))"));
    assert!(evaluate("(gap: 4px)"));
    assert!(evaluate("(background: url(a.png) no-repeat)"));
    assert!(!evaluate("(display: grid)"));
    assert!(evaluate("(position: sticky)"));
    assert!(!evaluate("(position: static-ish)"));
    assert!(evaluate("(transform: rotate(1deg))"));
    assert!(evaluate("(opacity: 0.5)"));
    assert!(evaluate("(font-family: serif)"));
    assert!(!evaluate("(backdrop-filter: blur(2px))"));
    assert!(!evaluate("(transform: rotate(foo))"));
    assert!(!evaluate("(width: ???)"));
}

#[test]
fn conditions_combine() {
    assert!(evaluate("(display: flex) and (gap: 4px)"));
    assert!(!evaluate("(display: flex) and (display: grid)"));
    assert!(evaluate("(display: grid) or (display: flex)"));
    assert!(evaluate("not (display: grid)"));
    assert!(!evaluate("not (display: flex)"));
    assert!(evaluate("((display: flex) and (gap: 4px))"));
    assert!(evaluate("(not (display: grid))"));
    assert!(!evaluate("selector(a > b)"));
    assert!(!evaluate(""));
    assert!(!evaluate("display: flex"));
}

fn tags(source: &str) -> Vec<String> {
    css::parse(source)
        .rules
        .iter()
        .map(|rule| rule.selectors[0].tag_name.clone().unwrap())
        .collect()
}

#[test]
fn supports_blocks_keep_or_drop_their_rules() {
    assert_eq!(
        tags("a { x: 1px } @supports (display: flex) { b { x: 1px } } @supports (display: grid) { c { x: 1px } } @supports not (display: grid) { d { x: 1px } } e { x: 1px }"),
        ["a", "b", "d", "e"]
    );
}

#[test]
fn layer_blocks_are_transparent_and_statements_are_ignored() {
    assert_eq!(
        tags(
            "@layer base, theme; @layer base { a { x: 1px } } @layer { b { x: 1px } } c { x: 1px }"
        ),
        ["a", "b", "c"]
    );
}

#[test]
fn nested_blocks_combine_with_media_conditions() {
    let sheet = css::parse(
        "@supports (display: flex) { @media (min-width: 10px) { a { x: 1px } } } @layer l { @media print { b { x: 1px } } }",
    );
    assert_eq!(sheet.rules.len(), 2);
    assert_eq!(sheet.rules[0].media.len(), 1);
    assert_eq!(sheet.rules[1].media.len(), 1);
}

#[test]
fn other_at_rules_are_still_skipped() {
    assert_eq!(
        tags("@container (min-width: 1px) { a { x: 1px } } @font-face { font-family: X } @supportsfoo { b { x: 1px } } c { x: 1px }"),
        ["c"]
    );
}
