// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Import, Value};
use foundation::style::media_applies;

fn import(url: &str, media: &str) -> Import {
    Import {
        url: url.to_string(),
        media: media.to_string(),
    }
}

#[test]
fn imports_are_parsed_in_all_forms() {
    let source = "@charset \"utf-8\"; /* c */ @import url(\"a.css\"); @import url(b.css) screen; @IMPORT 'c.css' print, tv; @import \"d.css\"; @import url( 'e f.css' );";
    assert_eq!(
        css::parse_imports(source),
        vec![
            import("a.css", ""),
            import("b.css", "screen"),
            import("c.css", "print, tv"),
            import("d.css", ""),
            import("e f.css", ""),
        ]
    );
}

#[test]
fn imports_after_the_first_rule_and_malformed_ones_are_ignored() {
    assert!(css::parse_imports("a { color: red } @import \"late.css\";").is_empty());
    assert!(css::parse_imports("@import ; @import url(); @import nothing.css;").is_empty());
    assert_eq!(
        css::parse_imports("@import \"x.css\"; @media print { a { b: c } } @import \"y.css\";"),
        vec![import("x.css", "")]
    );
}

#[test]
fn at_rules_do_not_disturb_the_following_rules() {
    let sheet = css::parse(
        "@import \"x.css\"; a { width: 1px } @media screen { b { width: 2px } c { width: 3px } } @font-face { font-family: X; src: url(a;b) } @keyframes k { from { width: 1px } to { width: 2px } } d { width: 4px }",
    );
    assert_eq!(sheet.rules.len(), 4);
    assert_eq!(sheet.rules[0].selectors[0].tag_name.as_deref(), Some("a"));
    assert_eq!(sheet.rules[1].selectors[0].tag_name.as_deref(), Some("b"));
    assert_eq!(sheet.rules[2].selectors[0].tag_name.as_deref(), Some("c"));
    assert_eq!(sheet.rules[3].selectors[0].tag_name.as_deref(), Some("d"));
    assert_eq!(sheet.rules[3].declarations[0].value, Value::Length(4.0));
    assert!(sheet.rules[3].media.is_empty());
}

#[test]
fn strings_and_urls_may_contain_braces_and_semicolons() {
    let sheet = css::parse("a { content: \"}\"; width: 5px } b { background: url(\"x;y}.png\"); height: 1px } c { width: 2px }");
    assert_eq!(sheet.rules.len(), 3);
    assert!(sheet.rules[0]
        .declarations
        .iter()
        .any(|d| d.name == "width" && d.value == Value::Length(5.0)));
    assert!(sheet.rules[1]
        .declarations
        .iter()
        .any(|d| d.name == "height" && d.value == Value::Length(1.0)));
}

#[test]
fn stray_closing_braces_and_unterminated_blocks_are_tolerated() {
    let sheet = css::parse("} a { width: 1px } } b { width: 3px");
    assert_eq!(sheet.rules.len(), 2);
    assert_eq!(sheet.rules[1].declarations[0].value, Value::Length(3.0));
}

#[test]
fn media_queries_apply_only_for_screen_and_all() {
    for media in [
        "",
        "  ",
        "all",
        "screen",
        "SCREEN",
        "only screen",
        "screen and (min-width: 1px)",
        "print, screen",
        "all and (orientation: landscape)",
    ] {
        assert!(media_applies(media), "{:?}", media);
    }
    for media in [
        "print",
        "(min-width: 600px)",
        "not screen",
        "allx",
        "(prefers-color-scheme: dark)",
        "tv, print",
    ] {
        assert!(!media_applies(media), "{:?}", media);
    }
}
