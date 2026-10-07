//! System tray icon and its four states.
//!
//! The artwork lives in `icons`; this module decides which badge wins and wires
//! the icon into the app.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Theme};

use crate::flyout;

mod icons;

pub use icons::icon_bytes;

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
        Badge::Review(n) => format!("{} awaiting your review", plural(n, "pull request", "pull requests")),
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

/// Builds the tray icon, its menu, and the click behaviour.
pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let idle = Image::from_bytes(icon_bytes(flyout::taskbar_theme(app.handle()), Badge::Idle))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(idle)
        .tooltip(tooltip(Badge::Idle))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => flyout::show(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            tauri_plugin_positioner::on_tray_event(app, &event);

            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                flyout::toggle(app);
            }
        })
        .build(app)?;

    Ok(())
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
