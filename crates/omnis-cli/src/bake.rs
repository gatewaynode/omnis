//! `omnis-cli tileset bake`: generate the first-person viewport slot sprites of a tileset from
//! flat 16×16 textures, and write the tileset file that declares them (ARCHITECTURE.md §8.3;
//! `tasks/TODO.md` M1). No first-person crawler art exists in the placeholder sets, so front
//! walls are tiled and scaled per depth, side walls are sheared into trapezoids, and floors
//! and ceilings are perspective-mapped bands. A block (pillar, tree clump) is its near face
//! plus the side face toward the party; a door frame is a front face with the opening cut out.
//! Hand-drawn art replaces any slot later by pointing its path elsewhere; the game never
//! bakes at runtime.
//!
//! Geometry: the eye sits at the near edge of the party's tile, half a tile up, looking down
//! the facing axis. The far edge of the tile at depth `d` is at distance `d + 1`. A point at
//! lateral offset `x` and height `h` at distance `z` lands at screen
//! `(cx + x·f/z, cy − (h − ½)·f/z)` with focal length `f = 0.9·viewport height`, so the far
//! edge of the party's own tile nearly fills the canvas and each deeper row shrinks by `1/z`.
//!
//! This is a build tool, not a simulation crate: floating point is fine here.

use omnis_data::ron_io::{read_ron, write_ron};
use omnis_data::{DataError, Slot, SlotKind, Surface, Tileset};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// A bake spec file: `packs/<pack>/bake/<name>.ron`.
#[derive(Debug, Clone, Deserialize)]
pub struct BakeSpec {
    /// Source sheet, relative to the spec file.
    pub sheet: String,
    /// Texture tile size in the sheet, in pixels.
    pub tile_size: u32,
    /// The tileset id to write.
    pub tileset_id: String,
    /// Output name: sprites go to `assets/tilesets/<name>/`, the file to `data/tiles/<name>.ron`.
    pub name: String,
    /// Canvas size the slots are laid out on.
    pub viewport: (u16, u16),
    /// How many canvas pixels a texture pixel covers at depth 0 on this viewport; a viewport
    /// baked at four times the size keeps its look with 4. Default 1.
    #[serde(default = "one")]
    pub texel_scale: u32,
    /// Rows drawn with sprites.
    pub detail_depth: u8,
    /// Lateral half-width of slots.
    pub width: u8,
    /// Surfaces to bake: name to kind and sheet tile.
    pub surfaces: BTreeMap<String, SurfaceSpec>,
}

const fn one() -> u32 {
    1
}

/// One surface in a bake spec.
#[derive(Debug, Clone, Deserialize)]
pub struct SurfaceSpec {
    /// What the surface paints.
    pub kind: SlotKind,
    /// Top-left pixel of the texture tile in the sheet.
    pub tile: (u32, u32),
}

/// What a bake produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BakeReport {
    /// The tileset file written.
    pub tileset_file: PathBuf,
    /// Where the sprites went.
    pub sprite_dir: PathBuf,
    /// How many sprites were written.
    pub sprites: usize,
}

impl fmt::Display for BakeReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "wrote {} sprites to {} and {}",
            self.sprites,
            self.sprite_dir.display(),
            self.tileset_file.display()
        )
    }
}

/// Why a bake failed.
#[derive(Debug)]
pub enum BakeError {
    /// The spec or tileset file could not be read or written.
    Data(DataError),
    /// A file operation failed.
    Io(PathBuf, std::io::Error),
    /// A PNG could not be decoded or encoded.
    Png(PathBuf, String),
    /// The spec is inconsistent.
    Spec(String),
}

impl fmt::Display for BakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BakeError::Data(e) => write!(f, "{e}"),
            BakeError::Io(p, e) => write!(f, "{}: {e}", p.display()),
            BakeError::Png(p, e) => write!(f, "{}: {e}", p.display()),
            BakeError::Spec(e) => write!(f, "spec: {e}"),
        }
    }
}

impl std::error::Error for BakeError {}

impl From<DataError> for BakeError {
    fn from(e: DataError) -> Self {
        BakeError::Data(e)
    }
}

/// Bake a spec into the pack that contains it (`<pack>/bake/<spec>.ron`).
pub fn bake_spec(spec_path: &Path) -> Result<BakeReport, BakeError> {
    let pack_root = spec_path.parent().and_then(Path::parent).ok_or_else(|| {
        BakeError::Spec(format!(
            "{} is not inside a pack's bake/ directory",
            spec_path.display()
        ))
    })?;
    bake_to(spec_path, pack_root)
}

/// Bake a spec, writing sprites and the tileset file under `pack_root`.
pub fn bake_to(spec_path: &Path, pack_root: &Path) -> Result<BakeReport, BakeError> {
    let spec: BakeSpec = read_ron(spec_path, spec_path)?;
    let spec_dir = spec_path.parent().unwrap_or(Path::new("."));
    let sheet = load_png(&spec_dir.join(&spec.sheet))?;
    if spec.detail_depth == 0 || spec.tile_size == 0 || spec.viewport.0 == 0 || spec.viewport.1 == 0
    {
        return Err(BakeError::Spec(
            "detail_depth, tile_size, and viewport must be non-zero".into(),
        ));
    }
    let geometry = Geometry::new(spec.viewport, spec.tile_size, spec.texel_scale);
    let sprite_dir = pack_root.join("assets/tilesets").join(&spec.name);
    clear_pngs(&sprite_dir)?;
    std::fs::create_dir_all(&sprite_dir).map_err(|e| BakeError::Io(sprite_dir.clone(), e))?;

    let mut surfaces = BTreeMap::new();
    let mut count = 0;
    for (name, surface) in &spec.surfaces {
        let texture = sheet
            .crop(
                surface.tile.0,
                surface.tile.1,
                spec.tile_size,
                spec.tile_size,
            )
            .ok_or_else(|| {
                BakeError::Spec(format!(
                    "surface '{name}' tile {:?} is outside the {}x{} sheet",
                    surface.tile, sheet.width, sheet.height
                ))
            })?;
        let mut slots = Vec::new();
        for (depth, offset) in slot_positions(spec.detail_depth, spec.width, surface.kind) {
            let Some((sprite, x, y)) = render(&geometry, surface.kind, depth, offset, &texture)
            else {
                continue;
            };
            let file = format!("{name}_d{depth}_o{offset}.png");
            save_png(&sprite_dir.join(&file), &sprite)?;
            count += 1;
            slots.push(Slot {
                depth,
                offset,
                path: format!("assets/tilesets/{}/{file}", spec.name),
                x,
                y,
            });
        }
        surfaces.insert(
            name.clone(),
            Surface {
                kind: surface.kind,
                slots,
            },
        );
    }
    let tileset = Tileset {
        schema: omnis_data::SCHEMA,
        id: spec.tileset_id.clone(),
        detail_depth: spec.detail_depth,
        width: spec.width,
        viewport: spec.viewport,
        surfaces,
    };
    let tileset_file = pack_root
        .join("data/tiles")
        .join(format!("{}.ron", spec.name));
    write_ron(&tileset_file, &tileset)?;
    Ok(BakeReport {
        tileset_file,
        sprite_dir,
        sprites: count,
    })
}

fn clear_pngs(dir: &Path) -> Result<(), BakeError> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "png") {
            std::fs::remove_file(&path).map_err(|e| BakeError::Io(path, e))?;
        }
    }
    Ok(())
}

/// The `(depth, offset)` pairs a surface kind needs: every cone position for floors,
/// ceilings, front walls, and doors; only the party's side for side walls; never the
/// party's own tile for blocks. Like the cone, row `d` spans offsets `-(d + 1)..=(d + 1)`,
/// clamped to the width; slots that fall off-screen are dropped by `render`.
#[must_use]
pub fn slot_positions(detail_depth: u8, width: u8, kind: SlotKind) -> Vec<(u8, i8)> {
    let mut out = Vec::new();
    for depth in 0..detail_depth {
        let half = i8::try_from(depth.saturating_add(1).min(width)).unwrap_or(i8::MAX);
        for offset in -half..=half {
            let keep = match kind {
                SlotKind::WallLeft => offset <= 0,
                SlotKind::WallRight => offset >= 0,
                SlotKind::Block => depth > 0 || offset != 0,
                SlotKind::Object | SlotKind::Monster => false,
                _ => true,
            };
            if keep {
                out.push((depth, offset));
            }
        }
    }
    out
}

/// An RGBA8 image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major RGBA bytes.
    pub rgba: Vec<u8>,
}

impl Image {
    fn new(width: u32, height: u32) -> Image {
        Image {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    fn get(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    fn put(&mut self, x: u32, y: u32, px: [u8; 4]) {
        let i = ((y * self.width + x) * 4) as usize;
        self.rgba[i..i + 4].copy_from_slice(&px);
    }

    fn crop(&self, x0: u32, y0: u32, w: u32, h: u32) -> Option<Image> {
        if x0.checked_add(w)? > self.width || y0.checked_add(h)? > self.height {
            return None;
        }
        let mut out = Image::new(w, h);
        for y in 0..h {
            for x in 0..w {
                out.put(x, y, self.get(x0 + x, y0 + y));
            }
        }
        Some(out)
    }

    /// The smallest box holding every non-transparent pixel.
    fn bounds(&self) -> Option<(u32, u32, u32, u32)> {
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y)[3] != 0 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        (x0 != u32::MAX).then(|| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
    }

    /// Nearest-neighbour sample with wrap-around, in texture pixels.
    fn wrap_sample(&self, u: f64, v: f64) -> [u8; 4] {
        let x = u.floor().rem_euclid(f64::from(self.width)) as u32;
        let y = v.floor().rem_euclid(f64::from(self.height)) as u32;
        self.get(x, y)
    }
}

/// Camera and canvas numbers shared by every slot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    width: u32,
    height: u32,
    cx: f64,
    cy: f64,
    focal: f64,
    /// Texture pixels per world unit, so depth 0 shows the texture at roughly 1:1.
    texels_per_unit: f64,
}

impl Geometry {
    /// For a canvas, a texture size, and how many canvas pixels a texel covers at depth 0.
    #[must_use]
    pub fn new(viewport: (u16, u16), tile_size: u32, texel_scale: u32) -> Geometry {
        let (w, h) = (f64::from(viewport.0), f64::from(viewport.1));
        let focal = 0.9 * h;
        let texel = f64::from(tile_size) * f64::from(texel_scale.max(1));
        let repeats = (focal / texel).round().max(1.0);
        Geometry {
            width: u32::from(viewport.0),
            height: u32::from(viewport.1),
            cx: w / 2.0,
            cy: h / 2.0,
            focal,
            texels_per_unit: repeats * f64::from(tile_size),
        }
    }

    /// Screen x of lateral world x at distance z.
    fn sx(&self, x: f64, z: f64) -> f64 {
        self.cx + x * self.focal / z
    }

    /// Screen y of world height h (0 floor, 1 ceiling) at distance z.
    fn sy(&self, h: f64, z: f64) -> f64 {
        self.cy - (h - 0.5) * self.focal / z
    }

    /// Pixel columns whose centres lie in `[a, b)` in either order, clamped to the canvas.
    fn columns(&self, a: f64, b: f64) -> std::ops::Range<u32> {
        let (lo, hi) = (a.min(b), a.max(b));
        let start = (lo - 0.5).ceil().max(0.0) as u32;
        let end = ((hi - 0.5).ceil().max(0.0) as u32).min(self.width);
        start.min(end)..end
    }

    fn rows(&self, a: f64, b: f64) -> std::ops::Range<u32> {
        let (lo, hi) = (a.min(b), a.max(b));
        let start = (lo - 0.5).ceil().max(0.0) as u32;
        let end = ((hi - 0.5).ceil().max(0.0) as u32).min(self.height);
        start.min(end)..end
    }
}

/// Render one slot onto a canvas and crop it. `None` when nothing is visible.
#[must_use]
pub fn render(
    geo: &Geometry,
    kind: SlotKind,
    depth: u8,
    offset: i8,
    texture: &Image,
) -> Option<(Image, i16, i16)> {
    let mut canvas = Image::new(geo.width, geo.height);
    let (d, o) = (f64::from(depth), f64::from(offset));
    match kind {
        SlotKind::WallFront => front(geo, &mut canvas, d, o, texture, geo.texels_per_unit),
        SlotKind::Door => front(geo, &mut canvas, d, o, texture, f64::from(texture.width)),
        SlotKind::DoorFrame => {
            front(geo, &mut canvas, d, o, texture, geo.texels_per_unit);
            punch(geo, &mut canvas, d, o);
        }
        SlotKind::WallLeft => side(geo, &mut canvas, d, o - 0.5, texture),
        SlotKind::WallRight => side(geo, &mut canvas, d, o + 0.5, texture),
        SlotKind::Floor => band(geo, &mut canvas, d, o, texture, 0.0),
        SlotKind::Ceiling => band(geo, &mut canvas, d, o, texture, 1.0),
        SlotKind::Block if depth == 0 && offset == 0 => return None,
        SlotKind::Block => {
            if o > 0.0 {
                side(geo, &mut canvas, d, o - 0.5, texture);
            } else if o < 0.0 {
                side(geo, &mut canvas, d, o + 0.5, texture);
            }
            // The near face stands at distance `d`: the far edge of the tile before it. A
            // block beside the party has its near face in the camera plane, so only its
            // side shows.
            if depth > 0 {
                front(geo, &mut canvas, d - 1.0, o, texture, geo.texels_per_unit);
            }
        }
        SlotKind::Object | SlotKind::Monster => return None,
    }
    let (x, y, w, h) = canvas.bounds()?;
    let sprite = canvas.crop(x, y, w, h)?;
    Some((sprite, i16::try_from(x).ok()?, i16::try_from(y).ok()?))
}

/// The far edge of tile `(d, o)`: a rectangle at distance `d + 1`. `texels` is how many
/// texture pixels span one world unit: the tiling density for walls, or the texture width
/// for a door so its single image stretches across the edge.
fn front(geo: &Geometry, canvas: &mut Image, d: f64, o: f64, tex: &Image, texels: f64) {
    let z = d + 1.0;
    let (x0, x1) = (geo.sx(o - 0.5, z), geo.sx(o + 0.5, z));
    let (y0, y1) = (geo.sy(1.0, z), geo.sy(0.0, z));
    let scale = geo.focal / z;
    for y in geo.rows(y0, y1) {
        for x in geo.columns(x0, x1) {
            let u = (f64::from(x) + 0.5 - x0) / scale * texels;
            let v = (f64::from(y) + 0.5 - y0) / scale * texels;
            canvas.put(x, y, tex.wrap_sample(u, v));
        }
    }
}

/// Clear the opening of a door frame on the far edge of tile `(d, o)`: everything but a jamb
/// on each side and a lintel above, each an eighth of a tile.
fn punch(geo: &Geometry, canvas: &mut Image, d: f64, o: f64) {
    let z = d + 1.0;
    let (x0, x1) = (geo.sx(o - 0.375, z), geo.sx(o + 0.375, z));
    let (y0, y1) = (geo.sy(0.875, z), geo.sy(0.0, z));
    for y in geo.rows(y0, y1) {
        for x in geo.columns(x0, x1) {
            canvas.put(x, y, [0; 4]);
        }
    }
}

/// The wall plane at lateral `xw` from distance `d` to `d + 1`, seen at an angle.
fn side(geo: &Geometry, canvas: &mut Image, d: f64, xw: f64, tex: &Image) {
    if xw == 0.0 {
        return;
    }
    let far = geo.sx(xw, d + 1.0);
    let near = if d == 0.0 {
        if xw < 0.0 {
            -1.0
        } else {
            f64::from(geo.width) + 1.0
        }
    } else {
        geo.sx(xw, d)
    };
    for x in geo.columns(near, far) {
        let dx = f64::from(x) + 0.5 - geo.cx;
        if dx == 0.0 || (dx < 0.0) != (xw < 0.0) {
            continue;
        }
        let z = xw * geo.focal / dx;
        if z < d || z > d + 1.0 {
            continue;
        }
        let scale = geo.focal / z;
        let y0 = geo.sy(1.0, z);
        let u = (z - d) * geo.texels_per_unit;
        for y in geo.rows(y0, geo.sy(0.0, z)) {
            let v = (f64::from(y) + 0.5 - y0) / scale * geo.texels_per_unit;
            canvas.put(x, y, tex.wrap_sample(u, v));
        }
    }
}

/// The floor (`h = 0`) or ceiling (`h = 1`) of tile `(d, o)`: the band between distances
/// `d` and `d + 1`, perspective-mapped row by row.
fn band(geo: &Geometry, canvas: &mut Image, d: f64, o: f64, tex: &Image, h: f64) {
    let (y_far, y_near) = (
        geo.sy(h, d + 1.0),
        if d == 0.0 {
            if h == 0.0 {
                f64::from(geo.height) + 1.0
            } else {
                -1.0
            }
        } else {
            geo.sy(h, d)
        },
    );
    for y in geo.rows(y_far, y_near) {
        let dy = (f64::from(y) + 0.5 - geo.cy) * if h == 0.0 { 1.0 } else { -1.0 };
        if dy <= 0.0 {
            continue;
        }
        let z = 0.5 * geo.focal / dy;
        if z < d || z > d + 1.0 {
            continue;
        }
        let scale = geo.focal / z;
        let x0 = geo.sx(o - 0.5, z);
        let v = (z - d) * geo.texels_per_unit;
        for x in geo.columns(x0, geo.sx(o + 0.5, z)) {
            let u = (f64::from(x) + 0.5 - x0) / scale * geo.texels_per_unit;
            canvas.put(x, y, tex.wrap_sample(u, v));
        }
    }
}

/// Decode a PNG of any colour type to RGBA8.
pub fn load_png(path: &Path) -> Result<Image, BakeError> {
    let file = File::open(path).map_err(|e| BakeError::Io(path.to_path_buf(), e))?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::ALPHA | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| BakeError::Png(path.to_path_buf(), e.to_string()))?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| BakeError::Png(path.to_path_buf(), "image too large".into()))?;
    let mut buf = vec![0; size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| BakeError::Png(path.to_path_buf(), e.to_string()))?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        other => {
            return Err(BakeError::Png(
                path.to_path_buf(),
                format!("unexpected colour type {other:?} after expansion"),
            ));
        }
    };
    Ok(Image {
        width: info.width,
        height: info.height,
        rgba,
    })
}

/// Encode an RGBA8 image.
pub fn save_png(path: &Path, image: &Image) -> Result<(), BakeError> {
    let file = File::create(path).map_err(|e| BakeError::Io(path.to_path_buf(), e))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| BakeError::Png(path.to_path_buf(), e.to_string()))?;
    writer
        .write_image_data(&image.rgba)
        .map_err(|e| BakeError::Png(path.to_path_buf(), e.to_string()))?;
    writer
        .finish()
        .map_err(|e| BakeError::Png(path.to_path_buf(), e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker() -> Image {
        let mut t = Image::new(16, 16);
        for y in 0..16 {
            for x in 0..16 {
                let dark = (x / 8 + y / 8) % 2 == 0;
                t.put(
                    x,
                    y,
                    if dark {
                        [40, 40, 40, 255]
                    } else {
                        [200, 200, 200, 255]
                    },
                );
            }
        }
        t
    }

    #[test]
    fn slot_positions_follow_the_cone_and_the_side() {
        assert_eq!(
            slot_positions(2, 3, SlotKind::Floor),
            [
                (0, -1),
                (0, 0),
                (0, 1),
                (1, -2),
                (1, -1),
                (1, 0),
                (1, 1),
                (1, 2)
            ],
            "each row is one tile wider than the diagonal"
        );
        assert_eq!(
            slot_positions(2, 3, SlotKind::WallLeft),
            [(0, -1), (0, 0), (1, -2), (1, -1), (1, 0)]
        );
        assert_eq!(
            slot_positions(2, 3, SlotKind::WallRight),
            [(0, 0), (0, 1), (1, 0), (1, 1), (1, 2)]
        );
        assert_eq!(
            slot_positions(4, 1, SlotKind::Door).len(),
            3 + 3 + 3 + 3,
            "width clamps the cone"
        );
        assert!(slot_positions(4, 3, SlotKind::Monster).is_empty());
        assert_eq!(
            slot_positions(2, 3, SlotKind::Block),
            [(0, -1), (0, 1), (1, -2), (1, -1), (1, 0), (1, 1), (1, 2)],
            "the party never stands in a block"
        );
    }

    #[test]
    fn a_texel_scale_keeps_the_look_of_a_smaller_viewport() {
        let old = Geometry::new((240, 135), 16, 1);
        let four = Geometry::new((960, 540), 16, 4);
        assert_eq!(
            four.texels_per_unit, old.texels_per_unit,
            "eight repeats a face"
        );
        let fine = Geometry::new((960, 540), 16, 1);
        assert!(
            fine.texels_per_unit > 3.0 * old.texels_per_unit,
            "finer bricks"
        );
        let tex = checker();
        let (small, sx, sy) = render(&old, SlotKind::WallFront, 0, 0, &tex).unwrap();
        let (big, bx, by) = render(&four, SlotKind::WallFront, 0, 0, &tex).unwrap();
        // The face sits four times as far from the corner and is four times as large, to
        // the rounding of a pixel at each size.
        let near = |a: i32, b: i32| (a - 4 * b).abs() <= 4;
        assert!(near(i32::from(bx), i32::from(sx)) && near(i32::from(by), i32::from(sy)));
        assert!(
            near(big.width as i32, small.width as i32),
            "{} vs {}",
            big.width,
            small.width
        );
        assert!(near(big.height as i32, small.height as i32));
    }

    #[test]
    fn the_tiles_beside_the_party_reach_the_canvas_edges() {
        let geo = Geometry::new((240, 135), 16, 1);
        let tex = checker();
        let (floor, x, _) = render(&geo, SlotKind::Floor, 0, 1, &tex).unwrap();
        assert_eq!(x + i16::try_from(floor.width).unwrap(), 240);
        assert_eq!(x, 181, "starts where the party's own floor ends");
        let (front, fx, fy) = render(&geo, SlotKind::WallFront, 0, 1, &tex).unwrap();
        let (own, _, oy) = render(&geo, SlotKind::WallFront, 0, 0, &tex).unwrap();
        assert_eq!((fx, fy, front.height), (181, oy, own.height), "same plane");
        assert_eq!(fx + i16::try_from(front.width).unwrap(), 240, "clipped");
        assert!(
            render(&geo, SlotKind::WallLeft, 0, -1, &tex).is_none(),
            "a neighbour's far side wall lies off-screen"
        );
        assert!(render(&geo, SlotKind::WallRight, 0, 1, &tex).is_none());
        let (far, x, _) = render(&geo, SlotKind::Floor, 1, 2, &tex).unwrap();
        assert_eq!(x + i16::try_from(far.width).unwrap(), 240);
        assert!(
            (211..=213).contains(&x),
            "row 1's outer tile: a wedge about 29 px wide at its far edge, not {x}"
        );
        assert!(render(&geo, SlotKind::WallRight, 1, 2, &tex).is_none());
    }

    #[test]
    fn blocks_show_the_near_face_and_the_side_toward_the_party() {
        let geo = Geometry::new((240, 135), 16, 1);
        let tex = checker();
        assert!(render(&geo, SlotKind::Block, 0, 0, &tex).is_none());
        assert_eq!(
            render(&geo, SlotKind::Block, 0, 1, &tex),
            render(&geo, SlotKind::WallRight, 0, 0, &tex),
            "a block beside the party is its side wall"
        );
        assert_eq!(
            render(&geo, SlotKind::Block, 0, -1, &tex),
            render(&geo, SlotKind::WallLeft, 0, 0, &tex)
        );
        assert_eq!(
            render(&geo, SlotKind::Block, 1, 0, &tex),
            render(&geo, SlotKind::WallFront, 0, 0, &tex),
            "dead ahead, a block is its near face alone, on the party's far edge"
        );
        let (right, rx, _) = render(&geo, SlotKind::Block, 1, 1, &tex).unwrap();
        let (face, fx, _) = render(&geo, SlotKind::WallFront, 0, 1, &tex).unwrap();
        assert!(rx < fx, "the side face reaches toward the centre");
        assert_eq!(
            rx + i16::try_from(right.width).unwrap(),
            fx + i16::try_from(face.width).unwrap(),
            "and the near face ends where the wall would"
        );
        let (left, lx, _) = render(&geo, SlotKind::Block, 1, -1, &tex).unwrap();
        assert!(
            (lx + i16::try_from(left.width).unwrap()).abs_diff(240 - rx) <= 1,
            "mirror image on the left, up to pixel phase"
        );
    }

    #[test]
    fn door_frames_are_front_faces_with_the_opening_cut_out() {
        let geo = Geometry::new((240, 135), 16, 1);
        let tex = checker();
        let (frame, x, y) = render(&geo, SlotKind::DoorFrame, 0, 0, &tex).unwrap();
        let (wall, wx, wy) = render(&geo, SlotKind::WallFront, 0, 0, &tex).unwrap();
        assert_eq!(
            (x, y, frame.width, frame.height),
            (wx, wy, wall.width, wall.height),
            "same outline as the wall"
        );
        let (cx, cy) = (frame.width / 2, frame.height / 2);
        assert_eq!(frame.get(cx, cy)[3], 0, "open in the middle");
        assert_eq!(
            frame.get(cx, frame.height - 1)[3],
            0,
            "open down to the floor"
        );
        assert_ne!(frame.get(cx, 0)[3], 0, "a lintel above");
        let jamb = frame.width / 8;
        assert_ne!(frame.get(jamb - 2, cy)[3], 0, "a jamb on the left");
        assert_eq!(frame.get(jamb + 1, cy)[3], 0);
        assert_ne!(
            frame.get(frame.width - jamb + 1, cy)[3],
            0,
            "and on the right"
        );
        assert_eq!(frame.get(frame.width - jamb - 2, cy)[3], 0);
    }

    #[test]
    fn front_walls_are_centred_and_shrink_with_depth() {
        let geo = Geometry::new((240, 135), 16, 1);
        let tex = checker();
        let (d0, x0, y0) = render(&geo, SlotKind::WallFront, 0, 0, &tex).unwrap();
        let (d1, x1, y1) = render(&geo, SlotKind::WallFront, 1, 0, &tex).unwrap();
        assert_eq!(
            (x0, y0, d0.width, d0.height),
            (59, 7, 122, 121),
            "focal 0.9·135 = 121.5 px per unit at depth 0, pixel centres inside [59.25, 180.75)"
        );
        assert!(d1.width < d0.width && d1.height < d0.height);
        assert_eq!(
            x1 + i16::try_from(d1.width).unwrap() / 2,
            120,
            "still centred"
        );
        assert!(y1 > y0);
        let (right, xr, _) = render(&geo, SlotKind::WallFront, 1, 1, &tex).unwrap();
        assert_eq!(
            xr,
            x1 + i16::try_from(d1.width).unwrap(),
            "the next tile starts where this one ends"
        );
        assert!(
            right.width.abs_diff(d1.width) <= 1,
            "equal spans, up to pixel phase"
        );
    }

    #[test]
    fn side_walls_lean_toward_the_centre_and_bands_sit_at_the_edges() {
        let geo = Geometry::new((240, 135), 16, 1);
        let tex = checker();
        let (left, x, _) = render(&geo, SlotKind::WallLeft, 0, 0, &tex).unwrap();
        assert_eq!(x, 0, "the party's own left wall starts at the canvas edge");
        assert!(
            left.get(0, 0)[3] != 0 && left.get(left.width - 1, 0)[3] == 0,
            "top edge slopes down toward the far end"
        );
        assert_eq!(
            render(&geo, SlotKind::WallLeft, 1, 1, &tex),
            render(&geo, SlotKind::WallRight, 1, 0, &tex),
            "a tile's left edge is its neighbour's right edge: one plane, one sprite"
        );
        let (floor, _, fy) = render(&geo, SlotKind::Floor, 1, 0, &tex).unwrap();
        let (ceiling, _, cy) = render(&geo, SlotKind::Ceiling, 1, 0, &tex).unwrap();
        assert!(fy > 67 && cy < 67, "floor below the horizon, ceiling above");
        assert_eq!(floor.height, ceiling.height, "symmetric about the eye");
        let (own_floor, _, oy) = render(&geo, SlotKind::Floor, 0, 0, &tex).unwrap();
        assert_eq!(
            i64::from(oy) + i64::from(own_floor.height),
            135,
            "the party's floor reaches the bottom edge"
        );
    }
}
