# skon's Markdown Viewer Help

## Opening documents

- Double-click a Markdown file in Finder, or choose **Open With > skon's Markdown Viewer**.
- Drop a file on the app icon in the Dock or on a document window.
- Choose **File > Open…** (⌘O) or **File > Open Recent**.
- In Terminal, pass the file to the app: `/Applications/MarkdownViewer.app/Contents/MacOS/MarkdownViewer README.md`

Files with the extensions `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn`, `.mdwn`, `.mdtxt` and `.mdtext` can be opened.

When a file changes on disk, its window shows the new content automatically and keeps the scroll position. **File > Reload** (⌘R) reads the file again at any time.

## Reading

- A link to another Markdown file opens it in the app, a web link opens in your browser, and a link to a heading scrolls to it.
- **View > Zoom In** (⌘+), **Zoom Out** (⌘−) and **Actual Size** (⌘0) change the text size of all windows.
- **File > Print…** (⌘P) prints the document in black on white.
- Windows can be grouped as tabs with **Window > Merge All Windows**.

## Searching

- **Edit > Find > Find…** (⌘F) shows the find bar.
- Matches appear while you type. All of them are highlighted; the current one is orange.
- Return or ⌘G goes to the next match, Shift-Return or ⇧⌘G to the previous one.
- Turn on **Match Case** to tell upper and lower case apart.
- ⌘E searches for the selected text.
- Esc or **Done** closes the find bar; the current match stays selected.

## Margins

- By default the margins follow the window width: narrow windows get small margins, wide windows get generous ones.
- **View > Increase Margins** (⌥⌘+) and **Decrease Margins** (⌥⌘−) change the margins by 16 points.
- **View > Automatic Margins** (⌥⌘0) goes back to automatic margins.

## Settings

Choose **skon's Markdown Viewer > Settings…** (⌘,). Changes apply to all windows at once and are kept.

- **Fonts**: a font set combines a Latin font, a Korean font, a monospaced font for code, a font size and a line height. Korean text (Hangul) uses the Korean font and everything else the Latin font. The Default set cannot be changed: click **+** to make an editable copy, **−** to delete a set and **Rename…** to rename it.
- **Colors**: **Automatic** follows the Light or Dark appearance of macOS; **Custom** uses the text and background colors you choose.
- **Margins**: **Automatic**, or **Fixed** with a margin from 0 to 480 points.

## Keyboard shortcuts

| Command | Shortcut |
|---|---|
| Open | ⌘O |
| Close | ⌘W |
| Reload | ⌘R |
| Print | ⌘P |
| Find | ⌘F |
| Find Next / Previous | ⌘G / ⇧⌘G |
| Use Selection for Find | ⌘E |
| Zoom In / Out / Actual Size | ⌘+ / ⌘− / ⌘0 |
| Increase / Decrease / Automatic Margins | ⌥⌘+ / ⌥⌘− / ⌥⌘0 |
| Settings | ⌘, |
| Enter Full Screen | ⌃⌘F |
