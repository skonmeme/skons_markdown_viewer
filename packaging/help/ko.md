# skon's Markdown Viewer 도움말

## 문서 열기

- Finder에서 Markdown 파일을 이중 클릭하거나 **다음으로 열기 > skon's Markdown Viewer**를 고릅니다.
- 파일을 Dock의 앱 아이콘이나 문서 창에 끌어다 놓습니다.
- **File > Open…**(⌘O) 또는 **File > Open Recent**를 고릅니다.
- 터미널에서는 파일을 앱에 넘깁니다: `/Applications/MarkdownViewer.app/Contents/MacOS/MarkdownViewer README.md`

확장자가 `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn`, `.mdwn`, `.mdtxt`, `.mdtext`인 파일을 열 수 있습니다.

파일이 바뀌면 창이 저절로 새 내용을 보여 주고 스크롤 위치는 그대로 유지합니다. 언제든 **File > Reload**(⌘R)로 파일을 다시 읽을 수 있습니다.

## 읽기

- 다른 Markdown 파일로 가는 링크는 이 앱에서, 웹 링크는 브라우저에서 열리고, 제목으로 가는 링크는 그 위치로 스크롤합니다.
- **View > Zoom In**(⌘+), **Zoom Out**(⌘−), **Actual Size**(⌘0)로 모든 창의 글자 크기를 바꿉니다.
- **File > Print…**(⌘P)는 흰 바탕에 검은 글자로 인쇄합니다.
- **Window > Merge All Windows**로 창들을 탭으로 묶을 수 있습니다.

## 검색

- **Edit > Find > Find…**(⌘F)로 검색 막대를 엽니다.
- 입력하는 동안 결과가 나타납니다. 찾은 곳이 모두 강조되고, 현재 위치는 주황색입니다.
- Return이나 ⌘G는 다음, Shift-Return이나 ⇧⌘G는 이전 결과로 갑니다.
- **Match Case**를 켜면 대소문자를 구분합니다.
- ⌘E는 선택한 글자로 검색합니다.
- Esc나 **Done**을 누르면 검색 막대가 닫히고, 현재 결과가 선택된 채로 남습니다.

## 여백

- 기본적으로 여백은 창 폭에 맞춰 바뀝니다. 좁은 창에서는 작고, 넓은 창에서는 넉넉합니다.
- **View > Increase Margins**(⌥⌘+)와 **Decrease Margins**(⌥⌘−)는 여백을 16포인트씩 바꿉니다.
- **View > Automatic Margins**(⌥⌘0)로 자동 여백으로 돌아갑니다.

## 설정

**skon's Markdown Viewer > Settings…**(⌘,)를 고릅니다. 바꾼 내용은 모든 창에 바로 적용되고 저장됩니다.

- **Fonts**: 글꼴 세트는 영문 글꼴(Latin), 한글 글꼴(Korean), 코드용 고정폭 글꼴(Monospace), 글자 크기, 줄 간격의 묶음입니다. 한글에는 한글 글꼴이, 나머지 글자에는 영문 글꼴이 쓰입니다. Default 세트는 바꿀 수 없으니 **+**로 복사본을 만들어 고치세요. **−**는 세트를 지우고 **Rename…**은 이름을 바꿉니다.
- **Colors**: **Automatic**은 macOS의 라이트·다크 모드를 따르고, **Custom**은 직접 고른 글자색과 배경색을 씁니다.
- **Margins**: **Automatic** 또는 **Fixed**(0–480포인트)를 고릅니다.

## 단축키

| 명령 | 단축키 |
|---|---|
| 열기 | ⌘O |
| 닫기 | ⌘W |
| 다시 읽기 | ⌘R |
| 인쇄 | ⌘P |
| 검색 | ⌘F |
| 다음 / 이전 결과 | ⌘G / ⇧⌘G |
| 선택한 글자로 검색 | ⌘E |
| 확대 / 축소 / 실제 크기 | ⌘+ / ⌘− / ⌘0 |
| 여백 늘리기 / 줄이기 / 자동 | ⌥⌘+ / ⌥⌘− / ⌥⌘0 |
| 설정 | ⌘, |
| 전체 화면 | ⌃⌘F |
