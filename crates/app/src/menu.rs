//! The main menu (STANDARD §9.2).

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject, Sel};
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSApplication, NSDocumentController, NSEventModifierFlags, NSImage, NSMenu, NSMenuItem,
    NSWorkspace,
};
use objc2_foundation::{NSSize, NSURL};

use crate::app_delegate::AppDelegate;
use crate::util::ns;

pub const APP_NAME: &str = "skon's Markdown Viewer";

const CMD: NSEventModifierFlags = NSEventModifierFlags::Command;

fn cmd_with(extra: NSEventModifierFlags) -> NSEventModifierFlags {
    CMD | extra
}

fn item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<Sel>,
    key: &str,
    modifiers: NSEventModifierFlags,
) -> Retained<NSMenuItem> {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &ns(title),
            action,
            &ns(key),
        )
    };
    if !key.is_empty() {
        item.setKeyEquivalentModifierMask(modifiers);
    }
    item
}

/// A hidden item that adds a second shortcut for an action, e.g. ⌘= for
/// Zoom In next to the visible ⌘+.
fn alternate_key(
    mtm: MainThreadMarker,
    action: Sel,
    key: &str,
    modifiers: NSEventModifierFlags,
) -> Retained<NSMenuItem> {
    let item = item(mtm, "", Some(action), key, modifiers);
    item.setHidden(true);
    item.setAllowsKeyEquivalentWhenHidden(true);
    item
}

fn submenu(
    mtm: MainThreadMarker,
    title: &str,
    items: &[Retained<NSMenuItem>],
) -> Retained<NSMenuItem> {
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &ns(title));
    for item in items {
        menu.addItem(item);
    }
    let holder = item(mtm, title, None, "", NSEventModifierFlags::empty());
    holder.setSubmenu(Some(&menu));
    holder
}

fn separator(mtm: MainThreadMarker) -> Retained<NSMenuItem> {
    NSMenuItem::separatorItem(mtm)
}

pub fn build(mtm: MainThreadMarker, delegate: &AppDelegate) -> Retained<NSMenu> {
    let none = NSEventModifierFlags::empty();
    let opt = NSEventModifierFlags::Option;
    let shift = NSEventModifierFlags::Shift;
    let app = NSApplication::sharedApplication(mtm);
    let delegate_object: &AnyObject = delegate.as_ref();

    let settings = item(mtm, "Settings…", Some(sel!(smvShowSettings:)), ",", CMD);
    unsafe { settings.setTarget(Some(delegate_object)) };
    let services = submenu(mtm, "Services", &[]);
    app.setServicesMenu(services.submenu().as_deref());
    let app_menu = submenu(
        mtm,
        APP_NAME,
        &[
            item(
                mtm,
                &format!("About {APP_NAME}"),
                Some(sel!(orderFrontStandardAboutPanel:)),
                "",
                none,
            ),
            separator(mtm),
            settings,
            separator(mtm),
            services,
            separator(mtm),
            item(
                mtm,
                &format!("Hide {APP_NAME}"),
                Some(sel!(hide:)),
                "h",
                CMD,
            ),
            item(
                mtm,
                "Hide Others",
                Some(sel!(hideOtherApplications:)),
                "h",
                cmd_with(opt),
            ),
            item(
                mtm,
                "Show All",
                Some(sel!(unhideAllApplications:)),
                "",
                none,
            ),
            separator(mtm),
            item(
                mtm,
                &format!("Quit {APP_NAME}"),
                Some(sel!(terminate:)),
                "q",
                CMD,
            ),
        ],
    );

    let open_recent = submenu(mtm, "Open Recent", &[]);
    if let Some(menu) = open_recent.submenu() {
        menu.setDelegate(Some(ProtocolObject::from_ref(delegate)));
    }
    let file_menu = submenu(
        mtm,
        "File",
        &[
            item(mtm, "Open…", Some(sel!(openDocument:)), "o", CMD),
            open_recent,
            separator(mtm),
            item(mtm, "Close", Some(sel!(performClose:)), "w", CMD),
            separator(mtm),
            item(mtm, "Reload", Some(sel!(smvReload:)), "r", CMD),
            separator(mtm),
            item(
                mtm,
                "Page Setup…",
                Some(sel!(runPageLayout:)),
                "p",
                cmd_with(shift),
            ),
            item(mtm, "Print…", Some(sel!(printDocument:)), "p", CMD),
        ],
    );

    let find_menu = submenu(
        mtm,
        "Find",
        &[
            item(mtm, "Find…", Some(sel!(smvShowFind:)), "f", CMD),
            item(mtm, "Find Next", Some(sel!(smvFindNext:)), "g", CMD),
            item(
                mtm,
                "Find Previous",
                Some(sel!(smvFindPrevious:)),
                "g",
                cmd_with(shift),
            ),
            item(
                mtm,
                "Use Selection for Find",
                Some(sel!(smvUseSelectionForFind:)),
                "e",
                CMD,
            ),
        ],
    );
    let edit_menu = submenu(
        mtm,
        "Edit",
        &[
            item(mtm, "Undo", Some(sel!(undo:)), "z", CMD),
            item(mtm, "Redo", Some(sel!(redo:)), "z", cmd_with(shift)),
            separator(mtm),
            item(mtm, "Cut", Some(sel!(cut:)), "x", CMD),
            item(mtm, "Copy", Some(sel!(copy:)), "c", CMD),
            item(mtm, "Paste", Some(sel!(paste:)), "v", CMD),
            item(mtm, "Select All", Some(sel!(selectAll:)), "a", CMD),
            separator(mtm),
            find_menu,
        ],
    );

    let view_menu = submenu(
        mtm,
        "View",
        &[
            item(mtm, "Actual Size", Some(sel!(smvActualSize:)), "0", CMD),
            item(mtm, "Zoom In", Some(sel!(smvZoomIn:)), "+", CMD),
            alternate_key(mtm, sel!(smvZoomIn:), "=", CMD),
            item(mtm, "Zoom Out", Some(sel!(smvZoomOut:)), "-", CMD),
            separator(mtm),
            item(
                mtm,
                "Increase Margins",
                Some(sel!(smvIncreaseMargins:)),
                "+",
                cmd_with(opt),
            ),
            alternate_key(mtm, sel!(smvIncreaseMargins:), "=", cmd_with(opt)),
            item(
                mtm,
                "Decrease Margins",
                Some(sel!(smvDecreaseMargins:)),
                "-",
                cmd_with(opt),
            ),
            item(
                mtm,
                "Automatic Margins",
                Some(sel!(smvAutomaticMargins:)),
                "0",
                cmd_with(opt),
            ),
            separator(mtm),
            item(
                mtm,
                "Enter Full Screen",
                Some(sel!(toggleFullScreen:)),
                "f",
                cmd_with(NSEventModifierFlags::Control),
            ),
        ],
    );

    let window_menu = submenu(
        mtm,
        "Window",
        &[
            item(mtm, "Minimize", Some(sel!(performMiniaturize:)), "m", CMD),
            item(mtm, "Zoom", Some(sel!(performZoom:)), "", none),
            separator(mtm),
            item(
                mtm,
                "Bring All to Front",
                Some(sel!(arrangeInFront:)),
                "",
                none,
            ),
        ],
    );
    app.setWindowsMenu(window_menu.submenu().as_deref());

    let help = item(
        mtm,
        &format!("{APP_NAME} Help"),
        Some(sel!(smvShowHelp:)),
        "",
        none,
    );
    unsafe { help.setTarget(Some(delegate_object)) };
    let help_menu = submenu(mtm, "Help", &[help]);
    app.setHelpMenu(help_menu.submenu().as_deref());

    let main = NSMenu::initWithTitle(NSMenu::alloc(mtm), &ns("Main Menu"));
    for menu in [
        app_menu,
        file_menu,
        edit_menu,
        view_menu,
        window_menu,
        help_menu,
    ] {
        main.addItem(&menu);
    }
    main
}

/// Rebuilds the Open Recent submenu from the document controller's list.
pub fn fill_open_recent(mtm: MainThreadMarker, menu: &NSMenu, delegate: &AppDelegate) {
    menu.removeAllItems();
    let urls: Vec<Retained<NSURL>> = NSDocumentController::sharedDocumentController(mtm)
        .recentDocumentURLs()
        .iter()
        .collect();
    let names: Vec<String> = urls
        .iter()
        .map(|url| {
            url.lastPathComponent()
                .map(|n| n.to_string())
                .unwrap_or_default()
        })
        .collect();
    let delegate_object: &AnyObject = delegate.as_ref();
    for (url, name) in urls.iter().zip(&names) {
        let duplicate = names.iter().filter(|n| *n == name).count() > 1;
        let title = match url
            .URLByDeletingLastPathComponent()
            .and_then(|p| p.lastPathComponent())
        {
            Some(folder) if duplicate => format!("{name} — {folder}"),
            _ => name.clone(),
        };
        let entry = item(
            mtm,
            &title,
            Some(sel!(smvOpenRecent:)),
            "",
            NSEventModifierFlags::empty(),
        );
        unsafe {
            entry.setTarget(Some(delegate_object));
            entry.setRepresentedObject(Some(url.as_ref()));
        }
        if let Some(path) = url.path() {
            let icon: Retained<NSImage> = NSWorkspace::sharedWorkspace().iconForFile(&path);
            icon.setSize(NSSize::new(16.0, 16.0));
            entry.setImage(Some(&icon));
        }
        menu.addItem(&entry);
    }
    if !urls.is_empty() {
        menu.addItem(&separator(mtm));
    }
    let clear = item(
        mtm,
        "Clear Menu",
        Some(sel!(clearRecentDocuments:)),
        "",
        NSEventModifierFlags::empty(),
    );
    menu.addItem(&clear);
}
