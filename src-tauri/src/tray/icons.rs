//! The baked tray bitmaps and the lookup from badge to bytes.
//!
//! Windows has no badge API for tray icons, so each badge is baked into its own
//! bitmap. The PNGs are generated from the design source by
//! `scripts/make-tray-icons.mjs` and embedded here, which makes a missing asset
//! a compile error rather than an icon that silently never changes.

use tauri::Theme;

use super::Badge;

/// Index into the count arrays: 1..=9 map to themselves, anything above lands on
/// the shared `9plus` artwork — more digits than that cannot be read at 16px.
fn slot(count: u32) -> usize {
    count.clamp(1, 10) as usize - 1
}

// Folder names describe the TASKBAR, not the glyph: `on-light` holds the dark
// glyph meant for a light taskbar.
macro_rules! counts {
    ($theme:literal, $kind:literal) => {
        [
            include_bytes!(concat!("../../icons/tray/", $theme, "/", $kind, "-1.png")).as_slice(),
            include_bytes!(concat!("../../icons/tray/", $theme, "/", $kind, "-2.png")).as_slice(),
            include_bytes!(concat!("../../icons/tray/", $theme, "/", $kind, "-3.png")).as_slice(),
            include_bytes!(concat!("../../icons/tray/", $theme, "/", $kind, "-4.png")).as_slice(),
            include_bytes!(concat!("../../icons/tray/", $theme, "/", $kind, "-5.png")).as_slice(),
            include_bytes!(concat!("../../icons/tray/", $theme, "/", $kind, "-6.png")).as_slice(),
            include_bytes!(concat!("../../icons/tray/", $theme, "/", $kind, "-7.png")).as_slice(),
            include_bytes!(concat!("../../icons/tray/", $theme, "/", $kind, "-8.png")).as_slice(),
            include_bytes!(concat!("../../icons/tray/", $theme, "/", $kind, "-9.png")).as_slice(),
            include_bytes!(concat!("../../icons/tray/", $theme, "/", $kind, "-9plus.png")).as_slice(),
        ]
    };
}

struct IconSet {
    idle: &'static [u8],
    running: &'static [u8],
    failing: [&'static [u8]; 10],
    review: [&'static [u8]; 10],
}

const ON_LIGHT: IconSet = IconSet {
    idle: include_bytes!("../../icons/tray/on-light/idle.png"),
    running: include_bytes!("../../icons/tray/on-light/running.png"),
    failing: counts!("on-light", "failing"),
    review: counts!("on-light", "review"),
};

const ON_DARK: IconSet = IconSet {
    idle: include_bytes!("../../icons/tray/on-dark/idle.png"),
    running: include_bytes!("../../icons/tray/on-dark/running.png"),
    failing: counts!("on-dark", "failing"),
    review: counts!("on-dark", "review"),
};

pub fn icon_bytes(theme: Theme, badge: Badge) -> &'static [u8] {
    // Theme::Light means a light taskbar, which needs the dark glyph.
    let set = match theme {
        Theme::Light => &ON_LIGHT,
        _ => &ON_DARK,
    };
    match badge {
        Badge::Failing(n) => set.failing[slot(n)],
        Badge::Review(n) => set.review[slot(n)],
        Badge::Running => set.running,
        Badge::Idle => set.idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_the_badge_at_nine_plus() {
        assert_eq!(slot(1), 0);
        assert_eq!(slot(9), 8);
        assert_eq!(slot(10), 9, "ten is the first count that cannot be drawn");
        assert_eq!(slot(500), 9);
        // A zero count never becomes a badge, but must not index out of bounds
        // if it ever reached here.
        assert_eq!(slot(0), 0);
    }
}
