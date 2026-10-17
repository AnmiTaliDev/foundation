# Foundation

Browser engine written in Rust. Part of the Naryn project (formerly OpenSerfing).

## Dependencies

* Rust toolchain (edition 2021, tested with 1.98)

Crate dependencies: `fontdue` (glyph rasterization), `minifb` (window, used by `foundation-window` only).

## Build

```
cargo build --release
cargo test
```

## Usage

```
foundation <input.html> <output.ppm> [width height]
```

Renders an HTML file to a PPM image. Default size is 800x600. Supported so far: HTML parsing into a DOM, CSS rules with tag, id and class selectors, block and inline layout with word wrapping, background, border and text painting. Text uses the embedded DejaVu Sans font.

```
foundation-window <input.html>
```

Opens a resizable window with the rendered page. Escape closes it.

## Acknowledgments

`assets/DejaVuSans.ttf` is DejaVu Sans, licensed as described in `assets/DejaVuSans-LICENSE.txt`.

## License

GPL-3.0-only. See [LICENSE](LICENSE).
