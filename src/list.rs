// SPDX-License-Identifier: GPL-3.0-only

const TYPES: [&str; 10] = [
    "disc",
    "circle",
    "square",
    "decimal",
    "decimal-leading-zero",
    "lower-alpha",
    "lower-latin",
    "upper-alpha",
    "upper-latin",
    "lower-roman",
];

pub fn is_type(keyword: &str) -> bool {
    keyword == "none" || keyword == "upper-roman" || TYPES.contains(&keyword)
}

pub fn marker_text(kind: &str, value: i32) -> Option<String> {
    let text = match kind {
        "none" => return None,
        "disc" => return Some("\u{2022} ".to_string()),
        "circle" => return Some("\u{25E6} ".to_string()),
        "square" => return Some("\u{25AA} ".to_string()),
        "decimal-leading-zero" if (0..10).contains(&value) => format!("0{}", value),
        "lower-alpha" | "lower-latin" => alphabetic(value, b'a')?,
        "upper-alpha" | "upper-latin" => alphabetic(value, b'A')?,
        "lower-roman" => roman(value)?.to_lowercase(),
        "upper-roman" => roman(value)?,
        _ => value.to_string(),
    };
    Some(format!("{}. ", text))
}

fn alphabetic(value: i32, first: u8) -> Option<String> {
    if value < 1 {
        return None;
    }
    let mut number = value as u32;
    let mut letters = Vec::new();
    while number > 0 {
        number -= 1;
        letters.push(first + (number % 26) as u8);
        number /= 26;
    }
    letters.reverse();
    String::from_utf8(letters).ok()
}

fn roman(value: i32) -> Option<String> {
    if !(1..4000).contains(&value) {
        return None;
    }
    const NUMERALS: [(i32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut remaining = value;
    let mut text = String::new();
    for (size, numeral) in NUMERALS {
        while remaining >= size {
            text.push_str(numeral);
            remaining -= size;
        }
    }
    Some(text)
}
