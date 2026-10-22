// SPDX-License-Identifier: GPL-3.0-only

use std::fs;

use foundation::css::Color;
use foundation::paint::Canvas;
use foundation::Page;

const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};
const BLUE: Color = Color {
    r: 0,
    g: 117,
    b: 255,
};
const BORDER: Color = Color {
    r: 118,
    g: 118,
    b: 118,
};

fn render(name: &str, body: &str) -> Canvas {
    let directory =
        std::env::temp_dir().join(format!("foundation-form-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("index.html"),
        format!(
            "<html><style>body {{ margin: 0 }} .r {{ display: block }}</style><body>{}</body></html>",
            body
        ),
    )
    .unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    let canvas = page.render(240, 120, 0.0).canvas;
    fs::remove_dir_all(&directory).unwrap();
    canvas
}

fn pixels(canvas: &Canvas) -> Vec<Color> {
    let mut all = Vec::new();
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            all.push(canvas.pixel(x, y));
        }
    }
    all
}

fn same(a: &Canvas, b: &Canvas) -> bool {
    pixels(a) == pixels(b)
}

fn bounds(canvas: &Canvas, color: Color) -> Option<(usize, usize, usize, usize)> {
    let mut found: Option<(usize, usize, usize, usize)> = None;
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            if canvas.pixel(x, y) == color {
                found = Some(match found {
                    None => (x, y, x, y),
                    Some(f) => (f.0.min(x), f.1.min(y), f.2.max(x), f.3.max(y)),
                });
            }
        }
    }
    found
}

fn count(canvas: &Canvas, wanted: impl Fn(Color) -> bool) -> usize {
    pixels(canvas).into_iter().filter(|c| wanted(*c)).count()
}

#[test]
fn text_input_has_a_bordered_box() {
    let canvas = render("box", "<input>");
    let (left, top, right, bottom) = bounds(&canvas, BORDER).unwrap();
    assert_eq!((right - left + 1, bottom - top + 1), (160, 21));
    assert_eq!(canvas.pixel(80, 10), WHITE);
}

#[test]
fn input_value_is_drawn_in_black() {
    let canvas = render("value", "<input value=\"Hello\">");
    assert!(count(&canvas, |c| c.r < 60 && c.g < 60 && c.b < 60) > 20);
}

#[test]
fn placeholder_is_gray_and_value_wins() {
    let placeholder = render("placeholder", "<input placeholder=\"Name\">");
    assert!(count(&placeholder, |c| c.r < 200 && c.r == c.g && c.g == c.b) > 20);
    assert_eq!(count(&placeholder, |c| c.r < 60), 0);
    let both = render("both", "<input value=\"Name\" placeholder=\"Other\">");
    let value = render("only-value", "<input value=\"Name\">");
    assert!(same(&both, &value));
}

#[test]
fn empty_input_has_no_text() {
    let canvas = render("empty", "<input>");
    assert_eq!(count(&canvas, |c| c.r < 117), 0);
}

#[test]
fn password_is_masked() {
    let password = render("password", "<input type=password value=\"abc\">");
    let bullets = render("bullets", "<input value=\"\u{2022}\u{2022}\u{2022}\">");
    assert!(same(&password, &bullets));
}

#[test]
fn hidden_input_draws_nothing() {
    let canvas = render("hidden", "<input type=hidden value=\"x\">");
    assert_eq!(count(&canvas, |c| c != WHITE), 0);
}

#[test]
fn checkbox_shows_its_state() {
    let off = render("off", "<input type=checkbox>");
    let on = render("on", "<input type=checkbox checked>");
    let (left, top, right, bottom) = bounds(&off, BORDER).unwrap();
    assert_eq!((right - left + 1, bottom - top + 1), (13, 13));
    assert_eq!(off.pixel(10, 9), WHITE);
    assert_eq!(on.pixel(10, 9), BLUE);
}

#[test]
fn radio_is_round_and_shows_its_state() {
    let off = render("radio-off", "<input type=radio>");
    let on = render("radio-on", "<input type=radio checked>");
    assert_eq!(off.pixel(4, 3), WHITE);
    assert_eq!(off.pixel(10, 9), WHITE);
    assert_eq!(on.pixel(10, 9), BLUE);
    assert_eq!(on.pixel(5, 9), WHITE);
    assert_eq!(on.pixel(4, 9), BLUE);
}

#[test]
fn button_sizes_to_its_label() {
    let button = render("button", "<button>OK</button>");
    let (left, _, right, _) = bounds(&button, BORDER).unwrap();
    assert!(right - left + 1 < 60);
    assert!(count(&button, |c| c.r < 60 && c.g < 60 && c.b < 60) > 5);
    let wide = render("button-wide", "<button>OK OK OK OK OK</button>");
    let (wide_left, _, wide_right, _) = bounds(&wide, BORDER).unwrap();
    assert!(wide_right - wide_left > right - left);
}

#[test]
fn submit_has_a_default_label() {
    let default = render("submit", "<input type=submit>");
    let named = render("named", "<input type=submit value=\"Submit\">");
    let other = render("other", "<input type=submit value=\"Go\">");
    assert!(same(&default, &named));
    assert!(!same(&default, &other));
    let reset = render("reset", "<input type=reset>");
    let reset_named = render("reset-named", "<input type=reset value=\"Reset\">");
    assert!(same(&reset, &reset_named));
}

#[test]
fn select_shows_the_chosen_option_only() {
    let first = render("first", "<select><option>Alpha<option>Beta</select>");
    let alone = render("alone", "<select><option>Alpha</select>");
    assert!(same(&first, &alone));
    let chosen = render(
        "chosen",
        "<select><option>Alpha<option selected>Beta</select>",
    );
    let beta = render("beta", "<select><option>Beta</select>");
    assert!(same(&chosen, &beta));
    assert!(!same(&first, &chosen));
}

#[test]
fn select_looks_inside_optgroups() {
    let grouped = render(
        "grouped",
        "<select><optgroup label=g><option>Alpha</option></optgroup></select>",
    );
    let plain = render("plain", "<select><option>Alpha</select>");
    assert!(same(&grouped, &plain));
}

#[test]
fn textarea_keeps_its_text() {
    let text = render("area", "<textarea>ab\ncd</textarea>");
    let empty = render("area-empty", "<textarea></textarea>");
    assert!(!same(&text, &empty));
    let (left, top, right, bottom) = bounds(&empty, BORDER).unwrap();
    assert_eq!((right - left + 1, bottom - top + 1), (180, 40));
    let lead = render("area-lead", "<textarea>\nab\ncd</textarea>");
    assert!(same(&text, &lead));
}

#[test]
fn textarea_decodes_entities_and_ignores_tags() {
    let escaped = render("escaped", "<textarea>&lt;b&gt;</textarea>");
    let literal = render("literal", "<textarea><b></textarea>");
    assert!(same(&escaped, &literal));
}

#[test]
fn disabled_controls_are_grayed() {
    let canvas = render("disabled", "<input disabled>");
    assert_eq!(
        canvas.pixel(80, 10),
        Color {
            r: 235,
            g: 235,
            b: 235
        }
    );
}

#[test]
fn state_pseudo_classes_match() {
    let canvas = render(
        "states",
        "<style>input { width: 10px; height: 10px; padding: 0; border: 0; margin: 0; display: block; box-sizing: content-box }\
         input:optional { background: #ffff00 }\
         input:checked { background: #00ff00 }\
         input:disabled { background: #0000ff }\
         input:required { background: #ff00ff }</style>\
         <input type=checkbox checked><input disabled><input required><input>",
    );
    assert_eq!(canvas.pixel(5, 5), Color { r: 0, g: 255, b: 0 });
    assert_eq!(canvas.pixel(5, 15), Color { r: 0, g: 0, b: 255 });
    assert_eq!(
        canvas.pixel(5, 25),
        Color {
            r: 255,
            g: 0,
            b: 255
        }
    );
    assert_eq!(
        canvas.pixel(5, 35),
        Color {
            r: 255,
            g: 255,
            b: 0
        }
    );
}

#[test]
fn fieldset_draws_a_frame() {
    let canvas = render("fieldset", "<fieldset><legend>L</legend>x</fieldset>");
    assert!(count(&canvas, |c| c.r == 192 && c.g == 192 && c.b == 192) > 100);
}

#[test]
fn form_is_a_block() {
    let canvas = render("form", "<form><input></form><input type=checkbox>");
    let (left, top, _, _) = bounds(&canvas, BLUE).unwrap_or((0, 0, 0, 0));
    assert_eq!((left, top), (0, 0));
    let border = bounds(&canvas, BORDER).unwrap();
    assert_eq!(border.0, 0);
}
