//! The Settings window (STANDARD §9.1): a sidebar with a search field and the
//! groups, next to one list of all settings.

use std::cell::{Cell, OnceCell, RefCell};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol, ProtocolObject, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSBackingStoreType, NSBorderType, NSBox,
    NSBoxType, NSButton, NSColor, NSColorWell, NSControl, NSControlStateValueOff,
    NSControlStateValueOn, NSControlTextEditingDelegate, NSFont, NSGridCell, NSGridCellPlacement,
    NSGridRow, NSGridView, NSImage, NSLayoutAttribute, NSLayoutConstraint, NSMenuItem,
    NSPopUpButton, NSResponder, NSScrollView, NSSearchField, NSSecureTextField, NSSlider,
    NSStackView, NSStepper, NSTableColumn, NSTableView, NSTableViewDataSource, NSTableViewDelegate,
    NSTableViewStyle, NSTextField, NSUserInterfaceLayoutOrientation, NSView,
    NSViewBoundsDidChangeNotification, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectView, NSWindow, NSWindowController, NSWindowStyleMask,
};
use objc2_foundation::{
    NSArray, NSEdgeInsets, NSIndexSet, NSInteger, NSNotification, NSNotificationCenter, NSObject,
    NSPoint, NSRect, NSSize,
};
use smv_core::settings::{
    AiModel, ColorMode, FIXED_MARGIN_MAX, FIXED_MARGIN_MIN, FONT_SIZE_MAX, FONT_SIZE_MIN,
    LINE_HEIGHT_MAX, LINE_HEIGHT_MIN, MAX_TOKENS_STEPS, MarginMode, ModelOption, Settings,
};
use smv_core::settings_search::{self, Group, Row};

use crate::util::{self, color, ns};
use crate::{afm, fonts, settings_store};

/// Tag of the "System Font" entry in the font family pop-up buttons.
const SYSTEM_FONT_TAG: isize = 1;

/// Tags telling the shared actions which control sent them.
const TAG_LATIN: isize = 1;
const TAG_KOREAN: isize = 2;
const TAG_MONOSPACE: isize = 3;
const TAG_TEXT_COLOR: isize = 1;
const TAG_BACKGROUND_COLOR: isize = 2;
const TAG_URL: isize = 1;
const TAG_MODEL: isize = 2;

/// Width of the label column of every group.
const LABEL_WIDTH: f64 = 140.0;
const SIDEBAR_WIDTH: f64 = 190.0;

/// Shown and confirmed when an external LLM is chosen (STANDARD §9.1).
const EXTERNAL_NOTICE: &str = "AI search will send the text of the document you search and \
your query to the server set below.";

const ON_DEVICE_TITLE: &str = "Apple Foundation Model (on this Mac)";
const EXTERNAL_TITLE: &str = "External LLM";

define_class!(
    // SAFETY: NSView may be subclassed; the class has no ivars.
    #[unsafe(super(NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SMVFlippedView"]
    struct FlippedView;

    impl FlippedView {
        /// Lays the list out from the top.
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }
    }
);

impl FlippedView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

struct Controls {
    font_set: Retained<NSPopUpButton>,
    remove: Retained<NSButton>,
    rename: Retained<NSButton>,
    latin: Retained<NSPopUpButton>,
    korean: Retained<NSPopUpButton>,
    monospace: Retained<NSPopUpButton>,
    size_field: Retained<NSTextField>,
    size_stepper: Retained<NSStepper>,
    line_height_field: Retained<NSTextField>,
    line_height_stepper: Retained<NSStepper>,
    color_automatic: Retained<NSButton>,
    color_custom: Retained<NSButton>,
    text_color: Retained<NSColorWell>,
    background_color: Retained<NSColorWell>,
    margin_automatic: Retained<NSButton>,
    margin_fixed: Retained<NSButton>,
    margin_slider: Retained<NSSlider>,
    margin_label: Retained<NSTextField>,
    ai_on_device: Retained<NSButton>,
    ai_external: Retained<NSButton>,
    ai_state: Retained<NSTextField>,
    url: Retained<NSTextField>,
    model: Retained<NSTextField>,
    options: Retained<NSStackView>,
    api_key_secure: Retained<NSSecureTextField>,
    api_key_plain: Retained<NSTextField>,
    api_key_toggle: Retained<NSButton>,
    max_tokens_slider: Retained<NSSlider>,
    max_tokens_field: Retained<NSTextField>,
}

/// One row of the list as seen by the search.
struct RowInfo {
    row: Retained<NSGridRow>,
    label: &'static str,
    options: &'static [&'static str],
    attached: bool,
    /// Hidden because of the settings, whatever the search shows.
    hidden_by_settings: Cell<bool>,
}

struct GroupInfo {
    title: &'static str,
    view: Retained<NSView>,
    rows: Vec<RowInfo>,
}

struct Ui {
    controls: Controls,
    groups: Vec<GroupInfo>,
    search: Retained<NSSearchField>,
    sidebar: Retained<NSTableView>,
    list: Retained<NSScrollView>,
    document: Retained<NSView>,
    no_results: Retained<NSTextField>,
}

/// Rows of the options of an external LLM.
struct OptionRow {
    key: Retained<NSTextField>,
    value: Retained<NSTextField>,
    remove: Retained<NSButton>,
}

#[derive(Default)]
pub struct SettingsIvars {
    ui: OnceCell<Ui>,
    /// Groups listed in the sidebar, as indices into the groups.
    shown_groups: RefCell<Vec<usize>>,
    /// Set while the sidebar follows the list, so that it does not scroll.
    syncing: Cell<bool>,
    api_key_visible: Cell<bool>,
    option_rows: RefCell<Vec<OptionRow>>,
    add_option: OnceCell<Retained<NSButton>>,
}

thread_local! {
    static CONTROLLER: RefCell<Option<Retained<SettingsController>>> = const { RefCell::new(None) };
}

/// Placement of a label: centered on its row, or at the top of a tall row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    Center,
    Top,
}

/// A row before it is placed in a grid.
struct RowSpec {
    label: &'static str,
    content: Retained<NSView>,
    options: &'static [&'static str],
    attached: bool,
    align: Align,
}

fn row(label: &'static str, content: Retained<NSView>) -> RowSpec {
    RowSpec {
        label,
        content,
        options: &[],
        attached: false,
        align: Align::Center,
    }
}

define_class!(
    // SAFETY: NSWindowController may be subclassed; the designated
    // initializer is called with the ivars set. No Drop impl.
    #[unsafe(super(NSWindowController, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SMVSettingsController"]
    #[ivars = SettingsIvars]
    pub struct SettingsController;

    unsafe impl NSObjectProtocol for SettingsController {}

    unsafe impl NSControlTextEditingDelegate for SettingsController {}

    unsafe impl NSTableViewDataSource for SettingsController {
        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, _table: &NSTableView) -> NSInteger {
            self.ivars().shown_groups.borrow().len() as NSInteger
        }
    }

    unsafe impl NSTableViewDelegate for SettingsController {
        #[unsafe(method_id(tableView:viewForTableColumn:row:))]
        fn view_for_row(
            &self,
            _table: &NSTableView,
            _column: Option<&NSTableColumn>,
            row: NSInteger,
        ) -> Option<Retained<NSView>> {
            self.sidebar_item(row)
        }

        #[unsafe(method(tableViewSelectionDidChange:))]
        fn selection_did_change(&self, _notification: &NSNotification) {
            if !self.ivars().syncing.get() {
                self.scroll_to_selected_group();
            }
        }
    }

    impl SettingsController {
        #[unsafe(method(smvSelectFontSet:))]
        fn select_font_set(&self, sender: &NSPopUpButton) {
            let Some(name) = sender.titleOfSelectedItem() else {
                return;
            };
            let name = name.to_string();
            settings_store::update(self.mtm(), |s| s.select_font_set(&name));
        }

        #[unsafe(method(smvAddFontSet:))]
        fn add_font_set(&self, _sender: Option<&AnyObject>) {
            settings_store::update(self.mtm(), |s| {
                s.duplicate_active_font_set();
            });
        }

        #[unsafe(method(smvRemoveFontSet:))]
        fn remove_font_set(&self, _sender: Option<&AnyObject>) {
            let name = settings_store::current().active_font_set;
            let confirmed = self.confirm(
                &format!("Delete the font set “{name}”?"),
                "This action cannot be undone.",
                "Delete",
            );
            if confirmed {
                settings_store::update(self.mtm(), |s| s.delete_font_set(&name));
            }
        }

        #[unsafe(method(smvRenameFontSet:))]
        fn rename_font_set(&self, _sender: Option<&AnyObject>) {
            self.rename_active_font_set();
        }

        #[unsafe(method(smvFontFamilyChanged:))]
        fn font_family_changed(&self, sender: &NSPopUpButton) {
            let family = sender.selectedItem().and_then(|item| {
                (item.tag() != SYSTEM_FONT_TAG).then(|| item.title().to_string())
            });
            let tag = sender.tag();
            settings_store::update(self.mtm(), |s| {
                s.edit_active_font_set(|set| match tag {
                    TAG_LATIN => set.latin = family,
                    TAG_KOREAN => set.korean = family,
                    _ => set.monospace = family,
                });
            });
        }

        #[unsafe(method(smvFontSizeChanged:))]
        fn font_size_changed(&self, sender: &NSControl) {
            let size = sender.doubleValue();
            settings_store::update(self.mtm(), |s| {
                s.edit_active_font_set(|set| set.size = size.round());
            });
            self.refresh();
        }

        #[unsafe(method(smvLineHeightChanged:))]
        fn line_height_changed(&self, sender: &NSControl) {
            let line_height = (sender.doubleValue() * 20.0).round() / 20.0;
            settings_store::update(self.mtm(), |s| {
                s.edit_active_font_set(|set| set.line_height = line_height);
            });
            self.refresh();
        }

        #[unsafe(method(smvColorModeChanged:))]
        fn color_mode_changed(&self, _sender: Option<&AnyObject>) {
            let Some(ui) = self.ui() else {
                return;
            };
            let custom = ui.controls.color_custom.state() == NSControlStateValueOn;
            settings_store::update(self.mtm(), |s| {
                s.color_mode = if custom { ColorMode::Custom } else { ColorMode::Automatic };
            });
        }

        #[unsafe(method(smvColorChanged:))]
        fn color_changed(&self, sender: &NSColorWell) {
            let Some(rgb) = util::rgb(&sender.color()) else {
                return;
            };
            let tag = sender.tag();
            settings_store::update(self.mtm(), |s| match tag {
                TAG_TEXT_COLOR => s.text_color = rgb,
                _ => s.background_color = rgb,
            });
        }

        #[unsafe(method(smvMarginModeChanged:))]
        fn margin_mode_changed(&self, _sender: Option<&AnyObject>) {
            let Some(ui) = self.ui() else {
                return;
            };
            let fixed = ui.controls.margin_fixed.state() == NSControlStateValueOn;
            settings_store::update(self.mtm(), |s| {
                s.margin_mode = if fixed { MarginMode::Fixed } else { MarginMode::Automatic };
            });
        }

        #[unsafe(method(smvMarginChanged:))]
        fn margin_changed(&self, sender: &NSSlider) {
            let margin = sender.doubleValue().round();
            settings_store::update(self.mtm(), |s| {
                s.fixed_margin = margin;
                s.margin_mode = MarginMode::Fixed;
            });
        }

        #[unsafe(method(smvSettingsSearchChanged:))]
        fn settings_search_changed(&self, _sender: Option<&AnyObject>) {
            self.apply_search();
        }

        #[unsafe(method(smvListScrolled:))]
        fn list_scrolled(&self, _notification: &NSNotification) {
            self.follow_list();
        }

        #[unsafe(method(smvAiModelChanged:))]
        fn ai_model_changed(&self, _sender: Option<&AnyObject>) {
            let Some(ui) = self.ui() else {
                return;
            };
            let external = ui.controls.ai_external.state() == NSControlStateValueOn;
            let current = settings_store::current().ai_model;
            if external && current != AiModel::External {
                // The document may leave the Mac: ask first (STANDARD §9.1).
                if !self.confirm(EXTERNAL_NOTICE, "", "Use") {
                    return self.refresh();
                }
            }
            settings_store::update(self.mtm(), |s| {
                s.ai_model = if external { AiModel::External } else { AiModel::OnDevice };
            });
        }

        #[unsafe(method(smvExternalTextChanged:))]
        fn external_text_changed(&self, sender: &NSTextField) {
            let text = sender.stringValue().to_string();
            let tag = sender.tag();
            settings_store::update(self.mtm(), |s| match tag {
                TAG_URL => s.external_llm.url = text,
                TAG_MODEL => s.external_llm.model = text,
                _ => {}
            });
        }

        #[unsafe(method(smvApiKeyChanged:))]
        fn api_key_changed(&self, sender: &NSTextField) {
            let key = sender.stringValue().to_string();
            settings_store::update(self.mtm(), |s| s.external_llm.api_key = key);
        }

        #[unsafe(method(smvToggleApiKey:))]
        fn toggle_api_key(&self, _sender: Option<&AnyObject>) {
            let visible = !self.ivars().api_key_visible.get();
            self.ivars().api_key_visible.set(visible);
            self.refresh();
        }

        #[unsafe(method(smvOptionChanged:))]
        fn option_changed(&self, sender: &NSTextField) {
            let tag = sender.tag().max(0) as usize;
            let (index, is_value) = (tag / 2, tag % 2 == 1);
            let text = sender.stringValue().to_string();
            settings_store::update(self.mtm(), |s| {
                if let Some(option) = s.external_llm.options.get_mut(index) {
                    if is_value {
                        option.value = text;
                    } else {
                        option.key = text;
                    }
                }
            });
        }

        #[unsafe(method(smvAddOption:))]
        fn add_option(&self, _sender: Option<&AnyObject>) {
            settings_store::update(self.mtm(), |s| {
                s.external_llm.options.push(ModelOption::default());
            });
        }

        #[unsafe(method(smvRemoveOption:))]
        fn remove_option(&self, sender: &NSButton) {
            let index = sender.tag().max(0) as usize;
            settings_store::update(self.mtm(), |s| {
                if index < s.external_llm.options.len() {
                    s.external_llm.options.remove(index);
                }
            });
        }

        #[unsafe(method(smvMaxTokensSlider:))]
        fn max_tokens_slider(&self, sender: &NSSlider) {
            let step = sender.integerValue().clamp(0, MAX_TOKENS_STEPS.len() as isize - 1);
            let tokens = MAX_TOKENS_STEPS[step as usize];
            settings_store::update(self.mtm(), |s| s.external_llm.max_tokens = tokens);
        }

        #[unsafe(method(smvMaxTokensField:))]
        fn max_tokens_field(&self, sender: &NSTextField) {
            let digits: String = sender
                .stringValue()
                .to_string()
                .chars()
                .filter(char::is_ascii_digit)
                .collect();
            match digits.parse::<u64>() {
                Ok(tokens) => settings_store::update(self.mtm(), |s| {
                    s.external_llm.max_tokens = tokens.min(u64::from(u32::MAX)) as u32;
                }),
                Err(_) => self.refresh(),
            }
        }
    }
);

impl SettingsController {
    /// Shows the Settings window, creating it on first use.
    pub fn show(mtm: MainThreadMarker) {
        let controller = CONTROLLER.with(|cell| {
            cell.borrow_mut()
                .get_or_insert_with(|| Self::create(mtm))
                .clone()
        });
        controller.refresh();
        unsafe { controller.showWindow(None) };
        if let Some(window) = controller.window() {
            window.makeKeyAndOrderFront(None);
            // Do not start with a text field focused.
            window.makeFirstResponder(None);
        }
        controller.follow_list();
    }

    /// Updates the controls after the settings changed elsewhere.
    pub fn refresh_if_open(_mtm: MainThreadMarker) {
        let controller = CONTROLLER.with(|cell| cell.borrow().clone());
        if let Some(controller) = controller {
            controller.refresh();
        }
    }

    fn create(mtm: MainThreadMarker) -> Retained<Self> {
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(780.0, 580.0)),
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Resizable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&ns("Settings"));
        window.setContentMinSize(NSSize::new(620.0, 360.0));

        let this = Self::alloc(mtm).set_ivars(SettingsIvars::default());
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithWindow: &*window] };
        let (content, ui) = this.build(mtm);
        window.setContentView(Some(&content));
        let autosave_name = ns("SMVSettings");
        if !window.setFrameUsingName(&autosave_name) {
            window.center();
        }
        window.setFrameAutosaveName(&autosave_name);

        let clip = ui.list.contentView();
        clip.setPostsBoundsChangedNotifications(true);
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                this.as_ref(),
                sel!(smvListScrolled:),
                Some(NSViewBoundsDidChangeNotification),
                Some(&clip),
            );
        }
        let _ = this.ivars().ui.set(ui);
        this.apply_search();
        this
    }

    fn ui(&self) -> Option<&Ui> {
        self.ivars().ui.get()
    }

    /// The title of a group shown in the sidebar.
    fn sidebar_item(&self, row: NSInteger) -> Option<Retained<NSView>> {
        let group = *self.ivars().shown_groups.borrow().get(row as usize)?;
        let title = self.ui()?.groups.get(group)?.title;
        Some(view(&NSTextField::labelWithString(&ns(title), self.mtm())))
    }

    fn target(&self) -> &AnyObject {
        self.as_ref()
    }

    fn build(&self, mtm: MainThreadMarker) -> (Retained<NSView>, Ui) {
        let target = self.target();
        let lists = fonts::lists(mtm);

        // Fonts (§7)
        let font_set = NSPopUpButton::new(mtm);
        unsafe {
            font_set.setTarget(Some(target));
            font_set.setAction(Some(sel!(smvSelectFontSet:)));
        }
        font_set
            .widthAnchor()
            .constraintGreaterThanOrEqualToConstant(140.0)
            .setActive(true);
        let add = symbol_button(
            mtm,
            "plus",
            "Duplicate font set",
            target,
            sel!(smvAddFontSet:),
        );
        let remove = symbol_button(
            mtm,
            "minus",
            "Delete font set",
            target,
            sel!(smvRemoveFontSet:),
        );
        let rename = unsafe {
            NSButton::buttonWithTitle_target_action(
                &ns("Rename…"),
                Some(target),
                Some(sel!(smvRenameFontSet:)),
                mtm,
            )
        };
        let latin = family_popup(mtm, &lists.all, TAG_LATIN, target);
        let korean = family_popup(mtm, &lists.korean, TAG_KOREAN, target);
        let monospace = family_popup(mtm, &lists.monospace, TAG_MONOSPACE, target);
        for popup in [&latin, &korean, &monospace] {
            popup
                .widthAnchor()
                .constraintEqualToConstant(260.0)
                .setActive(true);
        }
        let (size_field, size_stepper) = number_input(
            mtm,
            FONT_SIZE_MIN,
            FONT_SIZE_MAX,
            1.0,
            sel!(smvFontSizeChanged:),
            target,
        );
        let (line_height_field, line_height_stepper) = number_input(
            mtm,
            LINE_HEIGHT_MIN,
            LINE_HEIGHT_MAX,
            0.05,
            sel!(smvLineHeightChanged:),
            target,
        );
        let default_hint = note(
            mtm,
            "The Default font set cannot be changed. Click + to make an editable copy.",
        );
        let fonts = group(
            mtm,
            "Fonts",
            vec![
                row(
                    "Font set:",
                    hstack(
                        mtm,
                        &[view(&font_set), view(&add), view(&remove), view(&rename)],
                    ),
                ),
                row("Latin font:", view(&latin)),
                row("Korean font:", view(&korean)),
                row("Monospace font:", view(&monospace)),
                row(
                    "Font size:",
                    hstack(
                        mtm,
                        &[view(&size_field), view(&size_stepper), label(mtm, "pt")],
                    ),
                ),
                row(
                    "Line height:",
                    hstack(mtm, &[view(&line_height_field), view(&line_height_stepper)]),
                ),
                RowSpec {
                    attached: true,
                    ..row("", view(&default_hint))
                },
            ],
        );

        // Colors (§8)
        let color_automatic = radio(mtm, "Automatic", target, sel!(smvColorModeChanged:));
        let color_custom = radio(mtm, "Custom", target, sel!(smvColorModeChanged:));
        let text_color = color_well(mtm, TAG_TEXT_COLOR, target);
        let background_color = color_well(mtm, TAG_BACKGROUND_COLOR, target);
        let colors = group(
            mtm,
            "Colors",
            vec![
                RowSpec {
                    options: &["Automatic", "Custom"],
                    ..row(
                        "Colors:",
                        hstack(mtm, &[view(&color_automatic), view(&color_custom)]),
                    )
                },
                row("Text color:", view(&text_color)),
                row("Background color:", view(&background_color)),
            ],
        );

        // Margins (§5)
        let margin_automatic = radio(mtm, "Automatic", target, sel!(smvMarginModeChanged:));
        let margin_fixed = radio(mtm, "Fixed", target, sel!(smvMarginModeChanged:));
        let margin_slider = unsafe {
            NSSlider::sliderWithValue_minValue_maxValue_target_action(
                0.0,
                FIXED_MARGIN_MIN,
                FIXED_MARGIN_MAX,
                Some(target),
                Some(sel!(smvMarginChanged:)),
                mtm,
            )
        };
        margin_slider.setContinuous(true);
        margin_slider
            .widthAnchor()
            .constraintEqualToConstant(220.0)
            .setActive(true);
        let margin_label = NSTextField::labelWithString(&ns(""), mtm);
        margin_label
            .widthAnchor()
            .constraintGreaterThanOrEqualToConstant(56.0)
            .setActive(true);
        let margins = group(
            mtm,
            "Margins",
            vec![
                RowSpec {
                    options: &["Automatic", "Fixed"],
                    ..row(
                        "Margins:",
                        hstack(mtm, &[view(&margin_automatic), view(&margin_fixed)]),
                    )
                },
                row(
                    "Fixed margin:",
                    hstack(mtm, &[view(&margin_slider), view(&margin_label)]),
                ),
            ],
        );

        // AI (§6.6)
        let ai_on_device = radio(mtm, ON_DEVICE_TITLE, target, sel!(smvAiModelChanged:));
        let ai_external = radio(mtm, EXTERNAL_TITLE, target, sel!(smvAiModelChanged:));
        let ai_state = note(mtm, "");
        let url = text_field(
            mtm,
            "https://example.com/v1/chat/completions",
            380.0,
            target,
        );
        url.setTag(TAG_URL);
        unsafe { url.setAction(Some(sel!(smvExternalTextChanged:))) };
        let model = text_field(mtm, "Model name", 240.0, target);
        model.setTag(TAG_MODEL);
        unsafe { model.setAction(Some(sel!(smvExternalTextChanged:))) };
        let options = vstack(mtm, &[], 6.0);
        let api_key_secure = NSSecureTextField::new(mtm);
        configure_text_field(&api_key_secure, "None", 300.0, target);
        unsafe { api_key_secure.setAction(Some(sel!(smvApiKeyChanged:))) };
        let api_key_plain = text_field(mtm, "None", 300.0, target);
        unsafe { api_key_plain.setAction(Some(sel!(smvApiKeyChanged:))) };
        // Hidden until the toggle shows it, so that the first layout does not
        // widen the window to fit both key fields.
        api_key_plain.setHidden(true);
        let api_key_toggle = symbol_button(
            mtm,
            "eye",
            "Show the API key",
            target,
            sel!(smvToggleApiKey:),
        );
        let max_tokens_slider = unsafe {
            NSSlider::sliderWithValue_minValue_maxValue_target_action(
                0.0,
                0.0,
                (MAX_TOKENS_STEPS.len() - 1) as f64,
                Some(target),
                Some(sel!(smvMaxTokensSlider:)),
                mtm,
            )
        };
        max_tokens_slider.setNumberOfTickMarks(MAX_TOKENS_STEPS.len() as NSInteger);
        max_tokens_slider.setAllowsTickMarkValuesOnly(true);
        max_tokens_slider
            .widthAnchor()
            .constraintEqualToConstant(200.0)
            .setActive(true);
        let max_tokens_field = text_field(mtm, "", 90.0, target);
        unsafe { max_tokens_field.setAction(Some(sel!(smvMaxTokensField:))) };
        let ai = group(
            mtm,
            "AI",
            vec![
                RowSpec {
                    options: &[ON_DEVICE_TITLE, EXTERNAL_TITLE],
                    align: Align::Top,
                    ..row(
                        "Model:",
                        view(&vstack(
                            mtm,
                            &[view(&ai_on_device), view(&ai_external), view(&ai_state)],
                            6.0,
                        )),
                    )
                },
                row("URL:", view(&url)),
                row("Model name:", view(&model)),
                RowSpec {
                    align: Align::Top,
                    ..row("Model options:", view(&options))
                },
                row(
                    "API key:",
                    hstack(
                        mtm,
                        &[
                            view(&api_key_secure),
                            view(&api_key_plain),
                            view(&api_key_toggle),
                        ],
                    ),
                ),
                row(
                    "Max tokens:",
                    hstack(
                        mtm,
                        &[
                            view(&max_tokens_slider),
                            view(&max_tokens_field),
                            label(mtm, "tokens"),
                        ],
                    ),
                ),
            ],
        );

        let groups = vec![fonts, colors, margins, ai];
        let (content, search, sidebar, list, document, no_results) = self.layout(mtm, &groups);
        let controls = Controls {
            font_set,
            remove,
            rename,
            latin,
            korean,
            monospace,
            size_field,
            size_stepper,
            line_height_field,
            line_height_stepper,
            color_automatic,
            color_custom,
            text_color,
            background_color,
            margin_automatic,
            margin_fixed,
            margin_slider,
            margin_label,
            ai_on_device,
            ai_external,
            ai_state,
            url,
            model,
            options,
            api_key_secure,
            api_key_plain,
            api_key_toggle,
            max_tokens_slider,
            max_tokens_field,
        };
        let ui = Ui {
            controls,
            groups,
            search,
            sidebar,
            list,
            document,
            no_results,
        };
        (content, ui)
    }

    /// Places the sidebar and the list in the window.
    #[allow(clippy::type_complexity)]
    fn layout(
        &self,
        mtm: MainThreadMarker,
        groups: &[GroupInfo],
    ) -> (
        Retained<NSView>,
        Retained<NSSearchField>,
        Retained<NSTableView>,
        Retained<NSScrollView>,
        Retained<NSView>,
        Retained<NSTextField>,
    ) {
        let target = self.target();

        let search = NSSearchField::new(mtm);
        search.setPlaceholderString(Some(&ns("Search")));
        unsafe {
            search.setTarget(Some(target));
            search.setAction(Some(sel!(smvSettingsSearchChanged:)));
        }
        let sidebar = NSTableView::new(mtm);
        sidebar.setStyle(NSTableViewStyle::SourceList);
        sidebar.setHeaderView(None);
        let column = NSTableColumn::initWithIdentifier(NSTableColumn::alloc(mtm), &ns("group"));
        sidebar.addTableColumn(&column);
        unsafe {
            sidebar.setDataSource(Some(ProtocolObject::from_ref(self)));
            sidebar.setDelegate(Some(ProtocolObject::from_ref(self)));
        }
        let sidebar_scroll = NSScrollView::new(mtm);
        sidebar_scroll.setDocumentView(Some(&sidebar));
        sidebar_scroll.setDrawsBackground(false);
        sidebar_scroll.setBorderType(NSBorderType::NoBorder);
        sidebar_scroll.setHasVerticalScroller(true);
        sidebar_scroll.setAutohidesScrollers(true);

        let sidebar_view = NSVisualEffectView::new(mtm);
        sidebar_view.setMaterial(NSVisualEffectMaterial::Sidebar);
        sidebar_view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        let sidebar_view: Retained<NSView> = Retained::into_super(sidebar_view);
        for subview in [&*search as &NSView, &sidebar_scroll] {
            subview.setTranslatesAutoresizingMaskIntoConstraints(false);
            sidebar_view.addSubview(subview);
        }

        // The list: every group below the other in a flipped document view.
        let stack = vstack(
            mtm,
            &groups.iter().map(|g| g.view.clone()).collect::<Vec<_>>(),
            28.0,
        );
        let no_results = NSTextField::labelWithString(&ns("No Results"), mtm);
        no_results.setTextColor(Some(&NSColor::secondaryLabelColor()));
        no_results.setFont(Some(&NSFont::systemFontOfSize(15.0)));
        no_results.setHidden(true);
        let document: Retained<NSView> = Retained::into_super(FlippedView::new(mtm));
        for subview in [&*stack as &NSView, &no_results] {
            subview.setTranslatesAutoresizingMaskIntoConstraints(false);
            document.addSubview(subview);
        }
        let list = NSScrollView::new(mtm);
        list.setHasVerticalScroller(true);
        list.setAutohidesScrollers(true);
        list.setBorderType(NSBorderType::NoBorder);
        list.setDrawsBackground(false);
        document.setTranslatesAutoresizingMaskIntoConstraints(false);
        list.setDocumentView(Some(&document));
        let clip = list.contentView();

        let divider = NSBox::new(mtm);
        divider.setBoxType(NSBoxType::Separator);

        let content = NSView::new(mtm);
        for subview in [&*sidebar_view, &divider, &list] {
            subview.setTranslatesAutoresizingMaskIntoConstraints(false);
            content.addSubview(subview);
        }
        NSLayoutConstraint::activateConstraints(&NSArray::from_retained_slice(&[
            sidebar_view
                .leadingAnchor()
                .constraintEqualToAnchor(&content.leadingAnchor()),
            sidebar_view
                .topAnchor()
                .constraintEqualToAnchor(&content.topAnchor()),
            sidebar_view
                .bottomAnchor()
                .constraintEqualToAnchor(&content.bottomAnchor()),
            sidebar_view
                .widthAnchor()
                .constraintEqualToConstant(SIDEBAR_WIDTH),
            search
                .topAnchor()
                .constraintEqualToAnchor_constant(&sidebar_view.topAnchor(), 12.0),
            search
                .leadingAnchor()
                .constraintEqualToAnchor_constant(&sidebar_view.leadingAnchor(), 10.0),
            search
                .trailingAnchor()
                .constraintEqualToAnchor_constant(&sidebar_view.trailingAnchor(), -10.0),
            sidebar_scroll
                .topAnchor()
                .constraintEqualToAnchor_constant(&search.bottomAnchor(), 8.0),
            sidebar_scroll
                .leadingAnchor()
                .constraintEqualToAnchor(&sidebar_view.leadingAnchor()),
            sidebar_scroll
                .trailingAnchor()
                .constraintEqualToAnchor(&sidebar_view.trailingAnchor()),
            sidebar_scroll
                .bottomAnchor()
                .constraintEqualToAnchor(&sidebar_view.bottomAnchor()),
            divider
                .leadingAnchor()
                .constraintEqualToAnchor(&sidebar_view.trailingAnchor()),
            divider
                .topAnchor()
                .constraintEqualToAnchor(&content.topAnchor()),
            divider
                .bottomAnchor()
                .constraintEqualToAnchor(&content.bottomAnchor()),
            divider.widthAnchor().constraintEqualToConstant(1.0),
            list.leadingAnchor()
                .constraintEqualToAnchor(&divider.trailingAnchor()),
            list.trailingAnchor()
                .constraintEqualToAnchor(&content.trailingAnchor()),
            list.topAnchor()
                .constraintEqualToAnchor(&content.topAnchor()),
            list.bottomAnchor()
                .constraintEqualToAnchor(&content.bottomAnchor()),
            document
                .leadingAnchor()
                .constraintEqualToAnchor(&clip.leadingAnchor()),
            document
                .trailingAnchor()
                .constraintEqualToAnchor(&clip.trailingAnchor()),
            document
                .topAnchor()
                .constraintEqualToAnchor(&clip.topAnchor()),
            stack
                .topAnchor()
                .constraintEqualToAnchor_constant(&document.topAnchor(), 20.0),
            stack
                .leadingAnchor()
                .constraintEqualToAnchor_constant(&document.leadingAnchor(), 24.0),
            stack
                .trailingAnchor()
                .constraintLessThanOrEqualToAnchor_constant(&document.trailingAnchor(), -24.0),
            document
                .bottomAnchor()
                .constraintEqualToAnchor_constant(&stack.bottomAnchor(), 24.0),
            no_results
                .centerXAnchor()
                .constraintEqualToAnchor(&document.centerXAnchor()),
            no_results
                .topAnchor()
                .constraintEqualToAnchor_constant(&document.topAnchor(), 40.0),
        ]));
        (content, search, sidebar, list, document, no_results)
    }

    /// Shows the current settings in the controls.
    fn refresh(&self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let controls = &ui.controls;
        let settings: Settings = settings_store::current();
        let set = settings.active_font_set();
        let editable = !settings.is_default_font_set_active();

        controls.font_set.removeAllItems();
        for name in settings.font_set_names() {
            controls.font_set.addItemWithTitle(&ns(&name));
        }
        controls
            .font_set
            .selectItemWithTitle(&ns(&settings.active_font_set));
        controls.remove.setEnabled(editable);
        controls.rename.setEnabled(editable);

        for (popup, family) in [
            (&controls.latin, &set.latin),
            (&controls.korean, &set.korean),
            (&controls.monospace, &set.monospace),
        ] {
            select_family(popup, family.as_deref());
            popup.setEnabled(editable);
        }

        set_text(&controls.size_field, &format!("{}", set.size));
        controls.size_stepper.setDoubleValue(set.size);
        set_text(
            &controls.line_height_field,
            &format!("{:.2}", set.line_height),
        );
        controls.line_height_stepper.setDoubleValue(set.line_height);
        for control in [
            &*controls.size_field as &NSControl,
            &controls.size_stepper,
            &controls.line_height_field,
            &controls.line_height_stepper,
        ] {
            control.setEnabled(editable);
        }
        if let Some(hint) = ui.groups[0].rows.last() {
            hint.hidden_by_settings.set(editable);
        }

        let custom = settings.color_mode == ColorMode::Custom;
        set_radio(&controls.color_automatic, !custom);
        set_radio(&controls.color_custom, custom);
        controls.text_color.setColor(&color(settings.text_color));
        controls
            .background_color
            .setColor(&color(settings.background_color));
        controls.text_color.setEnabled(custom);
        controls.background_color.setEnabled(custom);

        let fixed = settings.margin_mode == MarginMode::Fixed;
        set_radio(&controls.margin_automatic, !fixed);
        set_radio(&controls.margin_fixed, fixed);
        controls.margin_slider.setDoubleValue(settings.fixed_margin);
        controls.margin_slider.setEnabled(fixed);
        controls
            .margin_label
            .setStringValue(&ns(&format!("{} pt", settings.fixed_margin)));

        self.refresh_ai(&settings);
        self.apply_search();
    }

    fn refresh_ai(&self, settings: &Settings) {
        let Some(ui) = self.ui() else {
            return;
        };
        let controls = &ui.controls;
        let external = settings.ai_model == AiModel::External;
        set_radio(&controls.ai_on_device, !external);
        set_radio(&controls.ai_external, external);
        let state = if external {
            EXTERNAL_NOTICE
        } else {
            match afm::availability() {
                Ok(()) => "Available on this Mac.",
                Err(unavailable) => unavailable.reason(),
            }
        };
        controls.ai_state.setStringValue(&ns(state));

        let llm = &settings.external_llm;
        set_text(&controls.url, &llm.url);
        set_text(&controls.model, &llm.model);
        set_text(&controls.api_key_secure, &llm.api_key);
        set_text(&controls.api_key_plain, &llm.api_key);
        let visible = self.ivars().api_key_visible.get();
        controls.api_key_secure.setHidden(visible);
        controls.api_key_plain.setHidden(!visible);
        let symbol = if visible { "eye.slash" } else { "eye" };
        if let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &ns(symbol),
            Some(&ns("Show the API key")),
        ) {
            controls.api_key_toggle.setImage(Some(&image));
        }
        controls
            .max_tokens_slider
            .setIntegerValue(nearest_step(llm.max_tokens) as NSInteger);
        set_text(&controls.max_tokens_field, &llm.max_tokens.to_string());
        self.refresh_options(&llm.options);

        let mut editable: Vec<&NSControl> = vec![
            &controls.url,
            &controls.model,
            &controls.api_key_secure,
            &controls.api_key_plain,
            &controls.api_key_toggle,
            &controls.max_tokens_slider,
            &controls.max_tokens_field,
        ];
        let rows = self.ivars().option_rows.borrow();
        for option in rows.iter() {
            editable.extend([&*option.key as &NSControl, &option.value, &option.remove]);
        }
        let add = self.ivars().add_option.get();
        if let Some(add) = add {
            editable.push(add);
        }
        for control in editable {
            control.setEnabled(external);
        }
    }

    /// Shows one row of fields per model option, and the + button.
    fn refresh_options(&self, options: &[ModelOption]) {
        let Some(ui) = self.ui() else {
            return;
        };
        let mtm = self.mtm();
        let target = self.target();
        let stack = &ui.controls.options;
        let add = self.ivars().add_option.get_or_init(|| {
            symbol_button(mtm, "plus", "Add an option", target, sel!(smvAddOption:))
        });
        let mut rows = self.ivars().option_rows.borrow_mut();
        if rows.len() != options.len() {
            for subview in stack.arrangedSubviews() {
                stack.removeArrangedSubview(&subview);
                subview.removeFromSuperview();
            }
            rows.clear();
            for index in 0..options.len() {
                let key = text_field(mtm, "Key", 150.0, target);
                key.setTag((index * 2) as isize);
                unsafe { key.setAction(Some(sel!(smvOptionChanged:))) };
                let value = text_field(mtm, "Value", 200.0, target);
                value.setTag((index * 2 + 1) as isize);
                unsafe { value.setAction(Some(sel!(smvOptionChanged:))) };
                let remove = symbol_button(
                    mtm,
                    "minus",
                    "Remove the option",
                    target,
                    sel!(smvRemoveOption:),
                );
                remove.setTag(index as isize);
                stack.addArrangedSubview(&hstack(mtm, &[view(&key), view(&value), view(&remove)]));
                rows.push(OptionRow { key, value, remove });
            }
            stack.addArrangedSubview(add);
        }
        for (row, option) in rows.iter().zip(options) {
            set_text(&row.key, &option.key);
            set_text(&row.value, &option.value);
        }
    }

    /// Filters the list by the text of the search field (STANDARD §9.1).
    fn apply_search(&self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let text = ui.search.stringValue().to_string();
        let rows: Vec<Vec<Row>> = ui
            .groups
            .iter()
            .map(|group| {
                group
                    .rows
                    .iter()
                    .map(|row| Row {
                        label: row.label,
                        options: row.options,
                        attached: row.attached,
                    })
                    .collect()
            })
            .collect();
        let specs: Vec<Group> = ui
            .groups
            .iter()
            .zip(&rows)
            .map(|(group, rows)| Group {
                title: group.title,
                rows,
            })
            .collect();
        let visible = settings_search::visible_rows(&text, &specs);
        let mut shown = Vec::new();
        for (index, (group, visible)) in ui.groups.iter().zip(&visible).enumerate() {
            let mut any = false;
            for (row, &visible) in group.rows.iter().zip(visible) {
                row.row.setHidden(!visible || row.hidden_by_settings.get());
                any |= visible;
            }
            group.view.setHidden(!any);
            if any {
                shown.push(index);
            }
        }
        ui.no_results.setHidden(!shown.is_empty());
        let changed = *self.ivars().shown_groups.borrow() != shown;
        if changed {
            self.ivars().shown_groups.replace(shown);
            self.ivars().syncing.set(true);
            ui.sidebar.reloadData();
            self.ivars().syncing.set(false);
        }
        self.follow_list();
    }

    /// Scrolls the list so that the title of the group chosen in the sidebar
    /// is at the top.
    fn scroll_to_selected_group(&self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let row = ui.sidebar.selectedRow();
        let Some(&group) = self.ivars().shown_groups.borrow().get(row.max(0) as usize) else {
            return;
        };
        if row < 0 {
            return;
        }
        let view = &ui.groups[group].view;
        let frame = view.convertRect_toView(view.bounds(), Some(&ui.document));
        let clip = ui.list.contentView();
        let document = ui.document.frame().size.height;
        let visible = clip.bounds().size.height;
        let y = (frame.origin.y - 16.0).clamp(0.0, (document - visible).max(0.0));
        clip.scrollToPoint(NSPoint::new(0.0, y));
        ui.list.reflectScrolledClipView(&clip);
    }

    /// Selects the group at the top of the list in the sidebar.
    fn follow_list(&self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let shown = self.ivars().shown_groups.borrow().clone();
        if shown.is_empty() {
            return;
        }
        if let Some(content) = self.window().and_then(|w| w.contentView()) {
            content.layoutSubtreeIfNeeded();
        }
        let clip = ui.list.contentView();
        let top = clip.bounds().origin.y;
        let at_bottom = top + clip.bounds().size.height >= ui.document.frame().size.height - 1.0;
        let mut current = 0;
        for (row, &group) in shown.iter().enumerate() {
            let view = &ui.groups[group].view;
            let frame = view.convertRect_toView(view.bounds(), Some(&ui.document));
            if frame.origin.y <= top + 40.0 {
                current = row;
            }
        }
        if at_bottom && top > 0.0 {
            current = shown.len() - 1;
        }
        if ui.sidebar.selectedRow() != current as NSInteger {
            self.ivars().syncing.set(true);
            ui.sidebar.selectRowIndexes_byExtendingSelection(
                &NSIndexSet::indexSetWithIndex(current),
                false,
            );
            self.ivars().syncing.set(false);
        }
    }

    fn rename_active_font_set(&self) {
        let mtm = self.mtm();
        let current = settings_store::current().active_font_set;
        let alert = NSAlert::new(mtm);
        alert.setMessageText(&ns("Rename Font Set"));
        alert.setInformativeText(&ns(&format!("Enter a new name for “{current}”.")));
        alert.addButtonWithTitle(&ns("Rename"));
        alert.addButtonWithTitle(&ns("Cancel"));
        let field = NSTextField::textFieldWithString(&ns(&current), mtm);
        field.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(240.0, 24.0),
        ));
        alert.setAccessoryView(Some(&field));
        alert.window().setInitialFirstResponder(Some(&field));
        if alert.runModal() != NSAlertFirstButtonReturn {
            return;
        }
        let new_name = field.stringValue().to_string();
        let result = settings_store::current().validate_name(&new_name, Some(&current));
        match result {
            Ok(()) => settings_store::update(mtm, |s| {
                let _ = s.rename_font_set(&current, &new_name);
            }),
            Err(error) => {
                let alert = NSAlert::new(mtm);
                alert.setAlertStyle(NSAlertStyle::Warning);
                alert.setMessageText(&ns("The font set could not be renamed."));
                alert.setInformativeText(&ns(&error.to_string()));
                alert.runModal();
            }
        }
    }

    fn confirm(&self, message: &str, information: &str, action: &str) -> bool {
        let alert = NSAlert::new(self.mtm());
        alert.setMessageText(&ns(message));
        alert.setInformativeText(&ns(information));
        alert.addButtonWithTitle(&ns(action));
        alert.addButtonWithTitle(&ns("Cancel"));
        alert.runModal() == NSAlertFirstButtonReturn
    }
}

/// Index of the slider step closest to `tokens`.
fn nearest_step(tokens: u32) -> usize {
    let distance = |step: u32| (f64::from(step).ln() - f64::from(tokens.max(1)).ln()).abs();
    (0..MAX_TOKENS_STEPS.len())
        .min_by(|&a, &b| distance(MAX_TOKENS_STEPS[a]).total_cmp(&distance(MAX_TOKENS_STEPS[b])))
        .unwrap_or(0)
}

/// A titled group of rows (STANDARD §9.1).
fn group(mtm: MainThreadMarker, title: &'static str, specs: Vec<RowSpec>) -> GroupInfo {
    let views: Vec<Retained<NSArray<NSView>>> = specs
        .iter()
        .map(|spec| {
            let label = if spec.label.is_empty() {
                empty(mtm)
            } else {
                label(mtm, spec.label)
            };
            NSArray::from_retained_slice(&[label, spec.content.clone()])
        })
        .collect();
    let grid = NSGridView::gridViewWithViews(&NSArray::from_retained_slice(&views), mtm);
    grid.setRowSpacing(10.0);
    grid.setColumnSpacing(10.0);
    grid.setYPlacement(NSGridCellPlacement::Center);
    let labels = grid.columnAtIndex(0);
    labels.setXPlacement(NSGridCellPlacement::Trailing);
    labels.setWidth(LABEL_WIDTH);
    let rows = specs
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            let row = grid.rowAtIndex(index as NSInteger);
            if spec.align == Align::Top {
                row.setYPlacement(NSGridCellPlacement::Top);
            }
            RowInfo {
                row,
                label: spec.label,
                options: spec.options,
                attached: spec.attached,
                hidden_by_settings: Cell::new(false),
            }
        })
        .collect();
    let heading = NSTextField::labelWithString(&ns(title), mtm);
    heading.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
    let stack = vstack(mtm, &[view(&heading), view(&grid)], 12.0);
    GroupInfo {
        title,
        view: Retained::into_super(stack),
        rows,
    }
}

fn view<T: AsRef<NSView>>(control: &T) -> Retained<NSView> {
    control.as_ref().retain()
}

fn empty(mtm: MainThreadMarker) -> Retained<NSView> {
    NSGridCell::emptyContentView(mtm)
}

fn label(mtm: MainThreadMarker, text: &str) -> Retained<NSView> {
    view(&NSTextField::labelWithString(&ns(text), mtm))
}

/// A small secondary text that wraps.
fn note(mtm: MainThreadMarker, text: &str) -> Retained<NSTextField> {
    let note = NSTextField::wrappingLabelWithString(&ns(text), mtm);
    note.setTextColor(Some(&NSColor::secondaryLabelColor()));
    note.setFont(Some(&NSFont::systemFontOfSize(
        NSFont::smallSystemFontSize(),
    )));
    note.setPreferredMaxLayoutWidth(320.0);
    note
}

fn hstack(mtm: MainThreadMarker, views: &[Retained<NSView>]) -> Retained<NSView> {
    let stack = NSStackView::stackViewWithViews(&NSArray::from_retained_slice(views), mtm);
    stack.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
    stack.setSpacing(8.0);
    view(&stack)
}

fn vstack(
    mtm: MainThreadMarker,
    views: &[Retained<NSView>],
    spacing: f64,
) -> Retained<NSStackView> {
    let stack = NSStackView::stackViewWithViews(&NSArray::from_retained_slice(views), mtm);
    stack.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
    stack.setAlignment(NSLayoutAttribute::Leading);
    stack.setSpacing(spacing);
    stack.setEdgeInsets(NSEdgeInsets {
        top: 0.0,
        left: 0.0,
        bottom: 0.0,
        right: 0.0,
    });
    stack
}

fn symbol_button(
    mtm: MainThreadMarker,
    symbol: &str,
    description: &str,
    target: &AnyObject,
    action: Sel,
) -> Retained<NSButton> {
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
        &ns(symbol),
        Some(&ns(description)),
    )
    .expect("SF Symbols are available on macOS 26");
    let button =
        unsafe { NSButton::buttonWithImage_target_action(&image, Some(target), Some(action), mtm) };
    button.setToolTip(Some(&ns(description)));
    button
}

fn radio(
    mtm: MainThreadMarker,
    title: &str,
    target: &AnyObject,
    action: Sel,
) -> Retained<NSButton> {
    unsafe {
        NSButton::radioButtonWithTitle_target_action(&ns(title), Some(target), Some(action), mtm)
    }
}

fn set_radio(button: &NSButton, on: bool) {
    button.setState(if on {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
}

/// Sets the text of a field unless the user is editing it.
fn set_text(field: &NSTextField, text: &str) {
    if field.currentEditor().is_none() && field.stringValue().to_string() != text {
        field.setStringValue(&ns(text));
    }
}

/// A text field whose action is sent when Return is pressed or the field
/// loses focus.
fn text_field(
    mtm: MainThreadMarker,
    placeholder: &str,
    width: f64,
    target: &AnyObject,
) -> Retained<NSTextField> {
    let field = NSTextField::new(mtm);
    configure_text_field(&field, placeholder, width, target);
    field
}

fn configure_text_field(field: &NSTextField, placeholder: &str, width: f64, target: &AnyObject) {
    field.setPlaceholderString(Some(&ns(placeholder)));
    unsafe { field.setTarget(Some(target)) };
    field
        .widthAnchor()
        .constraintEqualToConstant(width)
        .setActive(true);
    if let Some(cell) = field.cell() {
        cell.setSendsActionOnEndEditing(true);
    }
}

fn color_well(mtm: MainThreadMarker, tag: isize, target: &AnyObject) -> Retained<NSColorWell> {
    let well = NSColorWell::new(mtm);
    well.setTag(tag);
    unsafe {
        well.setTarget(Some(target));
        well.setAction(Some(sel!(smvColorChanged:)));
    }
    well.widthAnchor()
        .constraintEqualToConstant(56.0)
        .setActive(true);
    well
}

fn number_input(
    mtm: MainThreadMarker,
    min: f64,
    max: f64,
    increment: f64,
    action: Sel,
    target: &AnyObject,
) -> (Retained<NSTextField>, Retained<NSStepper>) {
    let field = NSTextField::new(mtm);
    unsafe {
        field.setTarget(Some(target));
        field.setAction(Some(action));
    }
    field
        .widthAnchor()
        .constraintEqualToConstant(56.0)
        .setActive(true);
    // Apply the value when the field loses focus, not only on Return.
    if let Some(cell) = field.cell() {
        cell.setSendsActionOnEndEditing(true);
    }
    let stepper = NSStepper::new(mtm);
    stepper.setMinValue(min);
    stepper.setMaxValue(max);
    stepper.setIncrement(increment);
    stepper.setValueWraps(false);
    unsafe {
        stepper.setTarget(Some(target));
        stepper.setAction(Some(action));
    }
    (field, stepper)
}

fn family_popup(
    mtm: MainThreadMarker,
    families: &[String],
    tag: isize,
    target: &AnyObject,
) -> Retained<NSPopUpButton> {
    let popup = NSPopUpButton::new(mtm);
    popup.setTag(tag);
    unsafe {
        popup.setTarget(Some(target));
        popup.setAction(Some(sel!(smvFontFamilyChanged:)));
    }
    popup.addItemWithTitle(&ns("System Font"));
    if let Some(item) = popup.lastItem() {
        item.setTag(SYSTEM_FONT_TAG);
    }
    if let Some(menu) = popup.menu() {
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        for family in families {
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &ns(family),
                    None,
                    &ns(""),
                )
            };
            menu.addItem(&item);
        }
    }
    popup
}

/// Selects `family` (or the System Font entry). A family that is not
/// installed is added so the setting stays visible.
fn select_family(popup: &NSPopUpButton, family: Option<&str>) {
    let Some(family) = family else {
        popup.selectItemWithTag(SYSTEM_FONT_TAG);
        return;
    };
    if popup.itemWithTitle(&ns(family)).is_none() {
        popup.addItemWithTitle(&ns(family));
    }
    popup.selectItemWithTitle(&ns(family));
}

#[cfg(test)]
mod tests {
    use super::nearest_step;

    #[test]
    fn slider_steps() {
        assert_eq!(nearest_step(32 * 1024), 2);
        assert_eq!(nearest_step(1024), 0);
        assert_eq!(nearest_step(2_000_000), 7);
        assert_eq!(nearest_step(100_000), 4);
    }
}
