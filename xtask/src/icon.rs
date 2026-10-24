//! Generates `packaging/AppIcon.icns`: the Markdown mark on a blue rounded
//! square, drawn at every size of the icon set.

use std::fs;
use std::path::Path;
use std::process::Command;

use tiny_skia::{
    Color, FillRule, GradientStop, LineJoin, LinearGradient, Paint, Path as Shape, PathBuilder,
    Pixmap, Point, SpreadMode, Stroke, Transform,
};

/// Design canvas size; every drawing coordinate below uses this grid.
const CANVAS: f32 = 1024.0;

/// Pixel sizes of the icon set entries (`icon_<points>x<points>[@2x].png`).
const ENTRIES: [(u32, u32); 10] = [
    (16, 1),
    (16, 2),
    (32, 1),
    (32, 2),
    (128, 1),
    (128, 2),
    (256, 1),
    (256, 2),
    (512, 1),
    (512, 2),
];

pub fn generate(packaging: &Path) -> Result<(), String> {
    let iconset = packaging.join("AppIcon.iconset");
    fs::create_dir_all(&iconset).map_err(|e| e.to_string())?;
    for (points, scale) in ENTRIES {
        let pixels = points * scale;
        let suffix = if scale == 2 { "@2x" } else { "" };
        let file = iconset.join(format!("icon_{points}x{points}{suffix}.png"));
        render(pixels)?
            .save_png(&file)
            .map_err(|e| format!("cannot write {}: {e}", file.display()))?;
    }
    let output = packaging.join("AppIcon.icns");
    let status = Command::new("iconutil")
        .args(["--convert", "icns", "--output"])
        .arg(&output)
        .arg(&iconset)
        .status()
        .map_err(|e| e.to_string())?;
    fs::remove_dir_all(&iconset).map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("iconutil failed with {status}"));
    }
    println!("{}", output.display());
    Ok(())
}

fn render(pixels: u32) -> Result<Pixmap, String> {
    let mut pixmap = Pixmap::new(pixels, pixels).ok_or("invalid icon size")?;
    let transform = Transform::from_scale(pixels as f32 / CANVAS, pixels as f32 / CANVAS);

    // Rounded square on the macOS icon grid: 824 × 824 centered.
    let body = rounded_rect(100.0, 100.0, 824.0, 824.0, 185.0).ok_or("invalid body")?;
    let gradient = LinearGradient::new(
        Point::from_xy(512.0, 100.0),
        Point::from_xy(512.0, 924.0),
        vec![
            GradientStop::new(0.0, Color::from_rgba8(0x5b, 0x7c, 0xff, 0xff)),
            GradientStop::new(1.0, Color::from_rgba8(0x2a, 0x2f, 0x9e, 0xff)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    )
    .ok_or("invalid gradient")?;
    let background = Paint {
        shader: gradient,
        anti_alias: true,
        ..Paint::default()
    };
    pixmap.fill_path(&body, &background, FillRule::Winding, transform, None);

    // The Markdown mark (208 × 128 design units), scaled to 620 px wide.
    let scale = 620.0 / 208.0;
    let mark = transform
        .pre_translate(512.0 - 104.0 * scale, 512.0 - 64.0 * scale)
        .pre_scale(scale, scale);
    let mut white = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    white.set_color_rgba8(0xff, 0xff, 0xff, 0xff);

    let frame = rounded_rect(5.0, 5.0, 198.0, 118.0, 10.0).ok_or("invalid frame")?;
    let stroke = Stroke {
        width: 10.0,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    pixmap.stroke_path(&frame, &white, &stroke, mark, None);

    let letter_m = polygon(&[
        (30.0, 98.0),
        (30.0, 30.0),
        (50.0, 30.0),
        (70.0, 55.0),
        (90.0, 30.0),
        (110.0, 30.0),
        (110.0, 98.0),
        (90.0, 98.0),
        (90.0, 59.0),
        (70.0, 84.0),
        (50.0, 59.0),
        (50.0, 98.0),
    ])
    .ok_or("invalid letter")?;
    let arrow = polygon(&[
        (155.0, 98.0),
        (125.0, 65.0),
        (145.0, 65.0),
        (145.0, 30.0),
        (165.0, 30.0),
        (165.0, 65.0),
        (185.0, 65.0),
    ])
    .ok_or("invalid arrow")?;
    for shape in [&letter_m, &arrow] {
        pixmap.fill_path(shape, &white, FillRule::Winding, mark, None);
    }
    Ok(pixmap)
}

fn polygon(points: &[(f32, f32)]) -> Option<Shape> {
    let mut builder = PathBuilder::new();
    let (first, rest) = points.split_first()?;
    builder.move_to(first.0, first.1);
    for (x, y) in rest {
        builder.line_to(*x, *y);
    }
    builder.close();
    builder.finish()
}

fn rounded_rect(x: f32, y: f32, width: f32, height: f32, radius: f32) -> Option<Shape> {
    // Cubic approximation of a quarter circle.
    let k = radius * 0.552_284_8;
    let (right, bottom) = (x + width, y + height);
    let mut builder = PathBuilder::new();
    builder.move_to(x + radius, y);
    builder.line_to(right - radius, y);
    builder.cubic_to(
        right - radius + k,
        y,
        right,
        y + radius - k,
        right,
        y + radius,
    );
    builder.line_to(right, bottom - radius);
    builder.cubic_to(
        right,
        bottom - radius + k,
        right - radius + k,
        bottom,
        right - radius,
        bottom,
    );
    builder.line_to(x + radius, bottom);
    builder.cubic_to(
        x + radius - k,
        bottom,
        x,
        bottom - radius + k,
        x,
        bottom - radius,
    );
    builder.line_to(x, y + radius);
    builder.cubic_to(x, y + radius - k, x + radius - k, y, x + radius, y);
    builder.close();
    builder.finish()
}
