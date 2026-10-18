// SPDX-License-Identifier: GPL-3.0-only

use std::fs;
use std::io::{BufRead, BufReader, Cursor, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;

use foundation::css::Color;
use foundation::location::Location;
use foundation::{picture, Page};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn url(text: &str) -> Location {
    Location::Url(text.to_string())
}

fn png_bytes(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba(pixel));
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();
    bytes
}

fn temp_dir(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("foundation-test-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn urls_resolve_relative_references() {
    let base = url("http://example.com/a/b/page.html?x=1#top");
    let cases = [
        ("c.css", "http://example.com/a/b/c.css"),
        ("./c.css", "http://example.com/a/b/c.css"),
        ("../c.css", "http://example.com/a/c.css"),
        ("../../../c.css", "http://example.com/c.css"),
        ("/root.css", "http://example.com/root.css"),
        ("sub/c.css?v=2", "http://example.com/a/b/sub/c.css?v=2"),
        ("?y=2", "http://example.com/a/b/page.html?y=2"),
        ("//cdn.example.org/x.css", "http://cdn.example.org/x.css"),
        ("https://other.org/y.css#frag", "https://other.org/y.css"),
        ("c.css#frag", "http://example.com/a/b/c.css"),
    ];
    for (reference, expected) in cases {
        assert_eq!(
            base.resolve(reference).unwrap(),
            url(expected),
            "{}",
            reference
        );
    }
    assert_eq!(
        url("https://example.com").resolve("x.css").unwrap(),
        url("https://example.com/x.css")
    );
    assert_eq!(
        url("https://example.com/dir/").resolve("..").unwrap(),
        url("https://example.com/")
    );
}

#[test]
fn files_resolve_relative_to_document() {
    let base = Location::File(PathBuf::from("/site/pages/index.html"));
    assert_eq!(
        base.resolve("../img/a.png?v=1").unwrap(),
        Location::File(PathBuf::from("/site/pages/../img/a.png"))
    );
    assert_eq!(
        base.resolve("/abs/a.png").unwrap(),
        Location::File(PathBuf::from("/abs/a.png"))
    );
    assert_eq!(
        base.resolve("https://example.com/a.png").unwrap(),
        url("https://example.com/a.png")
    );
}

#[test]
fn unsupported_schemes_and_empty_references_are_rejected() {
    let remote = url("http://example.com/");
    let local = Location::File(PathBuf::from("index.html"));
    for base in [&remote, &local] {
        assert!(base.resolve("file:///etc/passwd").is_err());
        assert!(base.resolve("data:text/css,a{}").is_err());
        assert!(base.resolve("javascript:alert(1)").is_err());
        assert!(base.resolve("").is_err());
        assert!(base.resolve("#top").is_err());
    }
}

#[test]
fn picture_decodes_png_and_jpeg_and_rejects_garbage() {
    let png = picture::decode(&png_bytes(3, 2, [10, 20, 30, 255])).unwrap();
    assert_eq!((png.width, png.height), (3, 2));
    assert_eq!(&png.rgba[..4], &[10, 20, 30, 255]);

    let mut jpeg = Vec::new();
    image::RgbImage::from_pixel(5, 4, image::Rgb([200, 100, 50]))
        .write_to(&mut Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
        .unwrap();
    let decoded = picture::decode(&jpeg).unwrap();
    assert_eq!((decoded.width, decoded.height), (5, 4));

    assert!(picture::decode(b"not an image").is_err());
}

#[test]
fn page_loads_linked_stylesheet_and_images_from_files() {
    let directory = temp_dir("files");
    fs::write(
        directory.join("index.html"),
        "<html><head><link rel=\"Stylesheet\" href=\"style.css\"></head><body><div class=\"a\"></div><img src=\"red.png\" width=\"8\" height=\"8\"></body></html>",
    )
    .unwrap();
    fs::write(
        directory.join("style.css"),
        "body { margin: 0 } .a { height: 10px; background-color: red } img { display: block }",
    )
    .unwrap();
    fs::write(directory.join("red.png"), png_bytes(4, 2, [255, 0, 0, 255])).unwrap();

    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    let canvas = page.render(30, 30, 0.0).canvas;
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(29, 9), RED);
    assert_eq!(canvas.pixel(0, 10), RED);
    assert_eq!(canvas.pixel(7, 17), RED);
    assert_eq!(canvas.pixel(8, 10), WHITE);
    assert_eq!(canvas.pixel(0, 18), WHITE);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn image_keeps_aspect_ratio_and_natural_size() {
    let directory = temp_dir("sizes");
    fs::write(
        directory.join("index.html"),
        "<html><style>body { margin: 0 } img { display: block }</style><body><img src=\"a.png\" width=\"8\"><img src=\"a.png\"></body></html>",
    )
    .unwrap();
    fs::write(directory.join("a.png"), png_bytes(4, 2, [255, 0, 0, 255])).unwrap();

    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    let canvas = page.render(20, 20, 0.0).canvas;
    assert_eq!(canvas.pixel(7, 3), RED);
    assert_eq!(canvas.pixel(0, 4), RED);
    assert_eq!(canvas.pixel(3, 5), RED);
    assert_eq!(canvas.pixel(4, 5), WHITE);
    assert_eq!(canvas.pixel(0, 6), WHITE);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn transparent_pixels_keep_the_background() {
    let directory = temp_dir("alpha");
    fs::write(
        directory.join("index.html"),
        "<html><style>body { margin: 0 } img { display: block }</style><body><img src=\"t.png\"></body></html>",
    )
    .unwrap();
    fs::write(directory.join("t.png"), png_bytes(2, 2, [255, 0, 0, 0])).unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    let canvas = page.render(10, 10, 0.0).canvas;
    assert_eq!(canvas.pixel(0, 0), WHITE);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn inline_image_sits_on_the_baseline_next_to_text() {
    let directory = temp_dir("inline");
    fs::write(
        directory.join("index.html"),
        "<html><style>body { margin: 0 }</style><body>x <img src=\"r.png\" width=\"10\" height=\"30\"> y</body></html>",
    )
    .unwrap();
    fs::write(directory.join("r.png"), png_bytes(2, 2, [255, 0, 0, 255])).unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    let frame = page.render(100, 60, 0.0);
    assert!(frame.content_height >= 30.0);
    let mut red_rows = Vec::new();
    for y in 0..60 {
        let mut has_red = false;
        for x in 0..100 {
            if frame.canvas.pixel(x, y) == RED {
                has_red = true;
            }
        }
        if has_red {
            red_rows.push(y);
        }
    }
    assert_eq!(red_rows.len(), 30);
    assert_eq!(red_rows[0], 0);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn missing_resources_produce_warnings_not_failures() {
    let directory = temp_dir("missing");
    fs::write(
        directory.join("index.html"),
        "<html><head><link rel=\"stylesheet\" href=\"none.css\"><link rel=\"stylesheet\" href=\"file:///etc/passwd\"></head><body><img src=\"none.png\" width=\"4\" height=\"4\"><img src=\"bad.png\"></body></html>",
    )
    .unwrap();
    fs::write(directory.join("bad.png"), b"garbage").unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    assert_eq!(page.warnings.len(), 4, "{:?}", page.warnings);
    let frame = page.render(20, 20, 0.0);
    assert!(frame.content_height > 0.0);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn opening_a_missing_document_fails() {
    assert!(Page::open("/nonexistent/foundation/index.html").is_err());
}

fn serve(
    responses: Vec<(&'static str, &'static str, Vec<u8>)>,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        for _ in 0..responses.len() {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header == "\r\n" || header.is_empty() {
                    break;
                }
            }
            let path = request_line
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .to_string();
            let mut status = "404 Not Found";
            let mut content_type = "text/plain";
            let mut body: Vec<u8> = b"missing".to_vec();
            for (route, kind, data) in &responses {
                if *route == path {
                    status = "200 OK";
                    content_type = kind;
                    body = data.clone();
                }
            }
            let head = format!(
                "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                status,
                content_type,
                body.len()
            );
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    (format!("http://{}", address), handle)
}

#[test]
fn page_loads_document_stylesheet_and_image_over_http() {
    let html = "<html><head><link rel=\"stylesheet\" href=\"/css/s.css\"></head><body><img src=\"img/p.png\"></body></html>";
    let css = "body { margin: 0 } .a { height: 10px } img { display: block }";
    let (base, handle) = serve(vec![
        ("/dir/page.html", "text/html", html.as_bytes().to_vec()),
        ("/css/s.css", "text/css", css.as_bytes().to_vec()),
        (
            "/dir/img/p.png",
            "image/png",
            png_bytes(2, 2, [255, 0, 0, 255]),
        ),
    ]);
    let page = Page::open(&format!("{}/dir/page.html", base)).unwrap();
    handle.join().unwrap();
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    let canvas = page.render(10, 10, 0.0).canvas;
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(1, 1), RED);
    assert_eq!(canvas.pixel(2, 0), WHITE);
}

#[test]
fn http_errors_are_reported() {
    let (base, handle) = serve(vec![("/other", "text/plain", b"x".to_vec())]);
    let result = Page::open(&format!("{}/missing.html", base));
    handle.join().unwrap();
    assert!(result.is_err());
}

const GREEN: Color = Color { r: 0, g: 255, b: 0 };

fn write(directory: &std::path::Path, name: &str, content: &str) {
    let path = directory.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn open_file(directory: &std::path::Path, name: &str) -> Page {
    Page::open(directory.join(name).to_str().unwrap()).unwrap()
}

#[test]
fn file_base_with_a_trailing_slash_is_a_directory() {
    let base = Location::File(PathBuf::from("/site/pages/"));
    assert_eq!(
        base.resolve("a.css").unwrap(),
        Location::File(PathBuf::from("/site/pages/a.css"))
    );
    let file = Location::File(PathBuf::from("/site/pages"));
    assert_eq!(
        file.resolve("a.css").unwrap(),
        Location::File(PathBuf::from("/site/a.css"))
    );
}

#[test]
fn base_href_changes_where_subresources_are_loaded_from() {
    let directory = temp_dir("base");
    write(
        &directory,
        "index.html",
        "<html><head><base href=\"assets/\"><link rel=\"stylesheet\" href=\"s.css\"></head><body><img src=\"p.png\"></body></html>",
    );
    write(
        &directory,
        "assets/s.css",
        "body { margin: 0 } img { display: block }",
    );
    fs::write(
        directory.join("assets/p.png"),
        png_bytes(2, 2, [255, 0, 0, 255]),
    )
    .unwrap();
    let page = open_file(&directory, "index.html");
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    let canvas = page.render(10, 10, 0.0).canvas;
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(1, 1), RED);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn base_href_is_resolved_against_the_document_and_only_the_first_counts() {
    let directory = temp_dir("base-first");
    write(
        &directory,
        "pages/index.html",
        "<html><head><base target=\"_blank\"><base href=\"../shared\"><base href=\"other/\"><link rel=\"stylesheet\" href=\"s.css\"></head><body></body></html>",
    );
    write(&directory, "s.css", "body { background-color: red }");
    write(
        &directory,
        "shared/s.css",
        "body { background-color: lime }",
    );
    write(
        &directory,
        "pages/other/s.css",
        "body { background-color: blue }",
    );
    let page = open_file(&directory, "pages/index.html");
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    assert_eq!(page.render(4, 4, 0.0).canvas.pixel(0, 0), RED);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn base_href_resolves_the_directory_when_it_ends_with_a_slash() {
    let directory = temp_dir("base-dir");
    write(
        &directory,
        "pages/index.html",
        "<html><head><base href=\"../shared/\"><link rel=\"stylesheet\" href=\"s.css\"></head><body></body></html>",
    );
    write(
        &directory,
        "shared/s.css",
        "body { background-color: lime }",
    );
    let page = open_file(&directory, "pages/index.html");
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    assert_eq!(page.render(4, 4, 0.0).canvas.pixel(0, 0), GREEN);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn unsupported_base_href_is_reported_and_ignored() {
    let directory = temp_dir("base-bad");
    write(
        &directory,
        "index.html",
        "<html><head><base href=\"file:///etc/\"><link rel=\"stylesheet\" href=\"s.css\"></head><body></body></html>",
    );
    write(&directory, "s.css", "body { background-color: lime }");
    let page = open_file(&directory, "index.html");
    assert_eq!(page.warnings.len(), 1, "{:?}", page.warnings);
    assert_eq!(page.render(4, 4, 0.0).canvas.pixel(0, 0), GREEN);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn http_base_href_redirects_resources_of_a_local_document() {
    let (origin, handle) = serve(vec![(
        "/x/s.css",
        "text/css",
        b"body { background-color: lime }".to_vec(),
    )]);
    let directory = temp_dir("base-http");
    write(
        &directory,
        "index.html",
        &format!(
            "<html><head><base href=\"{}/x/\"><link rel=\"stylesheet\" href=\"s.css\"></head><body></body></html>",
            origin
        ),
    );
    let page = open_file(&directory, "index.html");
    handle.join().unwrap();
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    assert_eq!(page.render(4, 4, 0.0).canvas.pixel(0, 0), GREEN);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn imports_load_recursively_and_cascade_in_order() {
    let directory = temp_dir("imports");
    write(
        &directory,
        "index.html",
        "<html><head><link rel=\"stylesheet\" href=\"main.css\"></head><body><div class=\"x\"></div></body></html>",
    );
    write(
        &directory,
        "main.css",
        "@import \"a.css\"; body { margin: 0 } .x { background-color: lime }",
    );
    write(
        &directory,
        "a.css",
        "@import url(sub/b.css); .x { background-color: red; height: 10px }",
    );
    write(
        &directory,
        "sub/b.css",
        "@import '../c.css'; .x { width: 10px; background-color: blue }",
    );
    write(&directory, "c.css", ".x { margin-left: 2px }");
    let page = open_file(&directory, "index.html");
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    let canvas = page.render(30, 20, 0.0).canvas;
    assert_eq!(canvas.pixel(2, 0), GREEN);
    assert_eq!(canvas.pixel(11, 9), GREEN);
    assert_eq!(canvas.pixel(12, 0), WHITE);
    assert_eq!(canvas.pixel(1, 0), WHITE);
    assert_eq!(canvas.pixel(2, 10), WHITE);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn inline_style_imports_are_relative_to_the_document() {
    let directory = temp_dir("inline-import");
    write(
        &directory,
        "index.html",
        "<html><head><style>@import \"s.css\"; .x { height: 5px }</style></head><body><div class=\"x\"></div></body></html>",
    );
    write(
        &directory,
        "s.css",
        "body { margin: 0 } .x { background-color: lime }",
    );
    let page = open_file(&directory, "index.html");
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    let canvas = page.render(10, 10, 0.0).canvas;
    assert_eq!(canvas.pixel(0, 4), GREEN);
    assert_eq!(canvas.pixel(0, 5), WHITE);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn import_cycles_terminate() {
    let directory = temp_dir("cycle");
    write(
        &directory,
        "index.html",
        "<html><head><link rel=\"stylesheet\" href=\"a.css\"></head><body></body></html>",
    );
    write(
        &directory,
        "a.css",
        "@import \"b.css\"; body { margin: 0; background-color: red }",
    );
    write(
        &directory,
        "b.css",
        "@import \"a.css\"; @import \"b.css\"; body { height: 5px }",
    );
    let page = open_file(&directory, "index.html");
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    assert_eq!(page.render(10, 10, 0.0).canvas.pixel(0, 0), RED);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn imports_for_other_media_are_skipped_without_loading() {
    let directory = temp_dir("media");
    write(
        &directory,
        "index.html",
        "<html><head><link rel=\"stylesheet\" href=\"main.css\"><link rel=\"stylesheet\" media=\"print\" href=\"print.css\"><link rel=\"stylesheet\" media=\"(prefers-color-scheme: dark)\" href=\"missing.css\"></head><body></body></html>",
    );
    write(
        &directory,
        "main.css",
        "@import \"print.css\" print; @import \"gone.css\" (min-width: 10px); body { background-color: lime }",
    );
    write(&directory, "print.css", "body { background-color: red }");
    let page = open_file(&directory, "index.html");
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    assert_eq!(page.render(4, 4, 0.0).canvas.pixel(0, 0), GREEN);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn missing_imports_are_warnings_and_the_rest_still_applies() {
    let directory = temp_dir("import-missing");
    write(
        &directory,
        "index.html",
        "<html><head><link rel=\"stylesheet\" href=\"main.css\"></head><body></body></html>",
    );
    write(
        &directory,
        "main.css",
        "@import \"none.css\"; @import \"file:///etc/passwd\"; body { background-color: lime }",
    );
    let page = open_file(&directory, "index.html");
    assert_eq!(page.warnings.len(), 2, "{:?}", page.warnings);
    assert_eq!(page.render(4, 4, 0.0).canvas.pixel(0, 0), GREEN);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn import_depth_is_limited() {
    let directory = temp_dir("depth");
    write(
        &directory,
        "index.html",
        "<html><head><link rel=\"stylesheet\" href=\"c0.css\"></head><body></body></html>",
    );
    for level in 0..12 {
        write(
            &directory,
            &format!("c{}.css", level),
            &format!(
                "@import \"c{}.css\"; body {{ margin: {}px }}",
                level + 1,
                level
            ),
        );
    }
    let page = open_file(&directory, "index.html");
    assert!(
        page.warnings.iter().any(|w| w.contains("deeper than")),
        "{:?}",
        page.warnings
    );
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn imports_over_http_resolve_against_the_sheet_location() {
    let html = "<html><head><link rel=\"stylesheet\" href=\"/css/main.css\"></head><body><div class=\"x\"></div></body></html>";
    let main = "@import \"parts/a.css\"; @import \"/abs/b.css\"; body { margin: 0 } .x { background-color: lime }";
    let (origin, handle) = serve(vec![
        ("/page.html", "text/html", html.as_bytes().to_vec()),
        ("/css/main.css", "text/css", main.as_bytes().to_vec()),
        (
            "/css/parts/a.css",
            "text/css",
            b".x { height: 10px }".to_vec(),
        ),
        (
            "/abs/b.css",
            "text/css",
            b".x { width: 10px; background-color: blue }".to_vec(),
        ),
    ]);
    let page = Page::open(&format!("{}/page.html", origin)).unwrap();
    handle.join().unwrap();
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    let canvas = page.render(20, 20, 0.0).canvas;
    assert_eq!(canvas.pixel(9, 9), GREEN);
    assert_eq!(canvas.pixel(10, 0), WHITE);
    assert_eq!(canvas.pixel(0, 10), WHITE);
}
