//! System tray icon and its four states.
//!
//! Windows has no badge API for tray icons, so each badge is baked into its own
//! bitmap. The PNGs are generated from the design source by
//! `scripts/make-tray-icons.mjs` and embedded here, which makes a missing asset
//! a compile error rather than an icon that silently never changes.

use tauri::image::Image;
use tauri::{AppHandle, Theme};

pub const TRAY_ID: &str = "main";

/// What the flyout knows about the active repository, as raw numbers. The
/// priority between them is decided here so it can be tested.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrayState {
    pub failing: u32,
    pub review: u32,
    pub running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    Failing(u32),
    Review(u32),
    Running,
    Idle,
}

/// One state wins. A failed run is more urgent than a review request, and both
/// are more urgent than "something is in flight".
pub fn resolve(state: TrayState) -> Badge {
    if state.failing > 0 {
        return Badge::Failing(state.failing);
    }
    if state.review > 0 {
        return Badge::Review(state.review);
    }
    if state.running {
        return Badge::Running;
    }
    Badge::Idle
}

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
            include_bytes!(concat!("../icons/tray/", $theme, "/", $kind, "-1.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $theme, "/", $kind, "-2.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $theme, "/", $kind, "-3.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $theme, "/", $kind, "-4.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $theme, "/", $kind, "-5.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $theme, "/", $kind, "-6.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $theme, "/", $kind, "-7.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $theme, "/", $kind, "-8.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $theme, "/", $kind, "-9.png")).as_slice(),
            include_bytes!(concat!("../icons/tray/", $theme, "/", $kind, "-9plus.png")).as_slice(),
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
    idle: include_bytes!("../icons/tray/on-light/idle.png"),
    running: include_bytes!("../icons/tray/on-light/running.png"),
    failing: counts!("on-light", "failing"),
    review: counts!("on-light", "review"),
};

const ON_DARK: IconSet = IconSet {
    idle: include_bytes!("../icons/tray/on-dark/idle.png"),
    running: include_bytes!("../icons/tray/on-dark/running.png"),
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

fn plural(n: u32, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

pub fn tooltip(badge: Badge) -> String {
    match badge {
        Badge::Failing(n) => format!("{} failed", plural(n, "workflow run", "workflow runs")),
        Badge::Review(n) => format!(
            "{} awaiting your review",
            plural(n, "pull request", "pull requests")
        ),
        Badge::Running => "A workflow is running".into(),
        Badge::Idle => "GitHub Deploy Management".into(),
    }
}

/// Repaints the tray icon. Silent on failure: a tray that cannot be updated is
/// not worth taking the app down for.
pub fn apply(app: &AppHandle, theme: Theme, state: TrayState) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let badge = resolve(state);
    if let Ok(image) = Image::from_bytes(icon_bytes(theme, badge)) {
        let _ = tray.set_icon(Some(image));
    }
    let _ = tray.set_tooltip(Some(tooltip(badge)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_most_urgent_state() {
        let state = |failing, review, running| TrayState { failing, review, running };

        // Red outranks orange even when both apply — and both outrank the dot.
        assert_eq!(resolve(state(2, 3, true)), Badge::Failing(2));
        assert_eq!(resolve(state(0, 3, true)), Badge::Review(3));
        assert_eq!(resolve(state(0, 0, true)), Badge::Running);
        assert_eq!(resolve(state(0, 0, false)), Badge::Idle);
    }

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

    #[test]
    fn every_state_has_an_icon_for_both_themes() {
        let mut badges = vec![Badge::Idle, Badge::Running];
        for n in 1..=12 {
            badges.push(Badge::Failing(n));
            badges.push(Badge::Review(n));
        }

        for theme in [Theme::Light, Theme::Dark] {
            for badge in &badges {
                let image = Image::from_bytes(icon_bytes(theme, *badge))
                    .unwrap_or_else(|e| panic!("{theme:?} {badge:?} is not a valid PNG: {e}"));
                assert_eq!(
                    (image.width(), image.height()),
                    (64, 64),
                    "{theme:?} {badge:?} is the wrong size"
                );
            }
        }
    }

    #[test]
    fn tooltips_read_as_sentences() {
        assert_eq!(tooltip(Badge::Failing(1)), "1 workflow run failed");
        assert_eq!(tooltip(Badge::Failing(2)), "2 workflow runs failed");
        assert_eq!(tooltip(Badge::Review(1)), "1 pull request awaiting your review");
        assert_eq!(tooltip(Badge::Idle), "GitHub Deploy Management");
    }
}
