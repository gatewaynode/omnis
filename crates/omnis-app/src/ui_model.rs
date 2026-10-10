//! What every `bevy_ui` screen shares, Bevy-free: what a control reports (`Payload`), the
//! typefaces on offer, and the interface scale's numbers (the scale that follows the window,
//! and the largest one a window holds). A screen's own model (`creation_panel.rs`) turns
//! reports into edits with an `apply` function; `ui_kit.rs` is the Bevy side.

/// What a control reported.
#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    /// A button or a menu item was activated.
    Activate,
    /// A text input's content.
    Text(String),
    /// A number input's value, at every edit.
    Number(i64),
    /// A number input's value when Enter or leaving the field commits it; reported after its
    /// `Number`, for the panels that act once (the debug panel).
    Commit(i64),
    /// A slider's value.
    Slide(f32),
    /// A checkbox's new state.
    Flag(bool),
}

/// The typefaces on offer, by the index `PanelAction::Font` carries; the first is the
/// one Feathers embeds.
pub const FONTS: [&str; 3] = ["Fira Sans", "Inter", "Alegreya Sans"];

/// The smallest interface scale, in hundredths.
pub const SCALE_MIN: u16 = 50;
/// The largest interface scale, in hundredths.
pub const SCALE_MAX: u16 = 300;
/// The interface scale per physical pixel of a canvas pixel, in hundredths, until the slider
/// says otherwise: the owner's choice of 1.5 where the canvas is doubled (5120×1440).
pub const SCALE_PER_CANVAS_PIXEL: u16 = 75;
/// The smallest scale that follows the window: the owner's choice of 1.1 where the canvas
/// fits once (the 2560×1440 window class), because 0.75 is too small to read there.
pub const SCALE_FLOOR: u16 = 110;

/// The interface scale that follows the window, in hundredths: 1.1, 1.5, 2.25, 3.0 for a
/// canvas at one to four times its size.
#[must_use]
pub fn fitted_scale(canvas_scale: u32) -> u16 {
    let wanted = u32::from(SCALE_PER_CANVAS_PIXEL).saturating_mul(canvas_scale);
    u16::try_from(wanted.clamp(u32::from(SCALE_FLOOR), u32::from(SCALE_MAX))).unwrap_or(SCALE_MAX)
}

/// The largest interface scale the panel fits at, per physical pixel of a canvas pixel, in
/// hundredths. Measured 2026-09-20 with `layout_faults` over every class, all three fonts and
/// a party of six: the footer leaves the panel at 1.26 where the canvas fits once and at 2.52
/// where it is doubled.
pub const SCALE_FITS_PER_CANVAS_PIXEL: u16 = 125;

/// The largest interface scale this window holds, in hundredths: 1.25, 2.5, then `SCALE_MAX`.
/// The slider ends here, and a larger choice made in a larger window is held to it.
#[must_use]
pub fn scale_cap(canvas_scale: u32) -> u16 {
    let fits = u32::from(SCALE_FITS_PER_CANVAS_PIXEL).saturating_mul(canvas_scale);
    u16::try_from(fits.clamp(u32::from(SCALE_FLOOR), u32::from(SCALE_MAX))).unwrap_or(SCALE_MAX)
}

/// A slider's value as the whole number it stands for; not-a-number is refused.
#[must_use]
pub fn whole(value: f32) -> Option<i64> {
    // Saturating by definition of `as`; the callers clamp to a few dozen.
    #[allow(clippy::cast_possible_truncation)]
    value.is_finite().then(|| value.round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_follows_the_canvas_and_stops_at_what_the_window_holds() {
        // The owner's 1.5 on the doubled canvas of the ultrawide, in proportion elsewhere.
        assert_eq!([1, 2, 3, 4, 9].map(fitted_scale), [110, 150, 225, 300, 300]);
        // The cap is never under the scale that follows the window, and never over the slider.
        assert_eq!(
            [0, 1, 2, 3, 4, 9].map(scale_cap),
            [110, 125, 250, 300, 300, 300]
        );
        assert!((0..9).all(|canvas| fitted_scale(canvas) <= scale_cap(canvas)));
        assert_eq!(whole(14.6), Some(15));
        assert_eq!(whole(f32::NAN), None);
    }
}
