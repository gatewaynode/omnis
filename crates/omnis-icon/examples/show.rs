//! Print an icon of the set as text: `cargo run -p omnis-icon --example show -- door-open`.

fn main() {
    let name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "door-open".to_owned());
    let Some(icon) = omnis_icon::get(&name) else {
        eprintln!(
            "no icon named {name}; the set has {} icons",
            omnis_icon::catalogue::count()
        );
        std::process::exit(1);
    };
    // Two columns per unit, one row per unit: a terminal's cells are about twice as tall.
    let (cols, rows) = (48_usize, 24_usize);
    let mut grid = vec![vec![b' '; cols]; rows];
    for [a, b] in icon.segments(24.0) {
        let steps = ((b[0] - a[0]).abs().max((b[1] - a[1]).abs()) * 4.0)
            .ceil()
            .max(1.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        for i in 0..=steps as usize {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f32 / steps;
            let x = a[0] + (b[0] - a[0]) * t;
            let y = a[1] + (b[1] - a[1]) * t;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (c, r) = ((x * 2.0) as usize, y as usize);
            if r < rows && c < cols {
                grid[r][c] = b'#';
            }
        }
    }
    println!("{name} ({} strokes)", icon.strokes.len());
    for row in grid {
        println!("{}", String::from_utf8_lossy(&row).trim_end());
    }
}
