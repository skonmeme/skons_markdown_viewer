//! Search modes and the mode tokens of the find bar (STANDARD §6.2, §6.3).

/// A search mode (STANDARD §6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Keyword,
    /// Regular expression that ignores case (`/match`).
    RegexIgnoringCase,
    /// Regular expression that matches case (`/case`).
    RegexMatchingCase,
    Ai,
}

impl Mode {
    pub const ALL: [Mode; 4] = [
        Mode::Keyword,
        Mode::RegexIgnoringCase,
        Mode::RegexMatchingCase,
        Mode::Ai,
    ];

    /// The command without its slash, which is also the label of the token.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Keyword => "key",
            Mode::RegexIgnoringCase => "match",
            Mode::RegexMatchingCase => "case",
            Mode::Ai => "ai",
        }
    }

    /// Whether the search waits for Return instead of running while the
    /// user types.
    pub fn starts_with_return(self) -> bool {
        self != Mode::Keyword
    }

    pub fn is_regex(self) -> bool {
        matches!(self, Mode::RegexIgnoringCase | Mode::RegexMatchingCase)
    }
}

/// A command that the user has just finished typing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    pub mode: Mode,
    /// Length of the command and its space in UTF-16 code units.
    pub length: usize,
}

/// Recognizes a command typed at the start of a search field that has no
/// token yet: the field starts with `/name` and the space after it was just
/// typed, so the insertion point `caret` (in UTF-16 code units) is right
/// after that space. Names are recognized regardless of case.
pub fn typed_command(text: &str, caret: usize) -> Option<Command> {
    let rest = text.strip_prefix('/')?;
    let space = rest.find(' ')?;
    let name = &rest[..space];
    let length = name.encode_utf16().count() + 2;
    if caret != length {
        return None;
    }
    let mode = Mode::ALL
        .into_iter()
        .find(|mode| mode.name().eq_ignore_ascii_case(name))?;
    Some(Command { mode, length })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_of_the_standard() {
        let ai = typed_command("/ai ", 4);
        assert_eq!(
            ai,
            Some(Command {
                mode: Mode::Ai,
                length: 4
            })
        );
        assert_eq!(
            typed_command("/key ", 5).map(|c| c.mode),
            Some(Mode::Keyword)
        );
        assert_eq!(
            typed_command("/match ", 7).map(|c| c.mode),
            Some(Mode::RegexIgnoringCase)
        );
        assert_eq!(
            typed_command("/case ", 6).map(|c| c.mode),
            Some(Mode::RegexMatchingCase)
        );
    }

    #[test]
    fn names_ignore_case() {
        assert_eq!(typed_command("/AI ", 4).map(|c| c.mode), Some(Mode::Ai));
        assert_eq!(
            typed_command("/Match ", 7).map(|c| c.mode),
            Some(Mode::RegexIgnoringCase)
        );
    }

    #[test]
    fn text_after_the_command_is_kept() {
        // The command was typed in front of existing text.
        let command = typed_command("/ai install steps", 4).unwrap();
        assert_eq!(&"/ai install steps"[command.length..], "install steps");
    }

    #[test]
    fn only_a_just_typed_command_at_the_start() {
        assert_eq!(typed_command("/ai", 3), None);
        assert_eq!(typed_command("/ai x", 5), None);
        assert_eq!(typed_command(" /ai ", 5), None);
        assert_eq!(typed_command("x /ai ", 6), None);
        assert_eq!(typed_command("/foo ", 5), None);
        assert_eq!(typed_command("/ ", 2), None);
        assert_eq!(typed_command("ai ", 3), None);
    }

    #[test]
    fn only_keyword_search_runs_while_typing() {
        assert!(!Mode::Keyword.starts_with_return());
        assert!(Mode::RegexIgnoringCase.starts_with_return());
        assert!(Mode::RegexMatchingCase.starts_with_return());
        assert!(Mode::Ai.starts_with_return());
    }
}
