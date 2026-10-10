//! The style system's seam (presentation-ARCHITECTURE §7): the interface's roles and type scale.
//! Every screen asks a role and a size here and never names a colour or a pixel size itself.
//! The style step adds the world's roles (structure, hostile, magic, ...), biosphere palettes
//! from pack data and the non-colour channels; the 3D view, the arena, the minimap and the
//! picture window keep their own colours until then. Bevy-free: colours are sRGB triples the
//! shell turns into `Color`.

/// What a piece of interface text or a control means, and so how it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Ordinary interface text and live controls.
    Interface,
    /// A heading.
    Title,
    /// Something that cannot be done now; drawn with its reason.
    Dim,
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
            Role::Alert => [1.0, 0.45, 0.35],
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
    fn dim_and_alert_differ_from_live_text() {
        assert_ne!(Role::Dim.rgb(), Role::Interface.rgb());
        assert_ne!(Role::Alert.rgb(), Role::Interface.rgb());
    }
}
