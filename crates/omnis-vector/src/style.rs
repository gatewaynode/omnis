//! The style system's seam (presentation-ARCHITECTURE §7): the palette of the world's lines by
//! colour role, the interface's roles and type scale, and the channels that carry meaning
//! without colour. Every screen asks a role and a size here and never names a colour or a pixel
//! size itself. The arena and the picture window take the palette (`tones`, `cinema::Inks`); the
//! minimap keeps its own colours until it is brought to it. Bevy-free: the shell turns these into `Color` and uniforms.

use omnis_sim::omnis_data::{BiosphereDef, ColourRole};

/// How many roles a palette has room for. More than there are roles, so a role added later
/// changes no shader layout.
pub const SLOTS: usize = 16;

/// The colour of each role of the world's lines in one place (presentation-PRD X18, X23): a
/// linear colour a role, its glow multiplied in, so a value above 1.0 blooms. The line material
/// holds one; changing biosphere changes it and no mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// A role's ink at its number ([`ColourRole::index`]); black where there is no role.
    pub inks: [[f32; 3]; SLOTS],
}

/// The palette where no biosphere is named: the hues the view had before the roles, one a role,
/// each moved as little as keeps the drawn roles apart for every kind of colour vision
/// (`contrast::clashes`; found by search from the old hues, 2026-11-13). The walls' green leans
/// to teal, a door's amber to yellow, a hole's red is purer.
const DEFAULT: [(ColourRole, [f32; 3]); 9] = [
    (ColourRole::Structure, [0.0, 3.0, 0.99]),
    (ColourRole::Ground, [0.08, 0.45, 0.35]),
    (ColourRole::Passage, [3.2, 2.08, 0.02]),
    (ColourRole::Thing, [1.85, 0.88, 2.8]),
    (ColourRole::Party, [0.52, 2.0, 0.76]),
    (ColourRole::Hostile, [3.0, 0.97, 0.13]),
    (ColourRole::Magic, [1.23, 1.75, 2.2]),
    (ColourRole::Danger, [2.6, 0.2, 0.23]),
    (ColourRole::Interface, [0.26, 1.0, 0.38]),
];

impl Default for Palette {
    fn default() -> Palette {
        let mut inks = [[0.0; 3]; SLOTS];
        for (role, ink) in DEFAULT {
            inks[role.index()] = ink;
        }
        Palette { inks }
    }
}

impl Palette {
    /// The palette of `biosphere`: each role it writes, from sRGB to linear and times its glow;
    /// each it leaves out, and every role where there is no biosphere, the default's.
    #[must_use]
    pub fn of(biosphere: Option<&BiosphereDef>) -> Palette {
        let mut palette = Palette::default();
        for (role, ink) in biosphere.iter().flat_map(|b| &b.palette) {
            let glow = f32::from(ink.glow) / 1000.0;
            let (r, g, b) = ink.rgb;
            palette.inks[role.index()] = [r, g, b].map(|c| linear(c) * glow);
        }
        palette
    }

    /// The ink of `role`.
    #[must_use]
    pub fn ink(&self, role: ColourRole) -> [f32; 3] {
        self.inks[role.index()]
    }
}

/// An sRGB channel as written in a pack, as linear light.
#[must_use]
pub fn linear(channel: u8) -> f32 {
    let c = f32::from(channel) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear light as an sRGB channel, the other way from [`linear`]; light past the screen's range
/// is the brightest it shows.
#[must_use]
pub fn srgb(light: f32) -> u8 {
    let l = light.clamp(0.0, 1.0);
    let c = if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = (c * 255.0).round() as u8;
    byte
}

/// What a piece of interface text or a control means, and so how it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Ordinary interface text and live controls.
    Interface,
    /// A heading.
    Title,
    /// Something that cannot be done now; drawn with its reason.
    Dim,
    /// A warning short of danger: a member under half their hit points.
    Caution,
    /// A refusal, a failure, a danger to the party.
    Alert,
}

impl Role {
    /// The role's colour, sRGB.
    #[must_use]
    pub const fn rgb(self) -> [f32; 3] {
        match self {
            Role::Interface | Role::Title => [0.55, 1.0, 0.65],
            Role::Dim => [0.25, 0.4, 0.3],
            Role::Caution => [1.0, 0.85, 0.3],
            Role::Alert => [1.0, 0.45, 0.35],
        }
    }
}

/// The interface's colours, sRGB: each role's and a control's three backgrounds. The default is
/// the green the interface has always had; a palette whose interface ink is another colour
/// gives a theme in that ink (owner, 2026-11-13).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// Ordinary text, live controls and headings.
    pub interface: [f32; 3],
    /// What cannot be done now.
    pub dim: [f32; 3],
    /// A control's background at rest, hovered and pressed.
    pub shades: [[f32; 3]; 3],
}

impl Default for Theme {
    fn default() -> Theme {
        Theme {
            interface: Role::Interface.rgb(),
            dim: Role::Dim.rgb(),
            shades: SHADES,
        }
    }
}

/// Dim text's share of the interface ink's light.
pub const DIM_LIGHT: f32 = 0.15;
/// The three backgrounds' shares of the interface ink's light: at rest, hovered, pressed.
pub const SHADE_LIGHT: [f32; 3] = [0.007, 0.05, 0.21];

impl Theme {
    /// The theme of `palette`. Where its interface ink is the default's, the default theme,
    /// exactly. Otherwise ordinary text is the ink as the screen shows it, its glow taken out
    /// so text never clips, and dim text and the backgrounds are that ink darkened. Caution
    /// and alert are warnings, not the interface's own colour, and stay as they are.
    #[must_use]
    pub fn of(palette: &Palette) -> Theme {
        let ink = palette.ink(ColourRole::Interface);
        if ink == Palette::default().ink(ColourRole::Interface) {
            return Theme::default();
        }
        let peak = ink[0].max(ink[1]).max(ink[2]).max(1.0);
        let light = ink.map(|c| c / peak);
        let at = |k: f32| light.map(|c| f32::from(srgb(c * k)) / 255.0);
        Theme {
            interface: at(1.0),
            dim: at(DIM_LIGHT),
            shades: SHADE_LIGHT.map(at),
        }
    }

    /// `role`'s colour.
    #[must_use]
    pub fn rgb(&self, role: Role) -> [f32; 3] {
        match role {
            Role::Interface | Role::Title => self.interface,
            Role::Dim => self.dim,
            Role::Caution | Role::Alert => role.rgb(),
        }
    }

    /// A control's background: at rest, hovered or pressed.
    #[must_use]
    pub fn shade(&self, i: usize) -> [f32; 3] {
        self.shades[i.min(2)]
    }
}

/// A line's pattern, the channel that says what kind of thing it is without colour
/// (presentation-PRD X19, 5.3). Open for more: sawtooth waits for a quad wider than its line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pattern {
    /// An unbroken line.
    Solid,
    /// Dashes, measured along the line in the world.
    Dashed,
    /// Short dashes, closer together.
    Dotted,
    /// Two rails with a gap between them.
    Double,
}

impl Pattern {
    /// What the line shader reads in a vertex's style.
    #[must_use]
    pub const fn code(self) -> f32 {
        match self {
            Pattern::Solid => 0.0,
            Pattern::Dashed => 1.0,
            Pattern::Dotted => 2.0,
            Pattern::Double => 3.0,
        }
    }
}

/// A line's weight, the channel that says how much it matters now (presentation-PRD 5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weight {
    /// Scenery: half the view's line width.
    Hairline,
    /// The view's line width.
    Normal,
    /// What matters now: half as wide again.
    Heavy,
}

impl Weight {
    /// The factor on the view's line width.
    #[must_use]
    pub const fn factor(self) -> f32 {
        match self {
            Weight::Hairline => 0.5,
            Weight::Normal => 1.0,
            Weight::Heavy => 1.5,
        }
    }
}

/// A control's background at rest, hovered and pressed, sRGB; opaque, so the lines behind never
/// cross a label (PRD §7).
pub const SHADES: [[f32; 3]; 3] = [[0.0, 0.08, 0.03], [0.05, 0.25, 0.1], [0.15, 0.5, 0.2]];

/// A size on the type scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Size {
    /// Headings.
    Title,
    /// Text and controls.
    Body,
    /// Notes beside a control, the HUD in a fight.
    Small,
}

/// The window height the scale was drawn for: at it, the sizes are the proof's 22, 16 and 14
/// pixels.
pub const REFERENCE_HEIGHT: f32 = 900.0;

impl Size {
    /// The size in logical pixels for a window `height` logical pixels tall, so a 5120×1440
    /// panel and a 16:9 one read alike (presentation-PRD §7).
    #[must_use]
    pub fn px(self, height: f32) -> f32 {
        let at_reference = match self {
            Size::Title => 22.0,
            Size::Body => 16.0,
            Size::Small => 14.0,
        };
        (at_reference * height / REFERENCE_HEIGHT).max(at_reference * 0.75)
    }
}

/// A length given at the reference height, scaled to `height`, as the type is.
#[must_use]
pub fn scaled(px: f32, height: f32) -> f32 {
    px * height / REFERENCE_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_type_scale_follows_the_window_height() {
        assert!((Size::Body.px(900.0) - 16.0).abs() < 1e-4);
        assert!((Size::Body.px(1440.0) - 25.6).abs() < 1e-4);
        // Never below three quarters of the reference, so a small window still reads.
        assert!((Size::Title.px(300.0) - 16.5).abs() < 1e-4);
    }

    #[test]
    fn an_srgb_channel_comes_back_from_linear_light() {
        for c in [0, 1, 8, 40, 128, 200, 254, 255] {
            assert_eq!(srgb(linear(c)), c);
        }
        assert_eq!(srgb(3.0), 255, "past the range is the brightest");
        assert_eq!(srgb(-1.0), 0);
    }

    #[test]
    fn the_default_theme_is_the_interface_s_green_as_it_was() {
        let theme = Theme::default();
        assert_eq!(Theme::of(&Palette::default()), theme);
        assert_eq!(theme.rgb(Role::Interface), [0.55, 1.0, 0.65]);
        assert_eq!(theme.rgb(Role::Title), [0.55, 1.0, 0.65]);
        assert_eq!(theme.rgb(Role::Dim), [0.25, 0.4, 0.3]);
        assert_eq!(theme.rgb(Role::Caution), [1.0, 0.85, 0.3]);
        assert_eq!(theme.rgb(Role::Alert), [1.0, 0.45, 0.35]);
        assert_eq!(
            [
                theme.shade(0),
                theme.shade(1),
                theme.shade(2),
                theme.shade(9)
            ],
            [SHADES[0], SHADES[1], SHADES[2], SHADES[2]]
        );
    }

    #[test]
    fn a_palette_s_interface_ink_gives_a_theme_in_that_ink() {
        let mut palette = Palette::default();
        // Twice the screen's range: the glow is taken out, the hue kept.
        palette.inks[ColourRole::Interface.index()] = [2.0, 1.0, 0.2];
        let theme = Theme::of(&palette);
        let byte = |light: f32| f32::from(srgb(light)) / 255.0;
        assert_eq!(theme.rgb(Role::Interface), [1.0, byte(0.5), byte(0.1)]);
        assert_eq!(theme.rgb(Role::Title), theme.rgb(Role::Interface));
        assert_eq!(theme.rgb(Role::Dim), [byte(0.15), byte(0.075), byte(0.015)]);
        let sum = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(sum(theme.rgb(Role::Dim)) < sum(theme.rgb(Role::Interface)));
        assert!(sum(theme.shade(0)) < sum(theme.shade(1)));
        assert!(sum(theme.shade(1)) < sum(theme.shade(2)));
        // Warnings are not the interface's own colour.
        assert_eq!(theme.rgb(Role::Caution), Role::Caution.rgb());
        assert_eq!(theme.rgb(Role::Alert), Role::Alert.rgb());
        assert_ne!(theme, Theme::default());
    }

    #[test]
    fn dim_and_alert_differ_from_live_text() {
        assert_ne!(Role::Dim.rgb(), Role::Interface.rgb());
        assert_ne!(Role::Alert.rgb(), Role::Interface.rgb());
    }
}
