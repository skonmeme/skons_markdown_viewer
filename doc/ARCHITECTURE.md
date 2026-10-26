# skon's Markdown Viewer 아키텍처

이 문서는 앱이 어떻게 동작하는지 설명합니다. 무엇을 해야 하는지는 [명세(spec/STANDARD.md)](../spec/STANDARD.md)가 정하고, 이 문서는 그것을 어떻게 구현했는지를 다룹니다. 두 문서가 다르면 명세가 맞습니다.

## 1. 한눈에 보기

- **언어**: Rust만 씁니다. AppKit·WebKit은 [objc2](https://github.com/madsmtm/objc2) 바인딩으로 직접 호출하고, Swift나 Objective-C 코드는 없습니다.
- **앱 구조**: 문서 기반 AppKit 앱입니다. Markdown 파일 하나가 `NSDocument` 하나이고, 창마다 `WKWebView`가 하나씩 있습니다.
- **렌더링**: Markdown → HTML 변환([comrak](https://github.com/kivikakk/comrak))과 코드 하이라이트([syntect](https://github.com/trishume/syntect))는 Rust에서 하고, 화면 표시는 WebKit이 합니다.
- **설정 반영**: 글꼴·색·여백 설정은 CSS로 바꿔 페이지에 넣습니다.

![구성 요소](ARCHITECTURE/components.svg)

## 2. 저장소 구조

```
Cargo.toml                 워크스페이스 (버전은 여기서 관리)
.cargo/config.toml         대상 aarch64-apple-darwin, MACOSX_DEPLOYMENT_TARGET=26.0, `cargo xtask` 별칭
crates/core/               smv-core: macOS API를 쓰지 않는 부분 (단위 테스트 대상)
  src/markdown.rs          Markdown → HTML (comrak 옵션)
  src/highlight.rs         코드 블록 하이라이트 (syntect + two-face 문법 모음)
  src/style.rs             설정 → 사용자 스타일시트(CSS)
  src/settings.rs          설정 모델과 규칙 (글꼴 세트, 색, 여백, 확대)
  src/page.rs              페이지 HTML 틀
  src/url.rs               파일 경로 ↔ smv: URL, HTTP Range 해석
  src/text.rs              UTF-8/UTF-16 해석
  assets/base.css          기본 스타일시트 (GitHub 스타일)
  assets/viewer.js         페이지 안에서 도는 유일한 스크립트
crates/app/                markdown-viewer: macOS 앱 (실행 파일 MarkdownViewer)
xtask/                     빌드 도구: 앱 번들, pkg, 공증
packaging/                 Info.plist, 아이콘(AppIcon.icon), Help 문서, 설치·제거 pkg 자료
.github/workflows/         릴리스 워크플로
spec/                      명세와 명세 요청서
doc/                       이 문서와 논의 사항
```

## 3. 시작과 문서 열기

1. `main.rs`는 문서 클래스 `SMVDocument`를 Objective-C 런타임에 먼저 등록합니다. `NSDocumentController`가 Info.plist의 `NSDocumentClass` 이름으로 클래스를 찾기 때문입니다. 그다음 앱 델리게이트를 붙이고 실행합니다.
2. `applicationWillFinishLaunching`에서 메뉴를 만들고, 명령줄 인자로 받은 파일을 엽니다.
3. 열린 문서가 없으면 AppKit이 `applicationOpenUntitledFile:`을 부르고, 앱은 새 문서 대신 열기 패널을 보여 줍니다. Dock 아이콘을 눌렀는데 창이 없을 때도 같습니다.
4. 파일 열기, 최근 문서, 이미 열린 문서 앞으로 가져오기, 탭, 창 상태 복원은 `NSDocumentController`와 `NSDocument`가 처리합니다. 최근 문서 메뉴만은 `menu.rs`가 `recentDocumentURLs`로 직접 채웁니다.

![문서를 열 때의 흐름](ARCHITECTURE/render-flow.svg)

### 문서 (`document.rs`)

- `readFromData:ofType:error:`에서 바이트를 텍스트로 바꿉니다(BOM이 있는 UTF-8/UTF-16, 그 밖에는 UTF-8).
- `makeWindowControllers`에서 창(`ViewerController`)을 만들고 파일 감시를 시작합니다.
- 파일이 옮겨지거나 이름이 바뀌면 AppKit이 `setFileURL:`을 부릅니다. 앱은 이것을 가로채 감시 대상과 페이지 주소를 새 경로로 바꿉니다.
- 인쇄는 `printOperationWithSettings:error:`에서 웹 뷰의 인쇄 작업을 돌려줍니다. 인쇄용 CSS가 흰 바탕·검은 글자를 강제하고, 코드 블록 배경은 인쇄되도록 켜 두었습니다.

### 창 (`viewer.rs`)

`ViewerController`(`NSWindowController` 하위 클래스)가 창 하나를 맡습니다.

- 창 위쪽에는 검색 막대(`find_bar.rs`)가, 그 아래에는 `WKWebView`가 있습니다.
- 첫 창은 저장된 위치·크기로 열리고, 그 뒤 창은 계단식으로 엇갈려 열립니다.
- 웹 뷰는 다음처럼 설정합니다.
  - `smv` URL 스킴 처리기를 연결합니다.
  - 문서 안의 JavaScript는 끕니다(`allowsContentJavaScript = false`).
  - `viewer.js`는 앱 전용의 격리된 스크립트 영역(content world "smv")에 문서 로드 끝에 넣습니다.
- 페이지가 이미 떠 있는 상태에서 내용이나 설정이 바뀌면, `callAsyncJavaScript`로 그 영역의 함수(`smv.update`, `smv.setStyle` 등)를 부릅니다. 페이지가 아직 로드 중이면, 내용·스타일 버전 번호를 비교해 로드가 끝난 뒤 반영합니다.

## 4. `smv:` URL 스킴 (`scheme.rs`)

WebKit의 웹 콘텐츠 프로세스는 샌드박스 안에서 돌아서 로컬 파일을 마음대로 읽을 수 없습니다. 그래서 앱이 `smv:` 스킴으로 파일을 직접 건네줍니다.

- 파일 `/Users/me/notes/a b.md`의 주소는 `smv://local/Users/me/notes/a%20b.md`입니다.
- 문서 자신의 경로를 요청하면, 그 자리에서 렌더링한 전체 페이지 HTML을 돌려줍니다.
- 다른 경로를 요청하면 파일 내용을 돌려줍니다. 문서 안의 상대 경로(`images/a.png`)가 파일 시스템과 똑같이 해석되므로 이미지가 그대로 보입니다.
- MIME 타입은 확장자로 정합니다(UTType). 동영상 재생에 필요한 HTTP Range 요청(206, 416)도 처리합니다.
- Markdown의 `file://` 이미지 주소는 렌더링할 때 `smv://local/…`로 바꿉니다.

### 링크

| 링크 대상 | 동작 |
|---|---|
| 같은 문서의 `#앵커` | 웹 뷰가 그대로 스크롤 |
| 다른 Markdown 파일 | `NSDocumentController`로 앱에서 열기. `#앵커`가 있으면 열린 뒤 그 위치로 스크롤 |
| 다른 로컬 파일·폴더 | 기본 앱(Finder 등)으로 열기 |
| `http`, `https`, `mailto` 등 | 사용자가 누른 경우에만 기본 앱으로 열기 |
| `javascript:`, `data:`, `blob:` | 무시 |

창에 파일을 끌어다 놓으면 웹 뷰가 `file://` 주소로 이동하려 합니다. 앱은 이것을 가로채 그 파일을 문서로 엽니다.

## 5. 렌더링 (`smv-core`)

- **Markdown** (`markdown.rs`): comrak으로 CommonMark + GFM(표, 작업 목록, 취소선, 자동 링크), 각주, GitHub 알림을 처리합니다.
  - 제목에는 접두어 없는 `id`가 붙어 `#제목` 링크가 동작합니다.
  - HTML은 그대로 출력하지만, 페이지의 JavaScript가 꺼져 있어 스크립트는 실행되지 않습니다.
  - YAML front matter는 본문에서 빼서 접힌 `<details>` 블록으로 맨 위에 보여 줍니다.
- **코드 하이라이트** (`highlight.rs`): syntect가 토큰마다 `sy-` 접두어 CSS 클래스를 붙입니다. 색은 스타일시트가 정하므로, 라이트·다크 전환이 CSS만으로 됩니다. 256 KiB를 넘는 코드 블록은 하이라이트하지 않습니다.
- **페이지** (`page.rs`): 기본 스타일시트(`base.css`)와 사용자 스타일시트를 `<style>` 두 개로 넣고, 본문을 `<article id="smv-content">`에 넣습니다.

## 6. 실시간 갱신

![실시간 갱신](ARCHITECTURE/live-reload.svg)

1. `watcher.rs`가 문서가 있는 **폴더**를 FSEvents([notify](https://github.com/notify-rs/notify))로 감시합니다. 편집기가 임시 파일에 쓰고 원본을 바꿔치기하는 원자적 저장도 이렇게 잡힙니다.
2. 감시 스레드의 이벤트는 메인 큐로 넘어오고, 100 ms 안에 들어온 이벤트는 한 번으로 묶입니다.
3. 파일의 수정 시각과 크기가 바뀌었을 때만 다시 읽고, 텍스트가 실제로 달라졌을 때만 다시 렌더링합니다.
4. 새 HTML을 `smv.update(html)`로 넘깁니다. `viewer.js`는 기존 본문과 새 본문의 최상위 노드를 앞뒤에서부터 비교해, 바뀐 블록만 교체합니다. 그래서 바뀌지 않은 이미지는 다시 불러오지 않고, 스크롤 위치도 유지됩니다.
5. 파일이 지워지면 마지막 내용을 그대로 보여 줍니다.

## 7. 검색

- **검색 막대** (`find_bar.rs`): 검색 필드, 이전·다음 버튼, 결과 표시, Match Case, Done으로 이루어집니다. 배경은 시스템 재질(`NSVisualEffectView`)이라, 사용자 지정 배경색에서도 글자가 잘 보입니다.
- **키 처리**: 검색 필드의 델리게이트가 Return(다음), Shift-Return(이전), Esc(닫기)를 처리합니다.
- **실제 검색**은 `viewer.js`가 합니다.
  1. 본문의 텍스트 노드를 이어 붙이되, 블록(문단, 목록 항목, 표 칸 등)이 바뀌는 곳에 구분 문자를 넣어 블록을 넘는 일치를 막습니다.
  2. 대소문자 무시는 코드 포인트 단위로 소문자로 바꾸되 길이가 변하는 문자는 그대로 둡니다. 공백 문자는 모두 공백으로 맞춥니다. 덕분에 원문과 비교용 문자열의 위치가 1:1로 대응합니다.
  3. 찾은 위치를 `Range`로 만들어 CSS Custom Highlight API(`CSS.highlights`)에 등록합니다. 전체 결과와 현재 결과에 다른 색을 줍니다.
  4. 현재 결과가 접힌 `<details>` 안에 있으면 펼친 뒤, 화면 안으로 스크롤합니다.
- 검색을 마치면(Esc/Done) 현재 결과가 텍스트 선택으로 남습니다.

## 8. 글꼴

![글꼴 적용](ARCHITECTURE/fonts.svg)

- 글꼴 세트는 영문(Latin)·한글(Korean)·고정폭(Monospace) 글꼴, 크기, 줄 간격의 묶음입니다(`settings.rs`). 내장 Default 세트는 시스템 글꼴, Apple SD Gothic Neo, 시스템 고정폭 글꼴, 16 pt, 1.6입니다.
- 한글에만 한글 글꼴을 쓰려고 CSS `unicode-range`를 씁니다.
  1. `fonts.rs`가 `NSFontManager`로 한글 글꼴 패밀리의 모든 글꼴(PostScript 이름, 굵기, 기울임)을 찾습니다.
  2. `style.rs`가 글꼴마다 `@font-face { font-family: "smv-korean"; src: local(...); unicode-range: 한글 범위 }`를 만듭니다.
  3. 본문 글꼴 목록은 `"smv-korean", 영문 글꼴, 한글 글꼴, sans-serif` 순서입니다. 한글은 맨 앞의 `smv-korean`이 맡고, 나머지 글자는 `smv-korean`의 범위 밖이라 영문 글꼴로 넘어갑니다.
- 코드에는 고정폭 글꼴을 먼저 씁니다. 그래서 고정폭 글꼴에 한글이 있으면 그 글꼴로, 없으면 한글 글꼴로 표시됩니다.
- 설정 창의 한글 목록에는 '가'(U+AC00)를 가진 패밀리만, 고정폭 목록에는 고정폭 특성을 가진 패밀리만 나옵니다.

## 9. 색상과 여백

**색상**
- 자동 모드는 CSS `light-dark()`로 macOS 라이트·다크 모드를 따릅니다.
- 사용자 지정 모드는 배경색의 상대 휘도가 0.18 미만이면 어두운 배경으로 보고, `color-scheme`, 링크 색, 코드 테마를 그에 맞춥니다.
- 테두리·코드 배경 같은 나머지 색은 `color-mix()`로 글자색과 배경색에서 만듭니다.
- 코드 하이라이트 테마는 라이트(GitHub)와 다크(OneHalfDark) 중 하나만 적용되도록 미디어 쿼리로 나눕니다. 인쇄할 때는 항상 라이트 테마입니다.

**여백**
- 자동 여백은 CSS 계산식 `max(8px, min(8px + (100vw - 480px) * 136 / 1440, 7.5vw))`입니다. 창 크기를 바꾸는 동안에도 WebKit이 바로 다시 계산합니다.
- 고정 여백은 `min(값px, 25vw)`로, 페이지 폭의 25 %를 넘지 않습니다.
- 메뉴의 여백 늘리기·줄이기는 현재 보이는 여백(점 단위로 반올림)에서 16 pt씩 더하거나 뺍니다.

![자동 여백](ARCHITECTURE/auto-margin.gif)

같은 내용의 동영상: [auto-margin.mp4](ARCHITECTURE/auto-margin.mp4)

## 10. 설정

- **저장** (`settings_store.rs`): 설정은 `NSUserDefaults`의 `Settings` 키에 JSON으로 저장됩니다.
- **변경 처리**: 값이 바뀌면 다음 순서로 진행합니다.
  1. 범위를 정규화합니다.
  2. 저장합니다.
  3. 열린 모든 창에 새 스타일시트를 보내고, 확대 배율과 창 배경색을 적용합니다.
  4. 설정 창이 열려 있으면 그 화면도 새로 고칩니다.
- **설정 창** (`settings_window.rs`): `NSGridView`로 배치합니다. Default 세트가 선택되면 글꼴 관련 컨트롤을 끄고 안내 문구를 보여 줍니다.

## 11. Help

- **문서 위치**: Help 메뉴의 사용법 문서는 `packaging/help/en.md`와 `ko.md`이고, 앱 안의 `Contents/Resources/Help/`에 들어갑니다.
- **언어 선택** (`help.rs`): 시스템의 첫 번째 선호 언어가 한국어면 `ko.md`, 그 밖에는 `en.md`를 고릅니다.
- **여는 방법**: `makeDocumentWithContentsOfURL:ofType:error:`로 문서를 직접 만들어 엽니다. `openDocument…`를 거치지 않으므로 최근 문서 목록에 남지 않습니다.
- **창 제목**과 재사용: 제목은 "skon's Markdown Viewer Help"이고, 이미 열려 있으면 그 창을 앞으로 가져옵니다.

## 12. 앱 아이콘

- 아이콘 원본은 Icon Composer 문서 `packaging/AppIcon.icon`입니다. `icon.json`과 SVG 레이어 세 개(배지, M↓, "skon's")로 이루어집니다.
- `cargo xtask bundle`이 `actool`로 이것을 컴파일합니다. macOS 26의 Liquid Glass 아이콘(`Assets.car`)과 이전 방식의 `AppIcon.icns`가 만들어지고, Info.plist에 `CFBundleIconName`·`CFBundleIconFile`이 들어갑니다.

시안을 만들며 알게 된 Icon Composer의 동작입니다.

- 굴절(`refractivity`)은 그룹 속성이고, 유리 그룹이 하나일 때만 적용됐습니다. 그래서 배지와 M↓를 한 그룹에 넣고, M↓는 유리가 아닌 레이어로 두었습니다.
- 흰색 유리에 투명도만 높이면 우윳빛이 됩니다. 맑은 유리는 모양을 꽉 채운 채 레이어 `fill`의 알파만 낮춰(0.05) 만들었습니다.
- 다크 모드에서 M↓를 흰색으로 바꾸는 것은 `fill-specializations`(`appearance: dark`)로 했습니다.

## 13. 빌드와 패키징 (`xtask`)

| 명령 | 하는 일 |
|---|---|
| `cargo xtask bundle [--release]` | 실행 파일을 빌드해 `target/aarch64-apple-darwin/<프로필>/bundle/MarkdownViewer.app`을 만들고 서명 |
| `cargo xtask run [--release] [파일…]` | 번들을 만든 뒤 앱 실행 |
| `cargo xtask dist` | 릴리스 번들과 `target/dist/MarkdownViewer-<버전>.pkg`, `MarkdownViewer-uninstaller-<버전>.pkg` 생성 |
| `cargo xtask notarize` | `target/dist`의 pkg를 공증하고 스테이플 |
| `cargo xtask version` | 버전 출력 |

**번들**
- `packaging/Info.plist` 틀에 버전을 넣고 아이콘을 컴파일합니다. 이어서 `en.lproj/InfoPlist.strings`와 Help 문서를 복사합니다.
- 파일 이름은 `MarkdownViewer.app`입니다. Finder·Dock·메뉴 막대에 보이는 이름 "skon's Markdown Viewer"는 `en.lproj/InfoPlist.strings`의 현지화된 표시 이름(`LSHasLocalizedDisplayName`)으로 정합니다.

**설치 pkg**
- 앱 하나를 담은 구성 요소 pkg를 배포용 pkg로 감쌉니다.
- 구성 요소 pkg는 재배치를 꺼서(`BundleIsRelocatable = false`) 항상 `/Applications`에 설치됩니다.
- 배포 설정(`distribution.xml`)이 Apple Silicon(`hostArchitectures="arm64"`)과 macOS 26 이상인지 검사합니다.

**제거 pkg**
- 스크립트만 있는 pkg 두 개로 이루어집니다.
  - `app`: 앱 종료, LaunchServices 등록 해제, 삭제, 설치 기록 제거
  - `userdata`: 실행한 사용자의 설정, 캐시, WebKit 데이터, 창 상태, 최근 문서 목록 삭제. 설치 과정의 사용자화에서 끌 수 있습니다.
- 알아 둘 점: macOS 13부터 빌드한 파일에는 `com.apple.provenance` 확장 속성이 붙고 지울 수 없습니다. 그래서 pkg 페이로드에 `._` 항목이 들어가지만, 설치할 때 속성으로 복원될 뿐 동작에는 영향이 없습니다.

## 14. 서명, 공증, 릴리스

![릴리스 흐름](ARCHITECTURE/release.svg)

### 로컬 테스트 (배포 전)

공증 자격 증명은 처음 한 번 키체인에 저장합니다. 프로필 이름은 `skon's Universe`입니다(키체인 항목 `com.apple.gke.notary.tool.saved-creds.skon's Universe`).

```sh
xcrun notarytool store-credentials "skon's Universe" \
  --key <AuthKey_XXXX.p8> --key-id <Key ID> --issuer <Issuer ID>
```

서명·공증된 pkg를 만들어 설치해 봅니다.

```sh
export SMV_SIGN_APP_IDENTITY="Developer ID Application: <이름> (<팀 ID>)"
export SMV_SIGN_PKG_IDENTITY="Developer ID Installer: <이름> (<팀 ID>)"
export SMV_NOTARY_PROFILE="skon's Universe"
cargo xtask dist && cargo xtask notarize
spctl --assess --type install --verbose target/dist/MarkdownViewer-*.pkg
```

1. `MarkdownViewer-<버전>.pkg`로 설치하고, 앱 기능과 Gatekeeper 통과를 확인합니다.
2. `MarkdownViewer-uninstaller-<버전>.pkg`로 깨끗이 지워지는지 확인합니다.

처음 서명할 때 키체인 접근 확인 창이 뜨면 "항상 허용"을 누릅니다.

### GitHub 워크플로 (`.github/workflows/release.yml`)

main으로 PR이 머지되면 다음 순서로 진행합니다.

1. `v<버전>` 릴리스가 이미 있으면 멈춥니다. 버전은 `Cargo.toml`의 `[workspace.package] version`입니다.
2. `DEVELOPER_ID_P12`를 임시 키체인에 넣고, 그 안에서 Developer ID Application·Installer 인증서 이름을 찾습니다.
3. `NOTARY_KEY`, `NOTARY_KEY_ID`, `NOTARY_ISSUER_ID`로 공증 키를 준비합니다.
4. 다음을 실행합니다.
   - `cargo test`
   - `cargo xtask dist`
   - `cargo xtask notarize`
   - `pkgutil --check-signature`
   - `spctl --assess`
5. `gh release create`로 두 pkg를 올립니다.

Secrets가 하나라도 없으면 처음 단계에서 실패합니다. 서명·공증되지 않은 pkg는 배포하지 않습니다.

## 15. 개발 절차와 테스트

- 명세 변경은 명세 요청서로만 합니다. 절차는 명세 §11.3이 정하며, Claude가 늘 따르도록 `CLAUDE.md`에도 같은 내용이 있습니다.
  1. 요청서 관리: 요청이 있으면 `spec/requests/`에 요청서를 만들고, 결정이 나올 때마다 내용·근거·영향을 적습니다. 요청서는 요청자가 지시할 때만 커밋합니다.
  2. STANDARD 갱신: 요청자가 명확히 지시할 때만 합니다. 커밋하지 않은 요청서가 있으면 먼저 커밋합니다.
  3. 구현 및 후속 작업: STANDARD 갱신을 커밋한 뒤 코드·테스트·문서를 STANDARD에 맞추고, 요청서를 `spec/merged/`로 옮깁니다.
  4. 커밋: 2·3단계의 결과는 문제가 없으면 작업 브랜치에 자동으로 커밋합니다.
- `main`, `develop`에서는 직접 작업하지 않습니다.
- `cargo test --workspace`가 smv-core의 렌더링·설정·CSS 생성과 Help 언어 선택을 검사합니다. AppKit 화면 동작은 단위 테스트 대상이 아닙니다.
