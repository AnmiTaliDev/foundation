// SPDX-License-Identifier: GPL-3.0-only

use std::borrow::Cow;

use crate::style::TextTransform;

pub fn apply(text: &str, transform: TextTransform) -> Cow<'_, str> {
    match transform {
        TextTransform::None => Cow::Borrowed(text),
        TextTransform::Uppercase => Cow::Owned(text.to_uppercase()),
        TextTransform::Lowercase => Cow::Owned(text.to_lowercase()),
        TextTransform::Capitalize => {
            let mut result = String::with_capacity(text.len());
            let mut at_start = true;
            for c in text.chars() {
                if at_start && c.is_alphanumeric() {
                    result.extend(c.to_uppercase());
                    at_start = false;
                } else {
                    result.push(c);
                    if c.is_whitespace() {
                        at_start = true;
                    } else if c.is_alphanumeric() {
                        at_start = false;
                    }
                }
            }
            Cow::Owned(result)
        }
    }
}
