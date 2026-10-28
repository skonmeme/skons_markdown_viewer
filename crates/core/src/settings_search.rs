//! Filtering of the Settings window by the text in its search field
//! (STANDARD §9.1).

/// A row of the Settings window as seen by the search.
pub struct Row<'a> {
    /// The label in front of the row, such as "Font size:".
    pub label: &'a str,
    /// Names of the options of the row, such as the titles of its radio
    /// buttons.
    pub options: &'a [&'a str],
    /// Whether the row belongs to the row above it and is shown with it.
    pub attached: bool,
}

/// A group of rows under a title.
pub struct Group<'a> {
    pub title: &'a str,
    pub rows: &'a [Row<'a>],
}

/// For each group, whether each of its rows is shown for `text`, ignoring
/// case. All rows of a group are shown when its title contains the text;
/// otherwise a row is shown when its label or one of its option names does.
/// An empty text shows everything. A group is shown when any row is.
pub fn visible_rows(text: &str, groups: &[Group]) -> Vec<Vec<bool>> {
    let needle = text.trim().to_lowercase();
    let contains = |haystack: &str| haystack.to_lowercase().contains(&needle);
    groups
        .iter()
        .map(|group| {
            if needle.is_empty() || contains(group.title) {
                return vec![true; group.rows.len()];
            }
            let mut owner = false;
            group
                .rows
                .iter()
                .map(|row| {
                    if !row.attached {
                        owner = contains(row.label) || row.options.iter().any(|o| contains(o));
                    }
                    owner
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORS: [Row; 3] = [
        Row {
            label: "Colors:",
            options: &["Automatic", "Custom"],
            attached: false,
        },
        Row {
            label: "Text color:",
            options: &[],
            attached: false,
        },
        Row {
            label: "Background color:",
            options: &[],
            attached: false,
        },
    ];
    const AI: [Row; 3] = [
        Row {
            label: "Model:",
            options: &["Apple Foundation Model (on this Mac)", "External LLM"],
            attached: false,
        },
        Row {
            label: "",
            options: &[],
            attached: true,
        },
        Row {
            label: "URL:",
            options: &[],
            attached: false,
        },
    ];

    fn groups() -> [Group<'static>; 2] {
        [
            Group {
                title: "Colors",
                rows: &COLORS,
            },
            Group {
                title: "AI",
                rows: &AI,
            },
        ]
    }

    #[test]
    fn empty_text_shows_everything() {
        assert_eq!(
            visible_rows("  ", &groups()),
            vec![vec![true; 3], vec![true; 3]]
        );
    }

    #[test]
    fn group_titles_show_whole_groups() {
        assert_eq!(
            visible_rows("COLORS", &groups()),
            vec![vec![true; 3], vec![false; 3]]
        );
    }

    #[test]
    fn labels_and_options_ignoring_case() {
        assert_eq!(
            visible_rows("background", &groups()),
            vec![vec![false, false, true], vec![false; 3]]
        );
        assert_eq!(
            visible_rows("custom", &groups()),
            vec![vec![true, false, false], vec![false; 3]]
        );
        // "llm" matches an option of the Model row and its attached row.
        assert_eq!(
            visible_rows("llm", &groups()),
            vec![vec![false; 3], vec![true, true, false]]
        );
        assert_eq!(
            visible_rows("url", &groups()),
            vec![vec![false; 3], vec![false, false, true]]
        );
    }

    #[test]
    fn nothing_matches() {
        let rows = visible_rows("zzz", &groups());
        assert!(rows.iter().flatten().all(|shown| !shown));
    }
}
