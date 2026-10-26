# skon's Markdown Viewer — Standard

## 1. About this document

1. This document is the only specification of skon's Markdown Viewer. If any other document conflicts with it, this document is correct.
2. It contains text and figures that help understanding. Supplementary files are kept in `spec/STANDARD/`.
3. It is written in English only, to avoid confusion between languages.
4. It is updated only through specification requests (§11.3).

## 2. Product

| Item | Value |
|---|---|
| Display name | skon's Markdown Viewer |
| App name | MarkdownViewer |
| Bundle file name | `MarkdownViewer.app` |
| Bundle identifier | `io.github.skonmeme.markdownviewer` |

- "skon" is always written in lowercase, also at the start of a sentence.
- The display name is shown wherever macOS shows the app to users: Finder, Launchpad, the Dock, the menu bar, the About panel and the installer.
- The app name is used for the bundle file and the package files (§10).

### 2.1 App icon

The app icon is an Icon Composer document (`.icon`) in the Liquid Glass icon format of macOS 26 (Figure 1). On its 1024 pt canvas it has four parts:

- **Background**: light gray (automatic gradient `extended-srgb:0.95,0.95,0.97`).
- **Badge**: a rectangle with a semicircle on each end, 880 × 440 pt, centered. It is colorless clear glass (white fill with alpha 0.05, translucency 0.92) with Icon Composer refraction of strength 0.15 and depth 0.1.
- **Mark**: the Markdown mark M↓ without its frame, 520 pt wide, centered on the badge. It is an opaque layer, not glass, and is black in the light appearance and white in the dark appearance.
- **Lettering**: "skon's" in lowercase round strokes drawn for the icon (not set in a font), with a stroke width of 26 pt, deep blue `#0033aa` at 50 % opacity, rising 15° to the right. It is about two thirds of the badge size and placed at the upper left, so that its lower part lies on the upper rim of the badge and is bent by the refraction.

![Figure 1. App icon in the light and dark appearance](STANDARD/app-icon.png)

## 3. Target environment

- macOS 26 or later.
- Macs with Apple silicon only.

## 4. Basic viewer features

### 4.1 Documents

- The app opens Markdown files with the extensions `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn`, `.mdwn`, `.mdtxt` and `.mdtext`.
- A document can be opened by double-clicking it or using Open With in Finder, by dropping it on the app icon or on a document window, with File > Open…, with File > Open Recent, and by passing its path as a command-line argument to the app's executable.
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

The page shows the document text between a left and a right margin of the same width (Figure 3).

### 5.1 Automatic margins

Automatic margins are the default. The margin `M` on each side depends on the width `W` of the page, both in points at 100 % zoom. Points are the unit in which macOS measures the screen, not the pixels of the display: on a display used at 2×, a width of 3840 pixels is 1920 pt.

```
M(W) = 8                               W ≤ 480
M(W) = 8 + 136 × (W − 480) / 1440      480 ≤ W ≤ 1920
M(W) = 0.075 × W                       W ≥ 1920
```

As a single formula, `M(W) = max(8, min(8 + 136 × (W − 480) / 1440, 0.075 × W))`.

A narrow page (up to 480 pt wide) keeps a margin of 8 pt on each side. From there the margin grows linearly with the width until, at 1920 pt, both margins together take 15 % of the width (144 pt each). On wider pages both margins together keep taking 15 % of the width (Figure 2).

| Page width `W` | Margin `M` | Both margins / `W` | Text width |
|---:|---:|---:|---:|
| 480 | 8 | 3.3 % | 464 |
| 800 | 38 | 9.6 % | 724 |
| 1024 | 59 | 11.6 % | 905 |
| 1440 | 99 | 13.7 % | 1243 |
| 1920 | 144 | 15 % | 1632 |
| 2560 | 192 | 15 % | 2176 |

![Figure 2. Automatic margin as a function of the page width](STANDARD/auto-margin.svg)

### 5.2 Manual margins

- The user can replace the automatic margin with a fixed margin between 0 and 480 pt (§9.1).
- A fixed margin is never larger than 25 % of the page width, so the text always keeps at least half of the width.
- View > Increase Margins (⌥⌘+) and View > Decrease Margins (⌥⌘−) change the margin by 16 pt, starting from the margin currently shown, and switch to a fixed margin. View > Automatic Margins (⌥⌘0) switches back to automatic margins.

## 6. Search

![Figure 3. Document window with the find bar](STANDARD/window-layout.svg)

- Edit > Find > Find… (⌘F) shows the find bar at the top of the window (Figure 3). It contains a search field, Previous and Next buttons, a match counter, a Match Case option and a Done button.
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

Help > skon's Markdown Viewer Help opens the usage guide in a viewer window. The usage guide is a Markdown document bundled with the app that explains only how to use the app. It exists in Korean and in English; the version matching the system language is shown, and English is shown for any other system language.

## 10. Installation and removal

- The app is installed with the installer package `MarkdownViewer-<version>.pkg`. It installs `MarkdownViewer.app` into `/Applications`, where it is available to all users of the Mac.
- The installer refuses to install on Macs without Apple silicon and on macOS versions earlier than 26.
- The app is removed with the uninstaller package `MarkdownViewer-uninstaller-<version>.pkg`. It quits and deletes the app. It also offers to delete the settings and caches of the user who runs it; this option is selected by default.
- The app is signed with a Developer ID Application certificate and the hardened runtime. Both packages are signed with a Developer ID Installer certificate, notarized by Apple and stapled. Unsigned or unnotarized packages are never released.

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
| `CLAUDE.md` | Instructions that Claude reads at the start of every session; it repeats §11.3 so that Claude always follows it | English |
| `README.md` | Concise installation and usage guide | Korean |
| `doc/ARCHITECTURE.md` | Detailed explanation of how the app works, with figures and videos; supplementary files are kept in `doc/ARCHITECTURE/` | Korean |
| `doc/DEVELOPMENT/` | Topics to discuss during development, one file per topic; a file is deleted when its topic is resolved | Korean |

Documents in `doc/` are written in Korean unless there is a particular reason not to.

### 11.3 Specification requests

![Figure 4. Development and release flow](STANDARD/process.svg)

This document is changed only through specification requests. The requester asks for changes and makes the decisions; Claude carries out the four steps below (Figure 4). `CLAUDE.md` repeats this section so that Claude always knows it (§11.2); if the two differ, this document is correct. No work is done on the `main` and `develop` branches; all work for a request is done on the branch that holds the request.

**Step 1. Managing a specification request**

1. When the requester asks for a request, Claude creates `spec/requests/NNN-title.md` on the branch that is currently checked out. `NNN` is the next three-digit number across `spec/requests/` and `spec/merged/`, and `title` is a short English name.
2. Claude never creates branches. If the current branch is `main` or `develop`, Claude does not create the request and asks the requester to change the branch.
3. A branch may hold several requests when needed.
4. A request has these sections:

| Section | Content |
|---|---|
| Request | The requester's original request, verbatim |
| Decisions | For each decision: its content (concrete values, formulas, behavior), its source (the requester's words, verbatim) and its reason. A proposal by Claude that the requester has not explicitly approved is marked as such. |
| Resulting specification | What this document will contain as a result of the decisions |
| Impact | Changes visible to users, and the sections and figures of this document, source code, tests and documents that change |
| Open questions | Points that have not been decided yet |

5. Whenever a decision is made or changed in the conversation, Claude updates the request at once. Decisions, calculated results and proposals are never kept only in the conversation. When a decision changes, the earlier statement stays and the new one is added.
6. A request is committed only when the requester says so, or at the start of step 2.

**Step 2. Updating this document**

1. Claude updates this document only when the requester explicitly instructs it. If the request still has open questions, Claude asks the requester first.
2. If the request has uncommitted changes, Claude commits the request first. The instruction to update this document also counts as the instruction to commit the request.
3. Every decision of the request is reflected, and nothing unrelated to the decisions is changed.
4. This document is written in English, and its figures in `spec/STANDARD/` are changed to match.
5. If a decision conflicts with another part of this document, Claude stops and asks the requester.

**Step 3. Implementation and follow-up work**

1. Step 3 may start right after the update of this document has been committed. Steps 2 and 3 are never done at the same time.
2. The source code and tests are always made to match this document. If the implementation turns out to need a change of this document, Claude asks the requester and continues only after the answer.
3. `cargo test --workspace` must pass.
4. The documents are brought in line with this document: `README.md`, `doc/ARCHITECTURE.md` and `doc/ARCHITECTURE/`, `doc/DEVELOPMENT/` (a topic to discuss is added, and a resolved topic is deleted), the Help (`packaging/help/`), and `CLAUDE.md` when this section changes.
5. When everything is done, the request is moved to `spec/merged/`.

**Step 4. Commits**

1. A request is committed only as described in step 1.
2. The results of steps 2 and 3 are committed automatically on the working branch if no problem occurs. The commit of step 2 is made before step 3 starts. If the build or a test fails, or a conflict within this document appears, nothing is committed and the requester is told.
3. Commits are made only on the working branch. Pushing, merging into `develop` and pull requests to `main` happen only when the requester instructs them (§11.4).
4. Commit messages have this form:

| Work | Commit message |
|---|---|
| Adding a request | `spec: add request NNN-title` |
| Updating a request | `spec: <change> to request NNN-title`, for example `spec: add decisions to request 002-package` |
| Updating this document | `spec: update STANDARD.md from request NNN-title` |
| Implementation | `Implement STANDARD from request NNN-title` |
| Documents | `docs: <change>` |
| Moving a request | `spec: move request NNN-title to merged` |

### 11.4 Branches and releases

- Development takes place on the `develop` branch.
- Before a release, the packages are tested locally:
  1. Signed and notarized installer and uninstaller packages are built on the developer's Mac. Signing uses the Developer ID certificates in the keychain; notarization uses notarytool credentials stored in the keychain.
  2. The installer package installs the app into `/Applications`, where its features are checked and Gatekeeper must accept it.
  3. The uninstaller package must remove it completely.
- When a pull request is merged into `main`, the GitHub workflow builds, signs and notarizes the app and both packages and publishes a release that contains the two packages (§10). It uses the same certificates as the local build, read from these repository secrets:

| Secret | Content |
|---|---|
| `DEVELOPER_ID_P12` | Base64 of a `.p12` file with the Developer ID Application and Developer ID Installer identities |
| `DEVELOPER_ID_P12_PASSWORD` | Password of that `.p12` file |
| `NOTARY_ISSUER_ID` | Issuer ID of the App Store Connect API key used for notarization |
| `NOTARY_KEY_ID` | Key ID of that API key |
| `NOTARY_KEY` | Private key (`.p8` contents) of that API key |
