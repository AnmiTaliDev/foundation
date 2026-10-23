// SPDX-License-Identifier: GPL-3.0-only

use foundation::bidi::{self, Class};

fn levels(text: &str, rtl: bool) -> Vec<u8> {
    let chars: Vec<char> = text.chars().collect();
    bidi::levels(&chars, rtl)
}

const HEBREW: &str = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";
const ARABIC: &str = "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}";

#[test]
fn characters_are_classified() {
    assert_eq!(bidi::class('a'), Class::L);
    assert_eq!(bidi::class('\u{05D0}'), Class::R);
    assert_eq!(bidi::class('\u{0627}'), Class::AL);
    assert_eq!(bidi::class('5'), Class::EN);
    assert_eq!(bidi::class('\u{0665}'), Class::AN);
    assert_eq!(bidi::class(' '), Class::WS);
    assert_eq!(bidi::class(','), Class::CS);
    assert_eq!(bidi::class('+'), Class::ES);
    assert_eq!(bidi::class('%'), Class::ET);
    assert_eq!(bidi::class('\u{0301}'), Class::NSM);
    assert_eq!(bidi::class('('), Class::ON);
    assert_eq!(bidi::class('\u{200F}'), Class::R);
}

#[test]
fn latin_text_in_ltr_stays_at_level_zero() {
    assert_eq!(levels("hello world 42", false), vec![0; 14]);
}

#[test]
fn hebrew_inside_ltr_text_is_level_one() {
    let text = format!("ab {} cd", HEBREW);
    let found = levels(&text, false);
    assert_eq!(found[..3], [0, 0, 0]);
    assert_eq!(found[3..7], [1, 1, 1, 1]);
    assert_eq!(found[7..], [0, 0, 0]);
}

#[test]
fn neutrals_between_different_directions_take_the_embedding_level() {
    let text = format!("a {}", HEBREW);
    assert_eq!(levels(&text, false)[1], 0);
    let text = format!("{} a", HEBREW);
    assert_eq!(levels(&text, true)[4], 1);
}

#[test]
fn neutrals_between_the_same_direction_follow_it() {
    let text = format!("{} {}", HEBREW, HEBREW);
    let found = levels(&text, false);
    assert_eq!(found[4], 1);
}

#[test]
fn trailing_punctuation_stays_on_the_embedding_side() {
    let text = format!("{},", HEBREW);
    let found = levels(&text, false);
    assert_eq!(found[4], 0);
    let found = levels(&text, true);
    assert_eq!(found[4], 1);
}

#[test]
fn numbers_in_right_to_left_text_are_level_two() {
    let text = format!("{} 123", HEBREW);
    let found = levels(&text, false);
    assert_eq!(found[5..], [2, 2, 2]);
    assert_eq!(found[4], 1);
}

#[test]
fn numbers_after_latin_text_are_left_to_right() {
    assert_eq!(levels("abc 123", true), vec![2; 7]);
    assert_eq!(levels("abc 123", false), vec![0; 7]);
}

#[test]
fn arabic_digits_are_numbers() {
    let text = format!("{} \u{0661}\u{0662}", ARABIC);
    let found = levels(&text, false);
    assert_eq!(found[6..], [2, 2]);
}

#[test]
fn european_digits_after_arabic_letters_become_arabic_numbers() {
    let text = format!("{}1", ARABIC);
    let found = levels(&text, false);
    assert_eq!(found[5], 2);
}

#[test]
fn separators_inside_numbers_stay_numeric() {
    let text = format!("{} 1,5", HEBREW);
    let found = levels(&text, false);
    assert_eq!(found[5..], [2, 2, 2]);
    let text = format!("{} 1+5", HEBREW);
    assert_eq!(levels(&text, false)[5..], [2, 2, 2]);
}

#[test]
fn currency_and_percent_attach_to_numbers() {
    let text = format!("{} 5%", HEBREW);
    assert_eq!(levels(&text, false)[5..], [2, 2]);
}

#[test]
fn marks_follow_their_base_character() {
    let text = format!("{}\u{05B0}", HEBREW);
    let found = levels(&text, false);
    assert_eq!(found[4], 1);
}

#[test]
fn latin_in_right_to_left_text_is_level_two() {
    let found = levels("ab cd", true);
    assert_eq!(found, vec![2; 5]);
    let text = format!("{} ab", HEBREW);
    let found = levels(&text, true);
    assert_eq!(found[5..], [2, 2]);
    assert_eq!(found[4], 1);
}

#[test]
fn reordering_reverses_odd_runs() {
    assert_eq!(bidi::reorder(&[0, 0, 0]), vec![0, 1, 2]);
    assert_eq!(bidi::reorder(&[1, 1, 1]), vec![2, 1, 0]);
    assert_eq!(bidi::reorder(&[0, 1, 1, 0]), vec![0, 2, 1, 3]);
    assert_eq!(bidi::reorder(&[1, 2, 2, 1]), vec![3, 1, 2, 0]);
    assert_eq!(bidi::reorder(&[0, 2, 2, 0]), vec![0, 1, 2, 3]);
    assert_eq!(bidi::reorder(&[]), Vec::<usize>::new());
}

#[test]
fn right_to_left_numbers_keep_their_internal_order() {
    let order = bidi::reorder(&[1, 1, 2, 2]);
    assert_eq!(order, vec![2, 3, 1, 0]);
}

#[test]
fn visual_reversal_keeps_marks_after_their_base() {
    assert_eq!(bidi::reverse_visual("abc"), "cba");
    assert_eq!(bidi::reverse_visual("a\u{0301}b"), "ba\u{0301}");
    assert_eq!(bidi::reverse_visual(""), "");
}
