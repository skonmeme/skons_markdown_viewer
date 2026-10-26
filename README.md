<p align="center"><img src="spec/STANDARD/app-icon.png" width="400" alt="skon's Markdown Viewer 아이콘 (라이트·다크)"></p>

# skon's Markdown Viewer

macOS용 Markdown 뷰어입니다. 한글과 영문 글꼴을 따로 고를 수 있고, 창 폭에 맞춰 좌우 여백이 저절로 조절됩니다.

## 요구 사항

- macOS 26 이상
- Apple Silicon Mac

## 설치

1. [Releases](https://github.com/skonmeme/skons_markdown_viewer/releases)에서 `MarkdownViewer-<버전>.pkg`를 내려받습니다.
2. 내려받은 파일을 열어 안내대로 설치합니다.

앱은 `/Applications/MarkdownViewer.app`에 설치되어 이 Mac의 모든 사용자가 쓸 수 있고, Finder와 Dock에는 "skon's Markdown Viewer"로 보입니다.

## 제거

Releases에서 `MarkdownViewer-uninstaller-<버전>.pkg`를 내려받아 실행합니다. 앱과 함께 실행한 사용자의 설정·캐시도 지워집니다. 설정을 남기려면 설치 과정의 **사용자화**에서 "현재 사용자의 설정 제거"를 끄세요.

## 사용법

| 하려는 것 | 방법 |
|---|---|
| 문서 열기 | Finder에서 이중 클릭(또는 다음으로 열기), Dock 아이콘이나 창에 끌어다 놓기, ⌘O, File > Open Recent |
| 검색 | ⌘F, 다음·이전 결과 ⌘G·⇧⌘G, 선택한 글자로 검색 ⌘E |
| 확대·축소 | ⌘+ · ⌘− · ⌘0 |
| 여백 | 기본은 자동. ⌥⌘+ · ⌥⌘−로 16포인트씩 조절, ⌥⌘0으로 자동 복귀 |
| 설정 | ⌘, — 글꼴 세트(영문·한글·코드 글꼴, 크기, 줄 간격), 색상(자동·사용자 지정), 여백 |
| 인쇄 | ⌘P |

- 열어 둔 파일이 바뀌면 화면이 저절로 갱신되고 스크롤 위치는 유지됩니다.
- 자세한 사용법은 앱의 **Help > skon's Markdown Viewer Help**에서 볼 수 있습니다.

## 개발자용 문서

- [spec/STANDARD.md](spec/STANDARD.md): 명세(유일한 기준 문서)
- [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md): 앱의 작동 방식과 빌드·배포
