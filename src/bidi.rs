// SPDX-License-Identifier: GPL-3.0-only

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    L,
    R,
    AL,
    EN,
    AN,
    ES,
    ET,
    CS,
    NSM,
    WS,
    ON,
}

pub fn class(c: char) -> Class {
    let code = c as u32;
    match c {
        '\u{200E}' => return Class::L,
        '\u{200F}' => return Class::R,
        '\u{061C}' => return Class::AL,
        '0'..='9' | '\u{06F0}'..='\u{06F9}' => return Class::EN,
        '+' | '-' => return Class::ES,
        '#' | '$' | '%' | '\u{00A2}'..='\u{00A5}' | '\u{00B0}' | '\u{20A0}'..='\u{20CF}' => {
            return Class::ET
        }
        ',' | '.' | '/' | ':' | '\u{00A0}' => return Class::CS,
        ' ' | '\t' | '\n' | '\r' | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{3000}' => {
            return Class::WS
        }
        _ => {}
    }
    match code {
        0x0660..=0x0669 | 0x066B..=0x066C | 0x0600..=0x0605 => Class::AN,
        0x0300..=0x036F
        | 0x0483..=0x0489
        | 0x0591..=0x05BD
        | 0x05BF
        | 0x05C1..=0x05C2
        | 0x05C4..=0x05C5
        | 0x05C7
        | 0x0610..=0x061A
        | 0x064B..=0x065F
        | 0x0670
        | 0x06D6..=0x06DC
        | 0x06DF..=0x06E4
        | 0x06E7..=0x06E8
        | 0x06EA..=0x06ED => Class::NSM,
        0x0590..=0x05FF
        | 0x07C0..=0x085F
        | 0xFB1D..=0xFB4F
        | 0x10800..=0x10FFF
        | 0x1E800..=0x1EFFF => Class::R,
        0x0600..=0x07BF | 0x0860..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => Class::AL,
        _ if c.is_alphabetic() => Class::L,
        _ => Class::ON,
    }
}

pub fn is_mark(c: char) -> bool {
    class(c) == Class::NSM
}

pub fn has_right_to_left(text: &[char]) -> bool {
    text.iter()
        .any(|c| matches!(class(*c), Class::R | Class::AL | Class::AN))
}

pub fn levels(text: &[char], rtl: bool) -> Vec<u8> {
    if !rtl && !has_right_to_left(text) {
        return vec![0; text.len()];
    }
    let mut classes: Vec<Class> = text.iter().map(|c| class(*c)).collect();

    let mut previous = if rtl { Class::R } else { Class::L };
    for current in classes.iter_mut() {
        if *current == Class::NSM {
            *current = previous;
        }
        previous = *current;
    }

    let mut strong = if rtl { Class::R } else { Class::L };
    for current in classes.iter_mut() {
        match *current {
            Class::L | Class::R | Class::AL => strong = *current,
            Class::EN if strong == Class::AL => *current = Class::AN,
            _ => {}
        }
    }
    for current in classes.iter_mut() {
        if *current == Class::AL {
            *current = Class::R;
        }
    }

    for index in 1..classes.len().saturating_sub(1) {
        let before = classes[index - 1];
        let after = classes[index + 1];
        match classes[index] {
            Class::ES if before == Class::EN && after == Class::EN => classes[index] = Class::EN,
            Class::CS if before == after && matches!(before, Class::EN | Class::AN) => {
                classes[index] = before
            }
            _ => {}
        }
    }

    let mut index = 0;
    while index < classes.len() {
        if classes[index] != Class::ET {
            index += 1;
            continue;
        }
        let start = index;
        while index < classes.len() && classes[index] == Class::ET {
            index += 1;
        }
        let touches_number = (start > 0 && classes[start - 1] == Class::EN)
            || (index < classes.len() && classes[index] == Class::EN);
        if touches_number {
            for class in &mut classes[start..index] {
                *class = Class::EN;
            }
        }
    }
    for current in classes.iter_mut() {
        if matches!(*current, Class::ES | Class::ET | Class::CS) {
            *current = Class::ON;
        }
    }

    let mut strong = if rtl { Class::R } else { Class::L };
    for current in classes.iter_mut() {
        match *current {
            Class::L | Class::R => strong = *current,
            Class::EN if strong == Class::L => *current = Class::L,
            _ => {}
        }
    }

    let direction = |class: Class| match class {
        Class::L => Some(Class::L),
        Class::R | Class::EN | Class::AN => Some(Class::R),
        _ => None,
    };
    let embedding = if rtl { Class::R } else { Class::L };
    let mut index = 0;
    while index < classes.len() {
        if direction(classes[index]).is_some() {
            index += 1;
            continue;
        }
        let start = index;
        while index < classes.len() && direction(classes[index]).is_none() {
            index += 1;
        }
        let before = if start == 0 {
            Some(embedding)
        } else {
            direction(classes[start - 1])
        };
        let after = if index == classes.len() {
            Some(embedding)
        } else {
            direction(classes[index])
        };
        let resolved = if before == after {
            before.unwrap_or(embedding)
        } else {
            embedding
        };
        for class in &mut classes[start..index] {
            *class = resolved;
        }
    }

    classes
        .iter()
        .map(|class| match (!rtl, class) {
            (true, Class::L) => 0,
            (true, Class::R) => 1,
            (true, _) => 2,
            (false, Class::R) => 1,
            (false, _) => 2,
        })
        .collect()
}

pub fn reorder(levels: &[u8]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..levels.len()).collect();
    let Some(highest) = levels.iter().copied().max() else {
        return order;
    };
    let Some(lowest_odd) = levels.iter().copied().filter(|level| level % 2 == 1).min() else {
        return order;
    };
    let mut level = highest;
    while level >= lowest_odd {
        let mut index = 0;
        while index < order.len() {
            if levels[order[index]] < level {
                index += 1;
                continue;
            }
            let start = index;
            while index < order.len() && levels[order[index]] >= level {
                index += 1;
            }
            order[start..index].reverse();
        }
        level -= 1;
    }
    order
}

const MIRRORED: [(char, char); 22] = [
    ('(', ')'),
    ('[', ']'),
    ('{', '}'),
    ('<', '>'),
    ('\u{00AB}', '\u{00BB}'),
    ('\u{2039}', '\u{203A}'),
    ('\u{2045}', '\u{2046}'),
    ('\u{207D}', '\u{207E}'),
    ('\u{208D}', '\u{208E}'),
    ('\u{2264}', '\u{2265}'),
    ('\u{2268}', '\u{2269}'),
    ('\u{226A}', '\u{226B}'),
    ('\u{2308}', '\u{2309}'),
    ('\u{230A}', '\u{230B}'),
    ('\u{2329}', '\u{232A}'),
    ('\u{3008}', '\u{3009}'),
    ('\u{300A}', '\u{300B}'),
    ('\u{300C}', '\u{300D}'),
    ('\u{300E}', '\u{300F}'),
    ('\u{3010}', '\u{3011}'),
    ('\u{3014}', '\u{3015}'),
    ('\u{3016}', '\u{3017}'),
];

pub fn mirror(c: char) -> char {
    for (open, close) in MIRRORED {
        if c == open {
            return close;
        }
        if c == close {
            return open;
        }
    }
    c
}

pub fn reverse_visual(text: &str) -> String {
    let mut clusters: Vec<String> = Vec::new();
    for c in text.chars() {
        match clusters.last_mut() {
            Some(last) if is_mark(c) => last.push(c),
            _ => clusters.push(c.to_string()),
        }
    }
    clusters
        .iter()
        .rev()
        .flat_map(|cluster| cluster.chars())
        .map(mirror)
        .collect()
}
