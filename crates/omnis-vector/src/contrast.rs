//! Whether a palette keeps its roles apart (presentation-ARCHITECTURE 7.3, presentation-PRD
//! 5.2 and XR8): each role the world's lines carry must stand off the dark ground and off every
//! other, as seen in full colour and by a player with one of the three colour deficiencies.
//! Colour is never the only channel (X19), but a palette that collapses two roles for some
//! players is refused all the same. Bevy-free arithmetic on a [`Palette`].
//!
//! An ink brighter than the screen is brought within it first, its hue kept, since that is what
//! a line's core shows; the deficiencies are the matrices of Machado, Oliveira and Fernandes
//! (2009) at full severity, on linear light; the distance is Euclidean in Oklab.

use crate::style::Palette;
use omnis_sim::omnis_data::ColourRole;

/// How a palette is looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vision {
    /// All three cones.
    Full,
    /// No long-wavelength cones (red-blind).
    Protanopia,
    /// No medium-wavelength cones (green-blind).
    Deuteranopia,
    /// No short-wavelength cones (blue-blind).
    Tritanopia,
}

impl Vision {
    /// Every way a palette is checked.
    pub const ALL: [Vision; 4] = [
        Vision::Full,
        Vision::Protanopia,
        Vision::Deuteranopia,
        Vision::Tritanopia,
    ];

    /// Linear light as this vision takes it.
    #[must_use]
    pub fn sees(self, [r, g, b]: [f32; 3]) -> [f32; 3] {
        let m: [[f32; 3]; 3] = match self {
            Vision::Full => return [r, g, b],
            Vision::Protanopia => [
                [0.152_286, 1.052_583, -0.204_868],
                [0.114_503, 0.786_281, 0.099_216],
                [-0.003_882, -0.048_116, 1.051_998],
            ],
            Vision::Deuteranopia => [
                [0.367_322, 0.860_646, -0.227_968],
                [0.280_085, 0.672_501, 0.047_413],
                [-0.011_820, 0.042_940, 0.968_881],
            ],
            Vision::Tritanopia => [
                [1.255_528, -0.076_749, -0.178_779],
                [-0.078_411, 0.930_809, 0.147_602],
                [0.004_733, 0.691_367, 0.303_900],
            ],
        };
        m.map(|row| (row[0] * r + row[1] * g + row[2] * b).clamp(0.0, 1.0))
    }
}

/// The roles some line of the world is drawn in: every kind's own, and magic, which a terrain
/// may name. The party's and the interface's draw no line in the world yet; they are held off
/// the dark only.
pub const DRAWN: [ColourRole; 7] = [
    ColourRole::Structure,
    ColourRole::Ground,
    ColourRole::Passage,
    ColourRole::Thing,
    ColourRole::Hostile,
    ColourRole::Magic,
    ColourRole::Danger,
];

/// The least distance between two drawn roles, in Oklab: five times what the eye can just
/// tell apart side by side, since these are thin lines on black, seen apart.
pub const APART: f32 = 0.1;

/// The least distance of any role from the dark ground, in Oklab. A pure red, which a
/// red-blind player sees darkest of the bright hues, still clears it (0.49).
pub const FROM_DARK: f32 = 0.45;

/// Two roles too alike, or one too near the dark, under one vision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clash {
    /// As seen how.
    pub vision: Vision,
    /// The role.
    pub role: ColourRole,
    /// The role it is too like; `None` is the dark ground.
    pub other: Option<ColourRole>,
    /// How far apart they are.
    pub distance: f32,
}

/// An ink within the screen's range, its hue kept: what the core of its line shows.
#[must_use]
pub fn shown([r, g, b]: [f32; 3]) -> [f32; 3] {
    let peak = r.max(g).max(b).max(1.0);
    [r / peak, g / peak, b / peak]
}

/// Linear light in Oklab (Ottosson, 2020).
#[must_use]
pub fn oklab([r, g, b]: [f32; 3]) -> [f32; 3] {
    let l = (0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

/// How far apart two inks are as `vision` sees them.
#[must_use]
pub fn distance(a: [f32; 3], b: [f32; 3], vision: Vision) -> f32 {
    let (a, b) = (oklab(vision.sees(shown(a))), oklab(vision.sees(shown(b))));
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Every pair of drawn roles nearer than [`APART`] and every role nearer the dark than
/// [`FROM_DARK`], under each vision; empty for a palette that keeps its roles apart.
#[must_use]
pub fn clashes(palette: &Palette) -> Vec<Clash> {
    let mut out = Vec::new();
    for vision in Vision::ALL {
        for role in ColourRole::ALL {
            let distance = distance(palette.ink(role), [0.0; 3], vision);
            if distance < FROM_DARK {
                out.push(Clash {
                    vision,
                    role,
                    other: None,
                    distance,
                });
            }
        }
        for (i, role) in DRAWN.iter().enumerate() {
            for other in &DRAWN[i + 1..] {
                let distance = distance(palette.ink(*role), palette.ink(*other), vision);
                if distance < APART {
                    out.push(Clash {
                        vision,
                        role: *role,
                        other: Some(*other),
                        distance,
                    });
                }
            }
        }
    }
    out
}
