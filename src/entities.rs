// SPDX-License-Identifier: GPL-3.0-only

const MAX_ENTITY_LENGTH: usize = 10;

const NAMED_ENTITIES: [(&str, char); 22] = [
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("nbsp", '\u{a0}'),
    ("copy", '\u{a9}'),
    ("reg", '\u{ae}'),
    ("trade", '\u{2122}'),
    ("deg", '\u{b0}'),
    ("middot", '\u{b7}'),
    ("bull", '\u{2022}'),
    ("hellip", '\u{2026}'),
    ("ndash", '\u{2013}'),
    ("mdash", '\u{2014}'),
    ("lsquo", '\u{2018}'),
    ("rsquo", '\u{2019}'),
    ("ldquo", '\u{201c}'),
    ("rdquo", '\u{201d}'),
    ("laquo", '\u{ab}'),
    ("raquo", '\u{bb}'),
    ("euro", '\u{20ac}'),
];

pub fn decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match decode_entity(after) {
            Some((c, consumed)) => {
                out.push(c);
                rest = &after[consumed..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode_entity(after: &str) -> Option<(char, usize)> {
    let end = after.find(';')?;
    if end == 0 || end > MAX_ENTITY_LENGTH {
        return None;
    }
    let name = &after[..end];
    let c = match name.strip_prefix('#') {
        Some(number) => decode_numeric(number)?,
        None => lookup_named(name)?,
    };
    Some((c, end + 1))
}

fn decode_numeric(number: &str) -> Option<char> {
    let code = match number.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => number.parse::<u32>().ok()?,
    };
    char::from_u32(code)
}

fn lookup_named(name: &str) -> Option<char> {
    for (entity, c) in NAMED_ENTITIES {
        if entity == name {
            return Some(c);
        }
    }
    None
}
