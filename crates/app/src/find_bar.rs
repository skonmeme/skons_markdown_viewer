//! The find bar shown above the document (STANDARD §6).

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{MainThreadMarker, Message, sel};
use objc2_app_kit::{
    NSBox, NSBoxType, NSButton, NSColor, NSControlStateValueOn, NSFont, NSImage,
    NSLayoutConstraint, NSLayoutConstraintOrientation, NSSearchField, NSSegmentSwitchTracking,
    NSSegmentedControl, NSStackView, NSTextField, NSUserInterfaceLayoutOrientation, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
};
use objc2_foundation::{NSArray, NSEdgeInsets};
use objc2_web_kit::WKWebView;

use crate::util::ns;
use crate::viewer::ViewerController;

pub struct FindBar {
    view: Retained<NSView>,
    field: Retained<NSSearchField>,
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

        let field = NSSearchField::new(mtm);
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
        self.set_status("");
    }

    pub fn query(&self) -> String {
        self.field.stringValue().to_string()
    }

    pub fn set_query(&self, query: &str) {
        self.field.setStringValue(&ns(query));
    }

    pub fn case_sensitive(&self) -> bool {
        self.match_case.state() == NSControlStateValueOn
    }

    pub fn set_status(&self, text: &str) {
        self.status.setStringValue(&ns(text));
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
    field: Retained<NSSearchField>,
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
            navigation: self.navigation,
            status: self.status,
            match_case: self.match_case,
            web_top_hidden,
            web_top_shown,
        }
    }
}
