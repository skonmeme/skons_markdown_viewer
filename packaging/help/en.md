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
- ⌘E searches for the selected text in the current search mode.
- Esc or **Done** closes the find bar; the current match stays selected.

### Search modes

Type a command at the start of the search field and press Space: the command turns into a rounded token and the search mode changes. Delete right after the token removes it, and the remaining text is searched as keywords. The clear button of the search field removes both the token and the text.

| Command | Search mode | Search starts |
|---|---|---|
| None or `/key` | Keywords: finds the text as you typed it | While you type |
| `/match` | Regular expression, ignoring case | With Return |
| `/case` | Regular expression, matching case | With Return |
| `/ai` | AI: finds the parts related to the meaning, even with other words | With Return |

- **Match Case** can be used only in keyword search.
- Regular expressions use the JavaScript syntax. `^` and `$` match at the start and end of a block, such as a paragraph or a list item. An invalid expression shows "Invalid pattern".

### AI search

- For example, type `/ai`, Space and "how to install", then press Return: the parts about installation are found even when they use other words.
- Each result is a whole block, such as a paragraph or a list item.
- While the search runs, the counter shows "Searching…", and in long documents also the progress, as in "Searching… 2/5". Results are highlighted as they are found. Editing the query or closing the find bar stops the search.
- The default model is Apple Foundation Model on your Mac, so the document never leaves the Mac. It needs Apple Intelligence to be turned on in System Settings. On macOS 26 it is slower and less accurate than on macOS 27.
- You can choose an external LLM in the **AI** settings instead.
- If the counter shows "AI unavailable", point at it to see why.

## Margins

- By default the margins follow the window width: narrow windows get small margins, wide windows get generous ones.
- **View > Increase Margins** (⌥⌘+) and **Decrease Margins** (⌥⌘−) change the margins by 16 points.
- **View > Automatic Margins** (⌥⌘0) goes back to automatic margins.

## Settings

Choose **skon's Markdown Viewer > Settings…** (⌘,). Changes apply to all windows at once and are kept.

All settings are in one list. Click **Fonts**, **Colors**, **Margins** or **AI** in the sidebar to go to that part, or type in the search field of the sidebar to see only the settings that contain the text.

- **Fonts**: a font set combines a Latin font, a Korean font, a monospaced font for code, a font size and a line height. Korean text (Hangul) uses the Korean font and everything else the Latin font. The Default set cannot be changed: click **+** to make an editable copy, **−** to delete a set and **Rename…** to rename it.
- **Colors**: **Automatic** follows the Light or Dark appearance of macOS; **Custom** uses the text and background colors you choose.
- **Margins**: **Automatic**, or **Fixed** with a margin from 0 to 480 points.
- **AI**: the model for AI search.
  - **Apple Foundation Model (on this Mac)**: the default. Whether it can be used now is shown below it.
  - **External LLM**: a server that offers the OpenAI-compatible Chat Completions API. AI search then sends the text of the document and your query to that server, so the app asks before you choose it. The provider may charge for its use.
  - **URL**: the full address of the endpoint, for example `http://localhost:11434/v1/chat/completions` (Ollama running on your Mac) or `https://api.openai.com/v1/chat/completions`.
  - **Model name**: the name of the model on the server.
  - **Model options**: keys and values added to each request. **+** adds a row and **−** removes one. A value that reads as JSON, such as a number, `true` or an array, is sent as that value; any other value is sent as text. Example: `temperature` and `0`.
  - **API key**: only if the server needs one. It is shown as `*` characters; click the eye button to see it. It is kept with the other settings.
  - **Max tokens**: how many tokens the model reads at once. Match it to the context window of the model with the slider (8K–1M) or type it (1,024–2,000,000). The default is 32K.

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
