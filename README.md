# Foundation

Browser engine written in Rust. Part of the Naryn project (formerly OpenSerfing).

## Dependencies

* Rust toolchain (edition 2021, tested with 1.98)

Crate dependencies: `fontdue` (glyph rasterization), `ureq` (HTTP and HTTPS), `image` (PNG and JPEG decoding), `minifb` (window, used by `foundation-window` only).

## Build

```
cargo build --release
cargo test
```

## Usage

```
foundation <input.html|url> <output.ppm> [width height]
```

Renders an HTML file or an http(s) URL to a PPM image. Linked stylesheets (`<link rel="stylesheet">`) and images (`<img>`, PNG and JPEG) are loaded relative to the document. Resources that fail to load are reported as warnings on stderr. Default size is 800x600. Supported so far: HTML parsing into a DOM, CSS rules with tag, id, class, descendant and child selectors, the `style` attribute, `px`, `em`, `rem`, `%`, `vw`, `vh`, `vmin` and `vmax` lengths, `calc()`, the `font` and `background` shorthands, HTML entities, block and inline layout with `float`, `clear` and `position` (`relative`, `absolute`, `fixed`, `z-index`) and `display: flex`, margin collapsing, `min-*` and `max-*` sizes, `text-align`, `line-height`, word wrapping and breaking of long words, background, border, image and text painting. Text uses the embedded DejaVu Sans font.

```
foundation-window <input.html|url>
```

Opens a resizable window with the rendered page. Scroll with the mouse wheel, Up, Down, PageUp, PageDown, Home and End. Escape closes it.

## Acknowledgments

`assets/DejaVuSans.ttf` is DejaVu Sans, licensed as described in `assets/DejaVuSans-LICENSE.txt`.

## License

GPL-3.0-only. See [LICENSE](LICENSE).
