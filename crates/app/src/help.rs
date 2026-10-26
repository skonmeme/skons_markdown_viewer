//! The usage guide opened from the Help menu (STANDARD §9.2).

use std::cell::RefCell;

use objc2::rc::Weak;
use objc2::{MainThreadMarker, msg_send};
use objc2_app_kit::{NSBeep, NSDocumentController};
use objc2_foundation::{NSBundle, NSLocale};

use crate::document::Document;
use crate::menu::APP_NAME;
use crate::util::{self, ns};

/// Languages of the bundled guide (`Resources/Help/<language>.md`).
const GUIDE_LANGUAGES: [&str; 2] = ["en", "ko"];
/// Guide shown for any other system language.
const FALLBACK_LANGUAGE: &str = "en";
const MARKDOWN_TYPE: &str = "net.daringfireball.markdown";

thread_local! {
    static GUIDE: RefCell<Option<Weak<Document>>> = const { RefCell::new(None) };
}

/// Shows the usage guide, reusing its window when it is already open.
pub fn show(mtm: MainThreadMarker) {
    let open = GUIDE.with(|guide| guide.borrow().as_ref().and_then(Weak::load));
    if let Some(document) = open {
        document.showWindows();
        return;
    }
    let Some(path) = guide_path() else {
        NSBeep();
        return;
    };
    let controller = NSDocumentController::sharedDocumentController(mtm);
    // Opened directly instead of through openDocument… so that the guide is
    // not added to Open Recent.
    let document = controller
        .makeDocumentWithContentsOfURL_ofType_error(&util::file_url(&path), &ns(MARKDOWN_TYPE));
    let Ok(document) = document else {
        NSBeep();
        return;
    };
    let title = ns(&format!("{APP_NAME} Help"));
    // setDisplayName: is not part of the generated bindings.
    let _: () = unsafe { msg_send![&*document, setDisplayName: &*title] };
    controller.addDocument(&document);
    document.makeWindowControllers();
    document.showWindows();
    if let Ok(document) = document.downcast::<Document>() {
        GUIDE.with(|guide| guide.replace(Some(Weak::from(&*document))));
    }
}

fn guide_path() -> Option<String> {
    let preferred = NSLocale::preferredLanguages();
    let system_language = preferred.firstObject().map(|l| l.to_string());
    let language = guide_language(system_language.as_deref());
    let path = NSBundle::mainBundle().pathForResource_ofType_inDirectory(
        Some(&ns(language)),
        Some(&ns("md")),
        Some(&ns("Help")),
    )?;
    Some(path.to_string())
}

/// The guide language for a system language tag such as `ko-KR`.
fn guide_language(system_language: Option<&str>) -> &'static str {
    let code = system_language
        .and_then(|tag| tag.split(['-', '_']).next())
        .unwrap_or_default()
        .to_ascii_lowercase();
    GUIDE_LANGUAGES
        .into_iter()
        .find(|language| *language == code)
        .unwrap_or(FALLBACK_LANGUAGE)
}

#[cfg(test)]
mod tests {
    use super::guide_language;

    #[test]
    fn follows_the_system_language() {
        assert_eq!(guide_language(Some("ko-KR")), "ko");
        assert_eq!(guide_language(Some("ko")), "ko");
        assert_eq!(guide_language(Some("en-US")), "en");
        assert_eq!(guide_language(Some("en_GB")), "en");
    }

    #[test]
    fn other_languages_get_english() {
        assert_eq!(guide_language(Some("ja-JP")), "en");
        assert_eq!(guide_language(Some("zh-Hans-CN")), "en");
        assert_eq!(guide_language(None), "en");
    }
}
