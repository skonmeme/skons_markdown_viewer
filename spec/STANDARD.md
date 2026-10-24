# skon's Markdown Viewer — Standard

## 1. About this document

1. This document is the only specification of skon's Markdown Viewer. If any other document conflicts with it, this document is correct.
2. It contains text and figures that help understanding. Supplementary files are kept in `spec/STANDARD/`.
3. It is written in English only, to avoid confusion between languages.
4. It is updated only through specification requests (§11.3).

## 2. Product

| Item | Value |
|---|---|
| Name | skon's Markdown Viewer |
| Bundle file name | `skons_markdown_viewer.app` |
| Bundle identifier | `skons_markdown_viewer` |

"skon" is always written in lowercase, also at the start of a sentence.

## 3. Target environment

- macOS 26 or later.
- Macs with Apple silicon only.

## 4. Basic viewer features

### 4.1 Documents

- The app opens Markdown files with the extensions `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn`, `.mdwn`, `.mdtxt` and `.mdtext`.
- A document can be opened by double-clicking it or using Open With in Finder, by dropping it on the app icon or on a document window, with File > Open…, and with File > Open Recent.
- Opening a document that is already open brings its window to the front.
- Each document is shown in its own window. Windows can be combined into tabs with the standard macOS window tabs.
- The window title shows the file name.
- When the app is started or activated while no document is open, it shows the Open panel.

### 4.2 Rendering

- Markdown syntax: CommonMark with the GitHub Flavored Markdown extensions (tables, task lists, strikethrough, autolinks), footnotes, and GitHub alerts (`> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`, `> [!WARNING]`, `> [!CAUTION]`).
- Every heading can be the target of a link (`#heading`).
- YAML front matter at the start of a document is not shown as normal text; it is shown as a collapsed block that can be expanded.
- Fenced code blocks that name a language are syntax highlighted.
- HTML inside a document is displayed. Scripts in a document never run.
- Images given by a relative path, an absolute path or an `http`/`https` URL are displayed.
- The look of the page follows GitHub's Markdown style.

### 4.3 Links

- A link to an anchor in the same document scrolls to its target.
- A link to another Markdown file opens that file in the app. A link to any other local file opens it with its default application.
- Any other link (`http`, `https`, `mailto`, …) opens with the default application.

### 4.4 Live reload

- When the file of an open document changes on disk, the document is updated automatically and keeps its scroll position. Editors that save by replacing the file are supported.
- If the file is deleted, the last content stays on screen.

### 4.5 Other features

- Zoom: View > Zoom In, Zoom Out and Actual Size change the size of the page between 50 % and 300 %.
- Printing: File > Print… prints the document. Printed pages always use dark text on a white background.
- Text in a document can be selected and copied.
- The page follows the light or dark appearance of macOS unless custom colors are chosen (§8).

## 5. Margins

The page shows the document text between a left and a right margin of the same width (Figure 2).

### 5.1 Automatic margins

Automatic margins are the default. The margin `M` depends on the width `W` of the page, both in points at 100 % zoom:

```
M(W) = max(16, 16 + 0.35 × (W − 560))
```

A narrow page (up to 560 pt wide) keeps a margin of 16 pt. Beyond that, 35 % of the extra width is added to each margin, so a wide window gets relatively generous margins (Figure 1).

| Page width `W` | Margin `M` | Text width |
|---:|---:|---:|
| 480 | 16 | 448 |
| 800 | 100 | 600 |
| 1024 | 178 | 667 |
| 1440 | 324 | 792 |
| 1920 | 492 | 936 |

![Figure 1. Automatic margin as a function of the page width](STANDARD/auto-margin.svg)

### 5.2 Manual margins

- The user can replace the automatic margin with a fixed margin between 0 and 480 pt (§9.1).
- A fixed margin is never larger than 25 % of the page width, so the text always keeps at least half of the width.
- View > Increase Margins (⌥⌘+) and View > Decrease Margins (⌥⌘−) change the margin by 16 pt, starting from the margin currently shown, and switch to a fixed margin. View > Automatic Margins (⌥⌘0) switches back to automatic margins.

## 6. Search

![Figure 2. Document window with the find bar](STANDARD/window-layout.svg)

- Edit > Find > Find… (⌘F) shows the find bar at the top of the window (Figure 2). It contains a search field, Previous and Next buttons, a match counter, a Match Case option and a Done button.
- Searching starts while the user types. The query is matched as literal text, ignoring case unless Match Case is on.
- A match can cover text with different styles inside one block, but never text from different blocks (paragraphs, list items, table cells, …). A space in the query matches any white space.
- All matches are highlighted. The current match is highlighted in a stronger color and scrolled into view. A match inside a collapsed section opens that section.
- The counter shows the position of the current match and the number of matches ("3 of 12"), or "Not found".
- Return and ⌘G go to the next match; Shift-Return and ⇧⌘G go to the previous match. The search wraps around at the end and the start of the document.
- Edit > Find > Use Selection for Find (⌘E) searches for the selected text.
- Esc and Done close the find bar and remove the highlights. The current match becomes the text selection.
- If the document is reloaded (§4.4) while the find bar is open, the search is repeated on the new content.

## 7. Fonts

### 7.1 Latin and Korean fonts

- Korean text and Latin text use separately chosen fonts.
- The Korean font is used for Hangul: the Unicode blocks Hangul Jamo (U+1100–U+11FF), Hangul Compatibility Jamo (U+3130–U+318F), Hangul Jamo Extended-A (U+A960–U+A97F), Hangul Syllables (U+AC00–U+D7AF), Hangul Jamo Extended-B (U+D7B0–U+D7FF) and the halfwidth Hangul forms (U+FFA0–U+FFDC).
- The Latin font is used for English text and for every other character that is not Hangul.
- Code uses a monospaced font. Hangul in code uses the monospaced font if that font contains Hangul, and the Korean font otherwise.

### 7.2 Font sets

- A font set is a named combination of a Latin font, a Korean font, a monospaced font, a font size (8–48 pt) and a line height (1.0–2.5 times the font size).
- The built-in font set "Default" is always available. It uses the system font for Latin text, Apple SD Gothic Neo for Korean text and the system monospaced font for code, with a font size of 16 pt and a line height of 1.6. It cannot be edited, renamed or deleted.
- The user can add a font set (a copy of the active one), edit it, rename it, delete it and choose the active font set.
- Font set names are unique, ignoring case. "Default" is reserved for the built-in font set.
- Each font list offers "System Font" and the installed font families. The Korean list contains only families that include Hangul; the monospaced list contains only fixed-pitch families.

## 8. Colors

- **Automatic** (default): the colors follow the macOS appearance — dark text on a light background in Light mode, light text on a dark background in Dark mode.
- **Custom**: the user chooses the text color and the background color. They are used in both appearances.
- The other colors of the page (links, borders, code backgrounds, syntax highlighting) are derived from the text and background colors so that they stay readable. A background counts as dark when its relative luminance is below 0.18.

## 9. Settings and menus

### 9.1 Settings window

App menu > Settings… (⌘,) opens the Settings window with three groups:

- Fonts: font set, Latin font, Korean font, monospaced font, font size, line height (§7).
- Colors: Automatic or Custom, text color, background color (§8).
- Margins: Automatic or Fixed, fixed margin (§5).

Changes apply immediately to every open document and are kept after the app quits. The zoom level (§4.5) is also shared by all windows and kept.

### 9.2 Menus and keyboard shortcuts

| Menu | Items |
|---|---|
| skon's Markdown Viewer | About skon's Markdown Viewer · Settings… ⌘, · Services · Hide skon's Markdown Viewer ⌘H · Hide Others ⌥⌘H · Show All · Quit skon's Markdown Viewer ⌘Q |
| File | Open… ⌘O · Open Recent · Close ⌘W · Reload ⌘R · Page Setup… ⇧⌘P · Print… ⌘P |
| Edit | Undo ⌘Z · Redo ⇧⌘Z · Cut ⌘X · Copy ⌘C · Paste ⌘V · Select All ⌘A · Find (Find… ⌘F · Find Next ⌘G · Find Previous ⇧⌘G · Use Selection for Find ⌘E) |
| View | Actual Size ⌘0 · Zoom In ⌘+ · Zoom Out ⌘− · Increase Margins ⌥⌘+ · Decrease Margins ⌥⌘− · Automatic Margins ⌥⌘0 · Enter Full Screen ⌃⌘F |
| Window | Minimize ⌘M · Zoom · Bring All to Front |
| Help | skon's Markdown Viewer Help |

Reload (⌘R) reads the file again even when no change was detected.

## 10. Installation and removal

- The app is installed with the installer package `skons_markdown_viewer-<version>.pkg`. It installs `skons_markdown_viewer.app` into `/Applications`, where it is available to all users of the Mac.
- The installer refuses to install on Macs without Apple silicon and on macOS versions earlier than 26.
- The app is removed with the uninstaller package `skons_markdown_viewer-uninstaller-<version>.pkg`. It quits and deletes the app. It also offers to delete the settings and caches of the user who runs it; this option is selected by default.

## 11. Development

### 11.1 Language

The app is written in Rust. Swift may be used together with Rust only where it is really necessary.

### 11.2 Documents

| Path | Content | Language |
|---|---|---|
| `spec/STANDARD.md` | This specification | English |
| `spec/STANDARD/` | Supplementary files of this specification | English |
| `spec/requests/` | Specification requests that have not been applied yet | Any |
| `spec/merged/` | Applied specification requests | Any |
| `README.md` | Concise installation and usage guide | Korean |
| `doc/ARCHITECTURE.md` | Detailed explanation of how the app works, with figures and videos; supplementary files are kept in `doc/ARCHITECTURE/` | Korean |
| `doc/DEVELOPMENT/` | Topics to discuss during development, one file per topic; a file is deleted when its topic is resolved | Korean |

Documents in `doc/` are written in Korean unless there is a particular reason not to.

### 11.3 Specification requests

![Figure 3. Development and release flow](STANDARD/process.svg)

1. This document is updated only through specification requests, which are written in `spec/requests/`.
2. No work is done on the `main` and `develop` branches. Work is always done on the branch where the specification request is committed.
3. While this document is updated, the source code is created or changed so that it matches this document.
4. After this document has been updated, the specification request is moved to `spec/merged/`.
5. If no problem occurs during the process, the changes are committed automatically on the working branch.

### 11.4 Branches and releases

- Development takes place on the `develop` branch.
- When a pull request is merged into `main`, a release is published. It contains the installer and the uninstaller packages (§10).
