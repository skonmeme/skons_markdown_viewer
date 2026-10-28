//! The find bar shown above the document (STANDARD §6.1–§6.3).

use std::cell::Cell;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, ProtocolObject};
use objc2::{
    ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, sel,
};
use objc2_app_kit::{
    NSActionCell, NSAutoresizingMaskOptions, NSBox, NSBoxType, NSButton, NSCell, NSColor,
    NSControl, NSControlStateValueOn, NSFont, NSImage, NSLayoutConstraint,
    NSLayoutConstraintOrientation, NSResponder, NSSearchField, NSSearchFieldCell,
    NSSegmentSwitchTracking, NSSegmentedControl, NSStackView, NSTextField, NSTextFieldCell,
    NSTitlePosition, NSUserInterfaceLayoutOrientation, NSView, NSVisualEffectBlendingMode,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
};
use objc2_foundation::{NSArray, NSEdgeInsets, NSObject, NSPoint, NSRange, NSRect, NSSize};
use objc2_web_kit::WKWebView;
use smv_core::search::Mode;

use crate::util::ns;
use crate::viewer::ViewerController;

/// Height of the mode token and the gaps around it.
const TOKEN_HEIGHT: f64 = 16.0;
const TOKEN_PADDING: f64 = 6.0;
const TOKEN_GAP: f64 = 4.0;

define_class!(
    // SAFETY: NSSearchFieldCell may be subclassed. The class has no ivars,
    // so the copies that NSCell makes stay valid.
    #[unsafe(super(NSSearchFieldCell, NSTextFieldCell, NSActionCell, NSCell, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SMVSearchFieldCell"]
    struct SearchFieldCell;

    impl SearchFieldCell {
        // Both rectangles leave room for the mode token in front of the
        // text: the first one is used while editing, the second one for
        // drawing.
        #[unsafe(method(searchTextRectForBounds:))]
        fn search_text_rect(&self, bounds: NSRect) -> NSRect {
            let rect: NSRect = unsafe { msg_send![super(self), searchTextRectForBounds: bounds] };
            self.inset_rect(rect)
        }

        #[unsafe(method(drawingRectForBounds:))]
        fn drawing_rect(&self, bounds: NSRect) -> NSRect {
            let rect: NSRect = unsafe { msg_send![super(self), drawingRectForBounds: bounds] };
            self.inset_rect(rect)
        }
    }
);

impl SearchFieldCell {
    /// Width taken by the mode token of the field.
    fn inset(&self) -> f64 {
        unsafe { self.controlView() }
            .and_then(|view| view.downcast::<SearchField>().ok())
            .map_or(0.0, |field| field.ivars().inset.get())
    }

    fn inset_rect(&self, mut rect: NSRect) -> NSRect {
        let inset = self.inset();
        rect.origin.x += inset;
        rect.size.width = (rect.size.width - inset).max(0.0);
        rect
    }
}

#[derive(Default)]
pub struct SearchFieldIvars {
    /// Width taken by the mode token.
    inset: Cell<f64>,
}

define_class!(
    // SAFETY: NSSearchField may be subclassed; the ivars are set before init.
    #[unsafe(super(NSSearchField, NSTextField, NSControl, NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SMVSearchField"]
    #[ivars = SearchFieldIvars]
    pub struct SearchField;

    impl SearchField {
        #[unsafe(method(cellClass))]
        fn cell_class() -> &'static AnyClass {
            SearchFieldCell::class()
        }
    }
);

impl SearchField {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(SearchFieldIvars::default());
        unsafe { msg_send![super(this), init] }
    }
}

pub struct FindBar {
    view: Retained<NSView>,
    field: Retained<SearchField>,
    token_box: Retained<NSBox>,
    token_label: Retained<NSTextField>,
    token: Cell<Option<Mode>>,
    navigation: Retained<NSSegmentedControl>,
    status: Retained<NSTextField>,
    match_case: Retained<NSButton>,
    /// Web view top pinned to the window top (find bar hidden).
    web_top_hidden: Retained<NSLayoutConstraint>,
    /// Web view top pinned to the find bar bottom (find bar shown).
    web_top_shown: Retained<NSLayoutConstraint>,
}

impl FindBar {
    /// Creates the find bar views; `install` places them in a window.
    pub fn builder(mtm: MainThreadMarker, target: &ViewerController) -> FindBarBuilder {
        let target_object: &AnyObject = target.as_ref();

        let field = SearchField::new(mtm);
        field.setPlaceholderString(Some(&ns("Find")));
        unsafe {
            field.setTarget(Some(target_object));
            field.setAction(Some(sel!(smvFindFieldChanged:)));
        }
        unsafe { field.setDelegate(Some(ProtocolObject::from_ref(target))) };
        field.setContentHuggingPriority_forOrientation(
            1.0,
            NSLayoutConstraintOrientation::Horizontal,
        );
        // The clear button removes the token as well as the text.
        if let Some(cancel) = field
            .cell()
            .and_then(|cell| cell.downcast::<NSSearchFieldCell>().ok())
            .and_then(|cell| cell.cancelButtonCell())
        {
            unsafe {
                cancel.setTarget(Some(target_object));
                cancel.setAction(Some(sel!(smvFindClear:)));
            }
        }

        let token_label = NSTextField::labelWithString(&ns(""), mtm);
        token_label.setFont(Some(&NSFont::systemFontOfSize(11.0)));
        token_label.setTextColor(Some(&NSColor::labelColor()));
        let token_box = NSBox::new(mtm);
        token_box.setBoxType(NSBoxType::Custom);
        token_box.setTitlePosition(NSTitlePosition::NoTitle);
        token_box.setCornerRadius(TOKEN_HEIGHT / 2.0);
        token_box.setBorderWidth(1.0);
        token_box.setBorderColor(&NSColor::separatorColor());
        token_box.setFillColor(&NSColor::quaternarySystemFillColor());
        token_box.setContentViewMargins(NSSize::new(0.0, 0.0));
        if let Some(content) = token_box.contentView() {
            content.addSubview(&token_label);
        }
        token_box.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewMaxXMargin
                | NSAutoresizingMaskOptions::ViewMinYMargin
                | NSAutoresizingMaskOptions::ViewMaxYMargin,
        );
        token_box.setHidden(true);
        field.addSubview(&token_box);

        let images =
            [("chevron.left", "Previous"), ("chevron.right", "Next")].map(|(symbol, label)| {
                NSImage::imageWithSystemSymbolName_accessibilityDescription(
                    &ns(symbol),
                    Some(&ns(label)),
                )
                .expect("SF Symbols are available on macOS 26")
            });
        let navigation = unsafe {
            NSSegmentedControl::segmentedControlWithImages_trackingMode_target_action(
                &NSArray::from_retained_slice(&images),
                NSSegmentSwitchTracking::Momentary,
                Some(target_object),
                Some(sel!(smvFindNavigate:)),
                mtm,
            )
        };

        let status = NSTextField::labelWithString(&ns(""), mtm);
        status.setTextColor(Some(&NSColor::secondaryLabelColor()));
        status.setFont(Some(&NSFont::systemFontOfSize(
            NSFont::smallSystemFontSize(),
        )));
        let min_width = status
            .widthAnchor()
            .constraintGreaterThanOrEqualToConstant(72.0);
        min_width.setActive(true);

        let match_case = unsafe {
            NSButton::checkboxWithTitle_target_action(
                &ns("Match Case"),
                Some(target_object),
                Some(sel!(smvFindOptionsChanged:)),
                mtm,
            )
        };
        let done = unsafe {
            NSButton::buttonWithTitle_target_action(
                &ns("Done"),
                Some(target_object),
                Some(sel!(smvCloseFind:)),
                mtm,
            )
        };

        let views: [Retained<NSView>; 5] = [
            as_view(&*field),
            as_view(&*navigation),
            as_view(&*status),
            as_view(&*match_case),
            as_view(&*done),
        ];
        let stack = NSStackView::stackViewWithViews(&NSArray::from_retained_slice(&views), mtm);
        stack.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
        stack.setSpacing(8.0);
        stack.setEdgeInsets(NSEdgeInsets {
            top: 6.0,
            left: 10.0,
            bottom: 6.0,
            right: 10.0,
        });

        let separator = NSBox::new(mtm);
        separator.setBoxType(NSBoxType::Separator);

        // A system material keeps the native controls legible on any custom
        // document background.
        let background = NSVisualEffectView::new(mtm);
        background.setMaterial(NSVisualEffectMaterial::HeaderView);
        background.setBlendingMode(NSVisualEffectBlendingMode::WithinWindow);
        background.setState(NSVisualEffectState::FollowsWindowActiveState);
        let view: Retained<NSView> = Retained::into_super(background);
        for subview in [&*stack as &NSView, &separator] {
            subview.setTranslatesAutoresizingMaskIntoConstraints(false);
            view.addSubview(subview);
        }
        NSLayoutConstraint::activateConstraints(&NSArray::from_retained_slice(&[
            stack.topAnchor().constraintEqualToAnchor(&view.topAnchor()),
            stack
                .leadingAnchor()
                .constraintEqualToAnchor(&view.leadingAnchor()),
            stack
                .trailingAnchor()
                .constraintEqualToAnchor(&view.trailingAnchor()),
            separator
                .topAnchor()
                .constraintEqualToAnchor(&stack.bottomAnchor()),
            separator
                .leadingAnchor()
                .constraintEqualToAnchor(&view.leadingAnchor()),
            separator
                .trailingAnchor()
                .constraintEqualToAnchor(&view.trailingAnchor()),
            separator
                .bottomAnchor()
                .constraintEqualToAnchor(&view.bottomAnchor()),
        ]));
        view.setHidden(true);

        FindBarBuilder {
            view,
            field,
            token_box,
            token_label,
            navigation,
            status,
            match_case,
        }
    }

    pub fn field(&self) -> &NSSearchField {
        &self.field
    }

    pub fn is_visible(&self) -> bool {
        !self.view.isHidden()
    }

    pub fn show(&self) {
        if self.is_visible() {
            return;
        }
        self.view.setHidden(false);
        self.web_top_hidden.setActive(false);
        self.web_top_shown.setActive(true);
    }

    pub fn hide(&self) {
        self.view.setHidden(true);
        self.web_top_shown.setActive(false);
        self.web_top_hidden.setActive(true);
        self.set_status("", None);
    }

    /// The query: the text after the token.
    pub fn query(&self) -> String {
        self.field.stringValue().to_string()
    }

    pub fn set_query(&self, query: &str) {
        self.field.setStringValue(&ns(query));
    }

    /// The insertion point in the field while it is edited, in UTF-16 code
    /// units.
    pub fn caret(&self) -> Option<usize> {
        let editor = self.field.currentEditor()?;
        let selection = editor.selectedRange();
        (selection.length == 0).then_some(selection.location)
    }

    /// The mode of the token, or `None` without a token (STANDARD §6.3).
    pub fn token(&self) -> Option<Mode> {
        self.token.get()
    }

    /// Shows or removes the token. While the field is edited, the insertion
    /// point is put at `caret` afterwards.
    pub fn set_token(&self, mode: Option<Mode>, caret: usize) {
        self.token.set(mode);
        self.match_case
            .setEnabled(mode.is_none_or(|m| m == Mode::Keyword));
        match mode {
            Some(mode) => {
                self.token_label.setStringValue(&ns(mode.name()));
                self.token_label.sizeToFit();
                let label = self.token_label.frame().size;
                let size = NSSize::new(label.width + 2.0 * TOKEN_PADDING, TOKEN_HEIGHT);
                self.token_label.setFrameOrigin(NSPoint::new(
                    TOKEN_PADDING,
                    ((TOKEN_HEIGHT - label.height) / 2.0).round(),
                ));
                let bounds = self.field.bounds();
                let start = self
                    .field
                    .cell()
                    .and_then(|cell| cell.downcast::<NSSearchFieldCell>().ok())
                    .map_or(22.0, |cell| {
                        let button = cell.searchButtonRectForBounds(bounds);
                        button.origin.x + button.size.width
                    });
                let y = ((bounds.size.height - TOKEN_HEIGHT) / 2.0).round();
                self.token_box
                    .setFrame(NSRect::new(NSPoint::new(start, y), size));
                self.token_box.setHidden(false);
                self.field.ivars().inset.set(size.width + TOKEN_GAP);
            }
            None => {
                self.token_box.setHidden(true);
                self.field.ivars().inset.set(0.0);
            }
        }
        self.refresh_editor(caret);
    }

    /// Lays the field editor out again after the room for the token changed.
    fn refresh_editor(&self, caret: usize) {
        let field: &NSView = &self.field;
        if let Some(window) = field.window()
            && self.field.currentEditor().is_some()
        {
            window.makeFirstResponder(None);
            window.makeFirstResponder(Some(field));
            if let Some(editor) = self.field.currentEditor() {
                editor.setSelectedRange(NSRange::new(caret, 0));
            }
        }
        field.setNeedsDisplay(true);
    }

    pub fn case_sensitive(&self) -> bool {
        self.match_case.state() == NSControlStateValueOn
    }

    /// Shows `text` in the counter; pointing at it shows `reason`.
    pub fn set_status(&self, text: &str, reason: Option<&str>) {
        self.status.setStringValue(&ns(text));
        self.status.setToolTip(reason.map(ns).as_deref());
    }

    /// 0 for "previous", 1 for "next".
    pub fn selected_navigation(&self) -> isize {
        self.navigation.selectedSegment()
    }
}

fn as_view(view: &impl AsRef<NSView>) -> Retained<NSView> {
    view.as_ref().retain()
}

/// A find bar that is not yet placed in a window.
pub struct FindBarBuilder {
    view: Retained<NSView>,
    field: Retained<SearchField>,
    token_box: Retained<NSBox>,
    token_label: Retained<NSTextField>,
    navigation: Retained<NSSegmentedControl>,
    status: Retained<NSTextField>,
    match_case: Retained<NSButton>,
}

impl FindBarBuilder {
    /// Places the find bar above `web_view` inside `content`.
    pub fn install(self, content: &NSView, web_view: &WKWebView) -> FindBar {
        for subview in [&*self.view, web_view as &NSView] {
            subview.setTranslatesAutoresizingMaskIntoConstraints(false);
            content.addSubview(subview);
        }
        let web_top_hidden = web_view
            .topAnchor()
            .constraintEqualToAnchor(&content.topAnchor());
        let web_top_shown = web_view
            .topAnchor()
            .constraintEqualToAnchor(&self.view.bottomAnchor());
        NSLayoutConstraint::activateConstraints(&NSArray::from_retained_slice(&[
            self.view
                .topAnchor()
                .constraintEqualToAnchor(&content.topAnchor()),
            self.view
                .leadingAnchor()
                .constraintEqualToAnchor(&content.leadingAnchor()),
            self.view
                .trailingAnchor()
                .constraintEqualToAnchor(&content.trailingAnchor()),
            web_view
                .leadingAnchor()
                .constraintEqualToAnchor(&content.leadingAnchor()),
            web_view
                .trailingAnchor()
                .constraintEqualToAnchor(&content.trailingAnchor()),
            web_view
                .bottomAnchor()
                .constraintEqualToAnchor(&content.bottomAnchor()),
            web_top_hidden.clone(),
        ]));
        FindBar {
            view: self.view,
            field: self.field,
            token_box: self.token_box,
            token_label: self.token_label,
            token: Cell::new(None),
            navigation: self.navigation,
            status: self.status,
            match_case: self.match_case,
            web_top_hidden,
            web_top_shown,
        }
    }
}
