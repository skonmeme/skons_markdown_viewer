//! Process-wide settings: loading, persistence and propagation of changes to
//! every window (STANDARD §9.1).

use std::cell::RefCell;

use objc2::MainThreadMarker;
use objc2_foundation::NSUserDefaults;
use smv_core::settings::Settings;
use smv_core::style;

use crate::util::ns;

/// `NSUserDefaults` key holding the settings as JSON.
const DEFAULTS_KEY: &str = "Settings";

struct Store {
    settings: Settings,
}

thread_local! {
    static STORE: RefCell<Option<Store>> = const { RefCell::new(None) };
}

fn with_store<R>(body: impl FnOnce(&mut Store) -> R) -> R {
    STORE.with(|cell| body(cell.borrow_mut().get_or_insert_with(load)))
}

fn load() -> Store {
    let json = NSUserDefaults::standardUserDefaults().stringForKey(&ns(DEFAULTS_KEY));
    Store {
        settings: json
            .map(|json| Settings::from_json(&json.to_string()))
            .unwrap_or_default(),
    }
}

fn save(settings: &Settings) {
    let defaults = NSUserDefaults::standardUserDefaults();
    unsafe { defaults.setObject_forKey(Some(&ns(&settings.to_json())), &ns(DEFAULTS_KEY)) };
}

pub fn current() -> Settings {
    with_store(|store| store.settings.clone())
}

/// Applies `change`, stores the result and refreshes every window.
pub fn update(mtm: MainThreadMarker, change: impl FnOnce(&mut Settings)) {
    let changed = with_store(|store| {
        let before = store.settings.clone();
        let mut next = before.clone();
        change(&mut next);
        store.settings = next.normalized();
        if store.settings == before {
            return false;
        }
        save(&store.settings);
        true
    });
    if changed {
        for viewer in crate::viewer::ViewerController::all(mtm) {
            viewer.apply_settings();
        }
        crate::settings_window::SettingsController::refresh_if_open(mtm);
    }
}

/// The user stylesheet for the current settings.
pub fn user_stylesheet(mtm: MainThreadMarker) -> String {
    let settings = current();
    let faces = settings
        .active_font_set()
        .korean
        .map(|family| crate::fonts::faces(mtm, &family))
        .unwrap_or_default();
    style::user_stylesheet(&settings, &faces)
}
