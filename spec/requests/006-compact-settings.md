# Compact Settings window

## Request

> 원래 요청은 한국어이고 그대로 옮겼다. 요청서 005를 마친 뒤 문서용 화면을 찍는 대화에서 나왔다.

설정창의 기본 크기를 560으로 해줘.

Claude가 "설정 창 기본 크기의 560은 높이인가요, 폭인가요?"라고 묻자 고른 답은 "폭 (560×580)"이고, 덧붙인 말은 다음과 같다.

폭이 좁은 경우는 사이드바를 아이콘만 노출하는 방식으로 줄일 수 있는지 확인해줘

할 수 있는지 Claude가 확인해 답한 뒤의 지시는 다음과 같다.

지금 브랜치에 명세 요청서를 추가해주고, 스크린샷을 다시 만든 후에 commit해줘

## Background

- 문서용 화면을 찍다가 구현의 버그를 찾았다. 설정 창이 처음 열릴 때 저장된 크기와 관계없이 폭이 약 1,050 pt로 넓어지고, 그 폭이 저장되었다. 첫 배치가 `refresh()`보다 먼저 돌아, API key의 가린 칸과 보이는 칸(각 300 pt)을 함께 배치했기 때문이다. 요청자가 "함께 고친다"를 골라, 이 요청과 따로 고쳐 커밋했다. STANDARD와는 관계없다.
- 이 요청서는 지금 브랜치 `feat/search-by-llm`에 둔다. 요청자의 지시 "지금 브랜치에 명세 요청서를 추가해주고"를 따른 것이다. 한 브랜치에 요청 여러 개를 둘 수 있다(STANDARD §11.3 단계 1의 3).

## Current specification

STANDARD §9.1:

> App menu > Settings… (⌘,) opens the Settings window. Its sidebar has a search field and the items Fonts, Colors, Margins and AI. All settings are shown in one list next to the sidebar, in four groups with these titles:

> Navigation and search:
>
> - Choosing an item in the sidebar scrolls the list so that the title of its group is at the top. When the list is scrolled, the sidebar selects the group at the top. The list scrolls when it is taller than the window.
> - Text in the search field filters the list, ignoring case. A setting is shown when its label or the name of one of its options contains the text; all settings of a group are shown when the group title contains it. Only groups with settings shown appear, in the list and in the sidebar. When nothing matches, the list shows "No Results". An empty search field shows all settings.

STANDARD는 설정 창의 크기를 정하지 않는다. 구현(`crates/app/src/settings_window.rs`)은 처음 크기가 780×580 pt, 내용의 최소 크기가 620×360 pt이고, 크기와 위치를 `SMVSettings`라는 이름으로 저장한다. 사이드바의 폭은 190 pt(검색 필드와 `NSTableView`), 라벨 열의 폭은 140 pt다.

## Investigation

### 지금 내용이 차지하는 폭

> 2026-10-28에 앱에서 묶음마다 `fittingSize`를 재었다.

| 묶음 | 폭(pt) | 가장 넓은 행 |
|---|---:|---|
| Fonts | 472.5 | Font set(팝업, +, −, Rename…) |
| Colors | 310.5 | Colors(Automatic, Custom) |
| Margins | 434 | Fixed margin(슬라이더 220과 값) |
| AI | 530 | URL(칸 380). 모델 옵션이 한 줄 있으면 약 545 |

- 창의 폭은 사이드바 + 구분선 1 + 목록 좌우 여백 48 + 가장 넓은 묶음이다.
- 이름이 보이는 지금의 사이드바(190 pt)로는 769 pt가 필요하다. 모델 옵션이 있으면 약 785 pt다.
- 아이콘만 보이는 사이드바(약 52 pt)로도 631 pt, 모델 옵션이 있으면 약 645 pt가 필요하다. 아이콘으로 줄이는 것만으로는 560 pt에 들어가지 않는다. 560 pt에 넣으려면 묶음이 459 pt 안에 들어가야 한다.

### AppKit에서 할 수 있는 것

- AppKit의 표준 사이드바(`NSSplitViewItem`의 sidebar)는 창을 줄이면 통째로 접히게 할 수 있지만(`canCollapseFromWindowResize`), 아이콘만 남기는 모드는 없다. 아이콘 모드는 직접 만든다. 사이드바의 행마다 SF Symbol 아이콘과 이름을 두고, 창이 좁으면 이름을 숨기고 사이드바의 폭을 줄인다.
- 검색 필드는 아이콘만 보이는 사이드바(약 52 pt)에 들어가지 않는다.
- `NSSearchToolbarItem`(macOS 11부터)은 툴바에 검색 필드를 두는 표준 항목이다. 툴바에 공간이 모자라면 돋보기 버튼으로 접히고, 검색을 시작하면 펼쳐진다(헤더의 `beginSearchInteraction`: "If necessary, expands to the preferred width").
- 아이콘 후보가 SF Symbols에 있는 것을 확인했다: `textformat`, `paintpalette`, `arrow.left.and.right`, `sparkles`.

## Decisions

> 각 결정의 "근거"에는 요청자의 말을 그대로 옮겼다. P로 시작하는 항목은 Claude의 제안이고, 아직 승인된 제안은 없다.

### D1. 설정 창의 기본 크기는 560×580 pt다

- 내용: 저장된 크기가 없을 때 설정 창은 폭 560 pt, 높이 580 pt로 열린다. 높이는 지금과 같다. 그 뒤로는 지금처럼 사용자가 바꾼 크기와 위치로 열린다.
- 근거: "설정창의 기본 크기를 560으로 해줘." / Claude의 질문에 고른 답 "폭 (560×580)"
- 이유: 요청자가 밝히지 않았다.

### D2. 창이 좁으면 사이드바에 아이콘만 보인다

- 내용: 창의 폭이 좁으면 사이드바는 항목의 아이콘만 보이고, 그만큼 폭이 줄어든다. 언제부터 좁은지는 아직 정하지 않았다(P3).
- 근거: "폭이 좁은 경우는 사이드바를 아이콘만 노출하는 방식으로 줄일 수 있는지 확인해줘"
- 해석: 요청자는 할 수 있는지 확인하라고 했다. Claude는 AppKit의 기본 기능은 아니지만 직접 만들 수 있다고 답했고(Investigation), 요청자는 그 뒤 이 요청서를 만들라고 했다. 그래서 아이콘 모드를 쓰기로 한 것으로 읽었다.
- 이유: 560 pt에서는 이름이 보이는 사이드바(190 pt)와 설정 목록이 함께 들어가지 않는다(Investigation).

### P1. (Claude의 제안) 검색 필드를 창의 툴바로 옮긴다

- 내용: 검색 필드를 사이드바에서 창의 툴바(`NSSearchToolbarItem`)로 옮긴다. 창의 폭과 관계없이 늘 그 자리다. 툴바에 공간이 모자라면 돋보기 버튼으로 접히고, 누르면 펼쳐진다. 거르는 규칙은 지금과 같다.
- 이유: 아이콘만 보이는 사이드바에는 검색 필드가 들어가지 않는다. 툴바의 검색 항목은 접고 펼치는 동작을 AppKit이 해 주므로 직접 만들 것이 적고, 다른 Mac 앱의 설정 창과도 같다.
- 대안:
  - (나) 아이콘 모드에서만 사이드바 맨 위에 돋보기 버튼을 두고, 누르면 검색 필드를 펼친다.
  - (다) 아이콘 모드에서만 검색 필드를 목록 맨 위로 옮긴다.

### P2. (Claude의 제안) 항목의 아이콘

- 내용: Fonts는 `textformat`, Colors는 `paintpalette`, Margins는 `arrow.left.and.right`, AI는 `sparkles`다. 넓은 창에서는 아이콘 옆에 이름이 보이고, 아이콘 모드에서는 포인터를 올리면 이름이 보인다.
- 이유: 아이콘 모드에서 항목을 가리려면 아이콘이 필요하다. 두 모드에서 같은 아이콘이 보여야 모드가 바뀌어도 헷갈리지 않는다. AI 묶음은 외부 LLM도 다루므로, Apple Intelligence의 기호(`apple.intelligence`)는 쓰지 않는다.

### P3. (Claude의 제안) 칸을 좁히고, 700 pt보다 좁으면 아이콘 모드로 한다

- 내용: 칸의 폭을 아래 표처럼 줄이고, 창의 폭에 따라 사이드바의 모드를 바꾼다.

| 칸 | 지금(pt) | 제안(pt) |
|---|---:|---:|
| 라벨 열 | 140 | 120 |
| URL | 380 | 300 |
| API key | 300 | 280 |
| 모델 옵션의 키, 값 | 150, 200 | 110, 150 |
| Max tokens 슬라이더 | 200 | 150 |
| 글꼴 세트 팝업 | 이름에 따라 넓어짐 | 지금 폭으로 고정, 긴 이름은 줄임 |

- 가장 넓은 묶음은 약 453 pt(Fonts)가 된다(어림). 아이콘 사이드바(52 pt)로는 약 554 pt, 이름이 보이는 사이드바(190 pt)로는 약 692 pt에 들어간다.
- 창의 폭이 700 pt보다 좁으면 아이콘 모드이고, 700 pt 이상이면 이름이 보인다.
- 창의 최소 폭은 560 pt다.
- 이유: D1의 560 pt에 아이콘 사이드바와 설정 목록을 함께 넣기 위해서다. 700 pt는 이름이 보이는 사이드바와 가장 넓은 묶음이 들어가는 폭(약 692 pt)에 여유를 둔 값이다.
- 대안: 칸의 폭을 고정하지 않고 창의 폭에 따라 늘고 줄게 하고, 최소 폭만 정한다.

## Resulting specification

> STANDARD가 갖게 될 내용이다. 단계 2에서 영어로 쓴다. [D…]는 요청자가 정한 것이고, [P…]는 아직 승인되지 않은 제안이다. 제안이 승인되면 그대로, 바뀌면 바뀐 대로 반영한다.

### §9.1 Settings window

- 설정 창은 처음에 560×580 pt로 열린다. 그 뒤로는 사용자가 바꾼 크기와 위치로 열린다 [D1].
- 사이드바의 항목에는 아이콘과 이름이 있다 [P2]. 창의 폭이 700 pt보다 좁으면 아이콘만 보이고, 포인터를 올리면 이름이 보인다 [D2, P3]. 창의 최소 폭은 560 pt다 [P3].
- 검색 필드는 창의 툴바에 있고, 공간이 모자라면 돋보기 버튼으로 접힌다 [P1].
- 항목을 고르면 목록이 스크롤되는 것, 목록을 스크롤하면 항목이 선택되는 것, 거르는 규칙과 "No Results"는 지금과 같다.

## Impact

### 사용자에게 보이는 변화

- 크기가 저장되지 않은 Mac에서 설정 창이 처음에 560 pt 폭으로 열리고, 사이드바에는 아이콘만 보인다 [D1, D2]. 창을 700 pt 넘게 넓히면 이름도 보인다 [P3].
- 크기가 이미 저장된 Mac에서는 저장된 크기로 열린다.
- 검색 필드가 사이드바에서 툴바로 옮겨진다 [P1].
- 설정 창의 몇몇 칸이 좁아진다 [P3].

### 바꿀 STANDARD 절과 그림

| 절·그림 | 바꿀 내용 |
|---|---|
| §9.1 | 처음 크기, 사이드바의 아이콘과 아이콘 모드, 최소 폭, 검색 필드의 자리 |

STANDARD에는 설정 창의 그림이 없으므로 그림은 바뀌지 않는다.

### 바꿀 소스 코드와 테스트

| 파일 | 바꿀 내용 |
|---|---|
| `crates/app/src/settings_window.rs` | 처음 크기와 최소 폭, 사이드바 행의 아이콘, 폭에 따른 아이콘 모드, 검색 필드의 자리, 칸의 폭 |
| 테스트 | 아이콘 모드로 바뀌는 폭의 판단을 smv-core의 함수로 두면 단위 테스트를 둔다. 화면 배치는 단위 테스트의 대상이 아니다 |

### 바꿀 문서

| 문서 | 바꿀 내용 |
|---|---|
| `README.md` | 사용법 표의 설정 행("사이드바의 검색 필드") |
| `packaging/help/ko.md`, `en.md` | 설정: 사이드바와 검색 필드의 설명 |
| `doc/ARCHITECTURE.md` | §10 설정 창의 구조 |
| `doc/ARCHITECTURE/settings.png` | 설정 창을 다시 찍는다 |

## Open questions

1. 검색 필드의 자리: P1(툴바), 또는 대안 (나), (다).
2. 항목의 아이콘, 넓은 창에서도 아이콘을 이름 옆에 보일지: P2.
3. 칸의 폭, 아이콘 모드로 바뀌는 폭(700 pt), 최소 폭(560 pt): P3, 또는 칸이 창의 폭에 따라 늘고 주는 대안.
