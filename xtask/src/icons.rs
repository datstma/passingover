//! `cargo xtask icons`: renders the icon masters in `design/icon/` into every
//! platform's icon files (D43). The output is committed; run this again after
//! changing a master.

use std::fs;
use std::path::Path;

use resvg::{tiny_skia, usvg};

use crate::{Result, workspace_root};

/// Masters are drawn on a 1024-unit square.
const CANVAS: f64 = 1024.0;

/// The rounded squares are superellipses, close to Apple's continuous corners.
const SUPERELLIPSE_EXPONENT: f64 = 5.0;

/// Android density buckets and their scale from dp to pixels.
const ANDROID_DENSITIES: [(&str, f64); 5] = [
    ("mdpi", 1.0),
    ("hdpi", 1.5),
    ("xhdpi", 2.0),
    ("xxhdpi", 3.0),
    ("xxxhdpi", 4.0),
];

/// Sizes in the Windows `.ico`.
const WINDOWS_SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

struct Masters {
    background: String,
    foreground: String,
    monochrome: String,
    menubar: String,
}

pub fn icons(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        return Err("usage: cargo xtask icons".into());
    }
    let root = workspace_root();
    let dir = root.join("design/icon");
    let read = |name: &str| {
        fs::read_to_string(dir.join(name)).map_err(|e| format!("design/icon/{name}: {e}"))
    };
    let masters = Masters {
        background: read("background.svg")?,
        foreground: read("foreground.svg")?,
        monochrome: read("monochrome.svg")?,
        menubar: read("menubar.svg")?,
    };
    let app = root.join("app");

    // iOS masks the icon itself and wants it opaque.
    let full = masters.full();
    asset_catalog(
        &app.join("ios/Runner/Assets.xcassets/AppIcon.appiconset"),
        |px| png_rgb(&render(&full, px, px)?),
    )?;

    // macOS shows the icon as drawn: a rounded square on the 824-unit grid,
    // with its own shadow.
    let mac = masters.shaped(100.0, true);
    asset_catalog(
        &app.join("macos/Runner/Assets.xcassets/AppIcon.appiconset"),
        |px| png_rgba(&render(&mac, px, px)?),
    )?;

    // Android: the adaptive-icon layers (108 dp, the launcher masks them) and
    // a rounded square for launchers older than Android 8.
    let res = app.join("android/app/src/main/res");
    let legacy = masters.shaped(43.0, false);
    let layers = [
        ("ic_launcher_background", svg(&masters.background)),
        ("ic_launcher_foreground", svg(&masters.foreground)),
        ("ic_launcher_monochrome", svg(&masters.monochrome)),
    ];
    for (density, scale) in ANDROID_DENSITIES {
        let mipmap = res.join(format!("mipmap-{density}"));
        let px = (48.0 * scale).round() as u32;
        write(
            &mipmap.join("ic_launcher.png"),
            &png_rgba(&render(&legacy, px, px)?)?,
        )?;
        let px = (108.0 * scale).round() as u32;
        for (name, layer) in &layers {
            write(
                &mipmap.join(format!("{name}.png")),
                &png_rgba(&render(layer, px, px)?)?,
            )?;
        }
    }

    // Windows shows the icon as drawn, slightly larger than on macOS.
    let windows = masters.shaped(40.0, false);
    let mut entries = Vec::new();
    for px in WINDOWS_SIZES {
        entries.push((px, png_rgba(&render(&windows, px, px)?)?));
    }
    write(
        &app.join("windows/runner/resources/app_icon.ico"),
        &ico(&entries),
    )?;

    // The macOS menu-bar template image, 18 × 18 pt (D39).
    let bar = app.join("macos/Runner/Assets.xcassets/MenuBarIcon.imageset");
    for (name, px) in [("menubar.png", 18), ("menubar@2x.png", 36)] {
        write(
            &bar.join(name),
            &png_rgba(&render(&masters.menubar, px, px)?)?,
        )?;
    }

    println!("icons written for iOS, macOS, Android and Windows");
    Ok(())
}

impl Masters {
    /// The background and foreground layers, full bleed.
    fn full(&self) -> String {
        svg(&format!(
            "{}{}",
            inner(&self.background),
            inner(&self.foreground)
        ))
    }

    /// The full icon inside a rounded square inset by `inset` units, with a
    /// soft drop shadow if `shadow`.
    fn shaped(&self, inset: f64, shadow: bool) -> String {
        let size = CANVAS - 2.0 * inset;
        let shape = superellipse(CANVAS / 2.0, size / 2.0);
        let (filter, backdrop) = if shadow {
            (
                r##"<filter id="icon-shadow" x="-20%" y="-20%" width="140%" height="140%"><feDropShadow dx="0" dy="10" stdDeviation="12" flood-color="#000000" flood-opacity="0.3"/></filter>"##.to_string(),
                format!(r##"<path d="{shape}" fill="#000000" filter="url(#icon-shadow)"/>"##),
            )
        } else {
            (String::new(), String::new())
        };
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {CANVAS} {CANVAS}"><defs><clipPath id="icon-shape"><path d="{shape}"/></clipPath>{filter}</defs>{backdrop}<g clip-path="url(#icon-shape)"><svg x="{inset}" y="{inset}" width="{size}" height="{size}" viewBox="0 0 {CANVAS} {CANVAS}">{}{}</svg></g></svg>"#,
            inner(&self.background),
            inner(&self.foreground),
        )
    }
}

/// Wraps `content` in a 1024-unit SVG document.
fn svg(content: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {CANVAS} {CANVAS}">{}</svg>"#,
        inner(content)
    )
}

/// What lies between a document's outer `<svg …>` and `</svg>`, or the text
/// itself if it has no outer element.
fn inner(document: &str) -> &str {
    let Some(start) = document.find("<svg") else {
        return document;
    };
    let open = start + document[start..].find('>').map_or(0, |i| i + 1);
    let close = document.rfind("</svg>").unwrap_or(document.len());
    &document[open..close]
}

/// A superellipse centred on (`centre`, `centre`) with half-width `radius`.
fn superellipse(centre: f64, radius: f64) -> String {
    const POINTS: usize = 256;
    let mut d = String::new();
    for i in 0..POINTS {
        let t = std::f64::consts::TAU * i as f64 / POINTS as f64;
        let bend = |v: f64| v.signum() * v.abs().powf(2.0 / SUPERELLIPSE_EXPONENT);
        let x = centre + radius * bend(t.cos());
        let y = centre + radius * bend(t.sin());
        d.push_str(&format!("{}{x:.2} {y:.2}", if i == 0 { "M" } else { "L" }));
    }
    d.push('Z');
    d
}

fn render(svg: &str, width: u32, height: u32) -> Result<tiny_skia::Pixmap> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .map_err(|e| format!("icon SVG: {e}"))?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or("empty icon size")?;
    let size = tree.size();
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / size.width(),
        height as f32 / size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok(pixmap)
}

fn png_rgba(pixmap: &tiny_skia::Pixmap) -> Result<Vec<u8>> {
    pixmap.encode_png().map_err(|e| format!("PNG: {e}"))
}

/// An opaque PNG without an alpha channel, which App Store icons must be.
fn png_rgb(pixmap: &tiny_skia::Pixmap) -> Result<Vec<u8>> {
    let mut rgb = Vec::with_capacity(pixmap.data().len() / 4 * 3);
    for pixel in pixmap.data().as_chunks::<4>().0 {
        if pixel[3] != 255 {
            return Err("the full-bleed icon has transparent pixels".into());
        }
        rgb.extend_from_slice(&pixel[..3]);
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, pixmap.width(), pixmap.height());
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| format!("PNG: {e}"))?;
    writer
        .write_image_data(&rgb)
        .map_err(|e| format!("PNG: {e}"))?;
    writer.finish().map_err(|e| format!("PNG: {e}"))?;
    Ok(out)
}

/// A Windows icon file holding one PNG per size.
fn ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let count = u16::try_from(images.len()).expect("a handful of sizes");
    let mut out = vec![0, 0, 1, 0];
    out.extend_from_slice(&count.to_le_bytes());
    let mut offset = 6 + 16 * u32::from(count);
    for (px, png) in images {
        // 0 means 256 pixels.
        let side = u8::try_from(*px).unwrap_or(0);
        let len = u32::try_from(png.len()).expect("icons are small");
        out.extend_from_slice(&[side, side, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes()); // colour planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += len;
    }
    for (_, png) in images {
        out.extend_from_slice(png);
    }
    out
}

/// Renders every image an asset catalog's `Contents.json` lists, at its pixel
/// size.
fn asset_catalog(dir: &Path, png: impl Fn(u32) -> Result<Vec<u8>>) -> Result<()> {
    let path = dir.join("Contents.json");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let contents: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let images = contents["images"]
        .as_array()
        .ok_or_else(|| format!("{}: no images", path.display()))?;
    let mut done = Vec::new();
    for image in images {
        let Some(name) = image["filename"].as_str() else {
            continue;
        };
        if done.contains(&name) {
            continue;
        }
        let points: f64 = image["size"]
            .as_str()
            .and_then(|s| s.split('x').next())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("{name}: no size"))?;
        let scale: f64 = image["scale"]
            .as_str()
            .and_then(|s| s.trim_end_matches('x').parse().ok())
            .unwrap_or(1.0);
        write(&dir.join(name), &png((points * scale).round() as u32)?)?;
        done.push(name);
    }
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}
