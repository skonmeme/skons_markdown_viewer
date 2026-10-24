//! Discovery of installed fonts (STANDARD §7).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSFont, NSFontManager};
use objc2_foundation::{NSArray, NSNumber, NSString};
use smv_core::style::FontFace;

use crate::util::ns;

const ITALIC_TRAIT: usize = 0x0000_0001;
const FIXED_PITCH_TRAIT: usize = 0x0000_0400;
/// HANGUL SYLLABLE GA, used to detect fonts that cover Hangul.
const HANGUL_PROBE: u16 = 0xAC00;

/// Font family names offered by the settings window.
pub struct FontLists {
    pub all: Vec<String>,
    pub korean: Vec<String>,
    pub monospace: Vec<String>,
}

struct Member {
    postscript_name: String,
    weight: i64,
    traits: usize,
}

thread_local! {
    static LISTS: RefCell<Option<Rc<FontLists>>> = const { RefCell::new(None) };
    static FACES: RefCell<HashMap<String, Vec<FontFace>>> = RefCell::new(HashMap::new());
}

/// Installed font families, computed once per launch.
pub fn lists(mtm: MainThreadMarker) -> Rc<FontLists> {
    LISTS.with(|cell| {
        cell.borrow_mut()
            .get_or_insert_with(|| Rc::new(compute_lists(mtm)))
            .clone()
    })
}

/// Faces of `family` for the Hangul `@font-face` rules.
pub fn faces(mtm: MainThreadMarker, family: &str) -> Vec<FontFace> {
    FACES.with(|cache| {
        cache
            .borrow_mut()
            .entry(family.to_owned())
            .or_insert_with(|| {
                members(mtm, family)
                    .into_iter()
                    .map(|m| FontFace {
                        postscript_name: m.postscript_name,
                        weight: css_weight(m.weight),
                        italic: m.traits & ITALIC_TRAIT != 0,
                    })
                    .collect()
            })
            .clone()
    })
}

fn compute_lists(mtm: MainThreadMarker) -> FontLists {
    let manager = NSFontManager::sharedFontManager(mtm);
    let mut lists = FontLists {
        all: Vec::new(),
        korean: Vec::new(),
        monospace: Vec::new(),
    };
    for family in manager.availableFontFamilies().iter() {
        let name = family.to_string();
        if name.starts_with('.') {
            continue;
        }
        let members = members(mtm, &name);
        let Some(regular) = members
            .iter()
            .min_by_key(|m| ((m.weight - 5).abs(), m.traits & ITALIC_TRAIT))
        else {
            continue;
        };
        if regular.traits & FIXED_PITCH_TRAIT != 0 {
            lists.monospace.push(name.clone());
        }
        let covers_hangul = NSFont::fontWithName_size(&ns(&regular.postscript_name), 12.0)
            .is_some_and(|font| font.coveredCharacterSet().characterIsMember(HANGUL_PROBE));
        if covers_hangul {
            lists.korean.push(name.clone());
        }
        lists.all.push(name);
    }
    for list in [&mut lists.all, &mut lists.korean, &mut lists.monospace] {
        list.sort_by_key(|name| name.to_lowercase());
    }
    lists
}

fn members(mtm: MainThreadMarker, family: &str) -> Vec<Member> {
    let manager = NSFontManager::sharedFontManager(mtm);
    let Some(members) = manager.availableMembersOfFontFamily(&ns(family)) else {
        return Vec::new();
    };
    members
        .iter()
        .filter_map(|member| parse_member(&member))
        .collect()
}

/// A member is `[PostScript name, style name, weight, traits]`.
fn parse_member(member: &NSArray) -> Option<Member> {
    let field = |index: usize| (index < member.count()).then(|| member.objectAtIndex(index));
    let postscript_name: Retained<NSString> = field(0)?.downcast().ok()?;
    let weight: Retained<NSNumber> = field(2)?.downcast().ok()?;
    let traits: Retained<NSNumber> = field(3)?.downcast().ok()?;
    Some(Member {
        postscript_name: postscript_name.to_string(),
        weight: weight.integerValue() as i64,
        traits: traits.unsignedIntegerValue(),
    })
}

/// Maps an AppKit weight (0–15, 5 = regular, 9 = bold) to a CSS weight.
fn css_weight(appkit_weight: i64) -> u16 {
    match appkit_weight {
        ..=2 => 100,
        3 => 200,
        4 => 300,
        5 => 400,
        6 => 500,
        7 | 8 => 600,
        9 => 700,
        10 => 800,
        _ => 900,
    }
}
