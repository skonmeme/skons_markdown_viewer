//! The Settings window (STANDARD §9.1).

use std::cell::{OnceCell, RefCell};

use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2::{ClassType, Message};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSBackingStoreType, NSBox, NSBoxType,
    NSButton, NSColor, NSColorWell, NSControl, NSControlStateValueOff, NSControlStateValueOn,
    NSFont, NSGridCell, NSGridCellPlacement, NSGridRow, NSGridView, NSImage, NSLayoutConstraint,
    NSMenuItem, NSPopUpButton, NSResponder, NSSlider, NSStackView, NSStepper, NSTextField,
    NSUserInterfaceLayoutOrientation, NSView, NSWindow, NSWindowController, NSWindowStyleMask,
};
use objc2_foundation::{NSArray, NSObject, NSPoint, NSRange, NSRect, NSSize};
use smv_core::settings::{
    ColorMode, FIXED_MARGIN_MAX, FIXED_MARGIN_MIN, FONT_SIZE_MAX, FONT_SIZE_MIN, LINE_HEIGHT_MAX,
    LINE_HEIGHT_MIN, MarginMode, Settings,
};

use crate::fonts;
use crate::settings_store;
use crate::util::{self, color, ns};

/// Tag of the "System Font" entry in the font family pop-up buttons.
const SYSTEM_FONT_TAG: isize = 1;

/// Tags telling the shared actions which control sent them.
const TAG_LATIN: isize = 1;
const TAG_KOREAN: isize = 2;
const TAG_MONOSPACE: isize = 3;
const TAG_TEXT_COLOR: isize = 1;
const TAG_BACKGROUND_COLOR: isize = 2;

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
    default_hint_row: Retained<NSGridRow>,
    color_automatic: Retained<NSButton>,
    color_custom: Retained<NSButton>,
    text_color: Retained<NSColorWell>,
    background_color: Retained<NSColorWell>,
    margin_automatic: Retained<NSButton>,
    margin_fixed: Retained<NSButton>,
    margin_slider: Retained<NSSlider>,
    margin_label: Retained<NSTextField>,
}

#[derive(Default)]
pub struct SettingsIvars {
    controls: OnceCell<Controls>,
}

thread_local! {
    static CONTROLLER: RefCell<Option<Retained<SettingsController>>> = const { RefCell::new(None) };
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
            let custom = self.controls().color_custom.state() == NSControlStateValueOn;
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
            let fixed = self.controls().margin_fixed.state() == NSControlStateValueOn;
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
            // Do not start with the font size field focused.
            window.makeFirstResponder(None);
        }
    }

    /// Updates the controls after the settings changed elsewhere.
    pub fn refresh_if_open(_mtm: MainThreadMarker) {
        let controller = CONTROLLER.with(|cell| cell.borrow().clone());
        if let Some(controller) = controller {
            controller.refresh();
        }
    }

    /// PNG image of the window content, for diagnostics.
    pub fn snapshot_png(_mtm: MainThreadMarker) -> Option<Vec<u8>> {
        let controller = CONTROLLER.with(|cell| cell.borrow().clone())?;
        let content = controller.window()?.contentView()?;
        crate::diagnostics::render_png(&content, None)
    }

    fn create(mtm: MainThreadMarker) -> Retained<Self> {
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(520.0, 480.0)),
                NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&ns("Settings"));

        let this = Self::alloc(mtm).set_ivars(SettingsIvars::default());
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithWindow: &*window] };
        let (content, controls) = this.build(mtm);
        window.setContentView(Some(&content));
        content.layoutSubtreeIfNeeded();
        window.setContentSize(content.fittingSize());
        let autosave_name = ns("SMVSettingsWindow");
        if !window.setFrameUsingName(&autosave_name) {
            window.center();
        }
        window.setFrameAutosaveName(&autosave_name);
        let _ = this.ivars().controls.set(controls);
        this
    }

    fn controls(&self) -> &Controls {
        self.ivars()
            .controls
            .get()
            .expect("controls are created in create()")
    }

    fn target(&self) -> &AnyObject {
        self.as_ref()
    }

    fn build(&self, mtm: MainThreadMarker) -> (Retained<NSView>, Controls) {
        let target = self.target();
        let lists = fonts::lists(mtm);

        let font_set = NSPopUpButton::new(mtm);
        unsafe {
            font_set.setTarget(Some(target));
            font_set.setAction(Some(sel!(smvSelectFontSet:)));
        }
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

        let default_hint = NSTextField::wrappingLabelWithString(
            &ns("The Default font set cannot be changed. Click + to make an editable copy."),
            mtm,
        );
        default_hint.setTextColor(Some(&NSColor::secondaryLabelColor()));
        default_hint.setFont(Some(&NSFont::systemFontOfSize(
            NSFont::smallSystemFontSize(),
        )));
        default_hint.setPreferredMaxLayoutWidth(300.0);

        let color_automatic = radio(mtm, "Automatic", target, sel!(smvColorModeChanged:));
        let color_custom = radio(mtm, "Custom", target, sel!(smvColorModeChanged:));
        let text_color = color_well(mtm, TAG_TEXT_COLOR, target);
        let background_color = color_well(mtm, TAG_BACKGROUND_COLOR, target);

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

        let rows: Vec<[Retained<NSView>; 2]> = vec![
            [
                label(mtm, "Font set:"),
                hstack(
                    mtm,
                    &[view(&font_set), view(&add), view(&remove), view(&rename)],
                ),
            ],
            [label(mtm, "Latin font:"), view(&latin)],
            [label(mtm, "Korean font:"), view(&korean)],
            [label(mtm, "Monospace font:"), view(&monospace)],
            [
                label(mtm, "Font size:"),
                hstack(
                    mtm,
                    &[view(&size_field), view(&size_stepper), label(mtm, "pt")],
                ),
            ],
            [
                label(mtm, "Line height:"),
                hstack(mtm, &[view(&line_height_field), view(&line_height_stepper)]),
            ],
            [empty(mtm), view(&default_hint)],
            [separator(mtm), empty(mtm)],
            [
                label(mtm, "Colors:"),
                hstack(mtm, &[view(&color_automatic), view(&color_custom)]),
            ],
            [label(mtm, "Text color:"), view(&text_color)],
            [label(mtm, "Background color:"), view(&background_color)],
            [separator(mtm), empty(mtm)],
            [
                label(mtm, "Margins:"),
                hstack(mtm, &[view(&margin_automatic), view(&margin_fixed)]),
            ],
            [
                label(mtm, "Fixed margin:"),
                hstack(mtm, &[view(&margin_slider), view(&margin_label)]),
            ],
        ];
        let hint_view = view(&default_hint);
        let hint_row = rows
            .iter()
            .position(|row| Retained::as_ptr(&row[1]) == Retained::as_ptr(&hint_view))
            .expect("the hint is one of the rows");
        let separator_rows: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row[0].isKindOfClass(NSBox::class()))
            .map(|(index, _)| index)
            .collect();
        let rows: Vec<Retained<NSArray<NSView>>> = rows
            .iter()
            .map(|row| NSArray::from_retained_slice(row))
            .collect();
        let grid = NSGridView::gridViewWithViews(&NSArray::from_retained_slice(&rows), mtm);
        grid.setRowSpacing(10.0);
        grid.setColumnSpacing(10.0);
        grid.setYPlacement(NSGridCellPlacement::Center);
        grid.columnAtIndex(0)
            .setXPlacement(NSGridCellPlacement::Trailing);
        let default_hint_row = grid.rowAtIndex(hint_row as isize);
        for row in separator_rows {
            grid.mergeCellsInHorizontalRange_verticalRange(
                NSRange::new(0, 2),
                NSRange::new(row, 1),
            );
            grid.rowAtIndex(row as isize).setTopPadding(4.0);
            grid.rowAtIndex(row as isize).setBottomPadding(4.0);
        }
        for popup in [&latin, &korean, &monospace] {
            popup
                .widthAnchor()
                .constraintEqualToConstant(260.0)
                .setActive(true);
        }
        font_set
            .widthAnchor()
            .constraintGreaterThanOrEqualToConstant(140.0)
            .setActive(true);

        let content = NSView::new(mtm);
        grid.setTranslatesAutoresizingMaskIntoConstraints(false);
        content.addSubview(&grid);
        NSLayoutConstraint::activateConstraints(&NSArray::from_retained_slice(&[
            grid.topAnchor()
                .constraintEqualToAnchor_constant(&content.topAnchor(), 20.0),
            grid.bottomAnchor()
                .constraintEqualToAnchor_constant(&content.bottomAnchor(), -20.0),
            grid.leadingAnchor()
                .constraintEqualToAnchor_constant(&content.leadingAnchor(), 24.0),
            grid.trailingAnchor()
                .constraintEqualToAnchor_constant(&content.trailingAnchor(), -24.0),
        ]));

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
            default_hint_row,
            color_automatic,
            color_custom,
            text_color,
            background_color,
            margin_automatic,
            margin_fixed,
            margin_slider,
            margin_label,
        };
        (content, controls)
    }

    /// Shows the current settings in the controls.
    fn refresh(&self) {
        let Some(controls) = self.ivars().controls.get() else {
            return;
        };
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

        controls
            .size_field
            .setStringValue(&ns(&format!("{}", set.size)));
        controls.size_stepper.setDoubleValue(set.size);
        controls
            .line_height_field
            .setStringValue(&ns(&format!("{:.2}", set.line_height)));
        controls.line_height_stepper.setDoubleValue(set.line_height);
        for control in [
            &*controls.size_field as &NSControl,
            &controls.size_stepper,
            &controls.line_height_field,
            &controls.line_height_stepper,
        ] {
            control.setEnabled(editable);
        }
        controls.default_hint_row.setHidden(editable);

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
        self.fit_window();
    }

    /// Resizes the window to its content, keeping the top edge in place.
    fn fit_window(&self) {
        let Some(window) = self.window() else {
            return;
        };
        let Some(content) = window.contentView() else {
            return;
        };
        content.layoutSubtreeIfNeeded();
        let size = content.fittingSize();
        let current = window.contentRectForFrameRect(window.frame());
        if (current.size.height - size.height).abs() < 0.5
            && (current.size.width - size.width).abs() < 0.5
        {
            return;
        }
        let top = current.origin.y + current.size.height;
        let content_rect = NSRect::new(NSPoint::new(current.origin.x, top - size.height), size);
        window.setFrame_display(window.frameRectForContentRect(content_rect), true);
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

fn view<T: AsRef<NSView>>(control: &T) -> Retained<NSView> {
    control.as_ref().retain()
}

fn empty(mtm: MainThreadMarker) -> Retained<NSView> {
    NSGridCell::emptyContentView(mtm)
}

fn label(mtm: MainThreadMarker, text: &str) -> Retained<NSView> {
    view(&NSTextField::labelWithString(&ns(text), mtm))
}

fn separator(mtm: MainThreadMarker) -> Retained<NSView> {
    let separator = NSBox::new(mtm);
    separator.setBoxType(NSBoxType::Separator);
    view(&separator)
}

fn hstack(mtm: MainThreadMarker, views: &[Retained<NSView>]) -> Retained<NSView> {
    let stack = NSStackView::stackViewWithViews(&NSArray::from_retained_slice(views), mtm);
    stack.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
    stack.setSpacing(8.0);
    view(&stack)
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
