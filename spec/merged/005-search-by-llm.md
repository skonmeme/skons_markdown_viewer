# Search by LLM

## Request

> The original request below is written in Korean and is kept verbatim.

키워드 검색말고 LLM 기반의 맥락 검색이 기능을 추가하는 명세요청서를 만들어줘

1. develop으로부터 feat/search-by-llm 브랜치 생성
2. LLM 기반은 Apple Foundation Model (AFM)
3. Embedding이 필요할까? RAG 방식을 쓰려면 매번 DB 구축을 해야할까? AFM에는 Embedding 지원이 있어? 
4. 설정에 AI 검색이 사용 on/off 넣고 기본은 on. 이 경우 검색창에 별도 표시없이 AI 기반 검색 활성화 (off시 keyword 검색 기본)
5. 검색창에 /ai 입력으로 AI 기반 검색, /key 입력으로 keyword 기반 검색, /match 입력으로 regex 기반 검색 (case-insensitive), /case 입력으로 regex 기반 검색 (case-sensitive)
6. 검색창에 /ai 등을 입력하고 스페이스를 누르면 해당 기능 활성화 의미로 ai에 둥근 테두리 배경으로 둘러쌓인 블럭으로 변경 (/ 삭제). 해당 블럭 삭제함으로써 기능 리셋

## Background

- 요청 1에 따라 Claude가 `develop`에서 `feat/search-by-llm` 브랜치를 만들고 이 요청서를 그 브랜치에 두었다(2026-10-27). STANDARD §11.3 단계 1의 2번은 Claude가 브랜치를 만들지 않는다고 정하지만, 이번에는 요청자가 명시적으로 지시했다.
- 요청 3은 결정이 아니라 질문이다. 답은 Investigation에 적었고, 그에 따른 Claude의 제안은 P1이다.
- 용어: 요청 6의 "블럭"을 이 요청서에서는 **토큰**이라고 부른다. 문서의 **블록**(§6의 문단, 목록 항목, 표 칸 등)과 헷갈리지 않게 하기 위해서다.
- 요청자는 같은 날 여덟 번 더 결정을 바꾸고 더했다(D5–D8, D9–D17, D18–D20, D21–D24, D25–D26, D27–D28, D29–D30, D31). 바뀐 결정과 제안은 지우지 않고, 무엇으로 바뀌었는지 그 아래에 덧붙였다.

## Current specification

STANDARD §6:

> - Edit > Find > Find… (⌘F) shows the find bar at the top of the window (Figure 3). It contains a search field, Previous and Next buttons, a match counter, a Match Case option and a Done button.
> - Searching starts while the user types. The query is matched as literal text, ignoring case unless Match Case is on.
> - A match can cover text with different styles inside one block, but never text from different blocks (paragraphs, list items, table cells, …). A space in the query matches any white space.
> - All matches are highlighted. The current match is highlighted in a stronger color and scrolled into view. A match inside a collapsed section opens that section.
> - The counter shows the position of the current match and the number of matches ("3 of 12"), or "Not found".
> - Return and ⌘G go to the next match; Shift-Return and ⇧⌘G go to the previous match. The search wraps around at the end and the start of the document.
> - Edit > Find > Use Selection for Find (⌘E) searches for the selected text.
> - Esc and Done close the find bar and remove the highlights. The current match becomes the text selection.
> - If the document is reloaded (§4.4) while the find bar is open, the search is repeated on the new content.

STANDARD §9.1의 설정 창은 한 화면에 Fonts, Colors, Margins 세 묶음을 둔다. 사이드바는 없다.

구현: 검색은 페이지 스크립트 `crates/core/assets/viewer.js`가 하고, 검색 막대는 `crates/app/src/find_bar.rs`, 둘의 연결은 `crates/app/src/viewer.rs`가 맡는다. 설정 창(`crates/app/src/settings_window.rs`)은 `NSGridView` 하나에 세 묶음을 구분선으로 나눠 둔다. 앱에는 Swift 코드가 없다(`doc/ARCHITECTURE.md` §1). STANDARD §11.1은 Swift를 꼭 필요한 곳에만 허용한다.

## Investigation

> 요청 3의 질문에 대한 조사다. 2026-10-27에 Apple 문서와 WWDC26 자료를 읽고, 이 Mac(macOS 26.7.1, Apple M4 Pro, Xcode의 macOS SDK 27.0)에서 직접 확인했다.

### AFM에 Embedding 지원이 있는가

없다.

- Foundation Models 프레임워크는 글 생성(`LanguageModelSession`), 구조화된 출력(`@Generable`), 도구 호출, 토큰 세기(`tokenCount(for:)`)를 제공한다. 벡터를 돌려주는 API는 없다. macOS 27 SDK의 FoundationModels 인터페이스에도 "embedding"이나 "vector"가 들어간 선언이 하나도 없다.
- Swift 전용 프레임워크다. SDK에 Objective-C나 C 헤더가 없다. Rust 앱에서 쓰려면 Swift로 작은 연결 코드를 만들어야 한다.
- 쓸 수 있는 조건: Apple Intelligence를 지원하는 Mac과 지역에서 Apple Intelligence를 켜고 모델을 내려받은 상태여야 한다. 아니면 `SystemLanguageModel.availability`가 `deviceNotEligible`, `appleIntelligenceNotEnabled`, `modelNotReady` 가운데 하나를 돌려준다. 이 Mac은 Apple Intelligence가 꺼져 있어 `appleIntelligenceNotEnabled`였다.
- 언어: 23개 언어를 지원하고 한국어(ko-KR)가 들어 있다(이 Mac에서 확인).
- 문맥 창: 한 세션의 입력과 출력을 합쳐 4096 토큰이다(이 Mac의 `contextSize`, Apple TN3193). WWDC26 세션의 예제 코드는 새 모델에서 8192를 출력한다. 토큰 하나는 한국어 약 1글자, 영어 약 3–4글자다(TN3193).
- 계산: 지침과 답에 약 600 토큰을 남기면 한 번에 넘길 수 있는 본문은 약 3,500 토큰, 곧 한국어 약 3,500자, 영어 약 10,000–14,000자다.
- 모델은 OS와 함께 바뀐다(26.0–26.3, 26.4, 27.0의 세 버전). 같은 질의라도 OS 버전에 따라 결과가 달라질 수 있다.

### Apple 플랫폼의 다른 Embedding

| 방법 | 한국어 | 특징 | 이 요청에 쓸 수 있나 |
|---|---|---|---|
| `NLEmbedding.sentenceEmbedding` (NaturalLanguage) | 안 됨(이 Mac에서 `nil`) | 문장마다 512차원 벡터 하나. Apple이 의미 유사도에 권하는 API | 한국어 문서에 못 쓴다 |
| `NLContextualEmbedding` (NaturalLanguage, macOS 14+) | 됨(중·일·한 모델) | 토큰마다 512차원 벡터, 한 번에 256 토큰까지. 중·일·한 모델과 라틴 문자 모델(영어 등 20개 언어)이 따로 있어 두 모델의 벡터는 서로 비교할 수 없다. Apple 문서는 의미 유사도에는 `NLEmbedding`을 쓰라고 한다 | 한·영이 섞인 문서, 한국어 질의로 영어 본문을 찾는 경우에 맞지 않는다 |
| Core Spotlight 의미 검색 (`CSUserQuery`, macOS 15+) | 확인하지 않음 | 앱이 내용을 Spotlight 색인에 넣어 두면 Spotlight가 의미 검색을 한다. 색인은 시스템에 남는다. 개발자 포럼에 의미 검색이 동작하지 않는다는 보고가 있다 | 문서를 시스템 색인에 넣어야 하고, 문서 하나 안의 위치를 찾기에는 과하다 |
| `SpotlightSearchTool` (WWDC26, macOS 27+) | 확인하지 않음 | AFM이 도구 호출로 앱의 Core Spotlight 색인을 검색하는 RAG. 앱이 먼저 내용을 Core Spotlight에 넣어야 한다 | macOS 26도 지원해야 하므로(§3) 쓸 수 없다 |
| 다국어 Embedding 모델을 앱에 넣기 (Core ML 등) | 모델에 따라 다름 | 앱이 수백 MB 커진다 | AFM을 쓰기로 한 요청 2와 다르다 |

### RAG를 쓰려면 매번 DB를 만들어야 하는가

- RAG는 글을 조각으로 나눠 Embedding 벡터로 DB에 넣어 두고, 질의와 가까운 조각을 찾아 LLM에 넘기는 방식이다. DB는 찾을 대상이 많고 오래 두고 쓸 때(폴더 전체, 여러 문서) 필요하다.
- 이 앱의 검색 범위는 지금 창의 문서 하나이고, 그 본문은 이미 메모리에 있다. 그래서 DB는 필요 없다. Embedding을 쓰더라도 창이 열려 있는 동안 메모리에만 두면 되고, 문서가 바뀌면 바뀐 블록만 다시 계산하면 된다.
- 그런데 AFM에는 Embedding이 없고, Apple의 다른 Embedding은 한국어를 못 쓰거나 한·영을 함께 비교하지 못한다. 그래서 Embedding도 쓰지 않고 AFM이 본문을 직접 읽어 관련 블록을 고르게 하는 것을 제안한다(P1). RAG의 "찾기"를 LLM이 직접 하는 방식이고, 답을 따로 만들어 내지는 않는다.
- 단점은 검색할 때마다 AFM이 문서 전체를 읽는다는 것이다. 한국어 30,000자 문서는 약 9번 나눠 묻는다(30,000 ÷ 3,500). 실제로 걸리는 시간은 Apple Intelligence가 켜진 Mac에서 재 봐야 한다. 이 Mac에서는 잴 수 없었다.
- 대안: AFM이 질의를 관련 낱말(동의어, 번역어)로 넓히고(1번), 그 낱말로 블록을 고른 뒤 상위 블록만 AFM이 판정한다(1–2번). 긴 문서에서도 빠르지만, 넓힌 낱말이 하나도 없는 블록은 놓친다.

### 외부 LLM의 API 형식

> D8(외부 LLM)을 위한 조사다(2026-10-27).

- 가장 널리 쓰이는 형식은 OpenAI 호환 Chat Completions다. 요청은 `POST …/chat/completions`, 열쇠는 `Authorization: Bearer <API key>` 헤더, 본문은 `model`과 `messages`다. OpenAI뿐 아니라 Ollama 같은 로컬 서버(`http://localhost:11434/v1/chat/completions`)와 여러 게이트웨이가 이 형식을 받는다.
- Anthropic(Claude)의 기본 API는 Messages API다. 요청은 `POST https://api.anthropic.com/v1/messages`, 헤더는 `x-api-key`와 `anthropic-version`이고, 본문에 `max_tokens`가 꼭 있어야 한다. Anthropic도 OpenAI 호환 주소(`https://api.anthropic.com/v1/`)를 제공하지만, 시험과 비교를 위한 것이고 운영용으로는 권하지 않는다.
- 모델 옵션은 공급자와 모델마다 다르다. 예: Anthropic의 OpenAI 호환 주소에서 Claude 4.7 이후 모델은 `temperature`를 1보다 작게 주면 오류(400)를 돌려주고, `response_format`은 무시한다.

### macOS 27의 문맥 창과 Private Cloud Compute

> 요청자의 질문 "macOS 27.2버전이 나왔는데 최대 토근 수는 얼마?"와 D15(PCC)를 위한 조사다(2026-10-27).

- Mac 안의 AFM: macOS 26에서는 4,096 토큰이다. macOS 27.0에서는 새 기기에서 8,192 토큰이고 그 밖의 기기에서는 4,096 토큰이다(WWDC26 세션 319의 예제 주석). Apple은 어떤 기기가 새 기기인지 밝히지 않았고, 개발자들이 잰 값도 4,096과 8,192로 엇갈린다. 27.2의 값은 따로 발표된 것을 찾지 못했다. 그래서 앱은 숫자를 정해 두지 않고 실행할 때 `contextSize`를 읽는다(P1, P19). 이 Mac은 macOS 26.7.1이라 4,096이다.
- PCC(`PrivateCloudComputeLanguageModel`): macOS 27.0 이상에만 있다(macOS 27 SDK에서 확인). 문맥 창은 32,768 토큰이다(WWDC26 세션 319).
  - 사용자 쪽 조건: Apple Intelligence를 지원하는 기기여야 한다. 아니면 `availability`가 `deviceNotEligible`이나 `systemNotReady`를 돌려준다.
  - 한도: 사용자마다 하루 한도가 있고 사용자의 iCloud 계정으로 센다. iCloud+를 쓰면 한도가 늘어난다. 한도에 닿으면 `quotaLimitReached`(다시 쓸 수 있는 시각이 들어 있음)를 돌려준다. 그 밖의 오류는 `networkFailure`, `serviceUnavailable`이다.
  - 개발자 쪽 조건: Apple의 PCC managed entitlement를 받아야 한다. 받을 수 있는 조건은 App Store Small Business Program 가입과, 개발자의 모든 앱을 합친 App Store 첫 다운로드 200만 회 미만이다. Apple 안내는 App Store로 배포하는 앱과 TestFlight·ad hoc 시험만 말하고, 이 앱처럼 Developer ID로 App Store 밖에서 배포하는 경우(§10)는 말하지 않는다.
  - 개인 정보: Apple은 PCC가 데이터를 저장하지 않고 요청을 처리하는 데에만 쓴다고 밝힌다. 그래도 문서의 글은 Mac 밖의 Apple 서버로 간다.
- 계산 1, 묶음 수: 한국어 30,000자(약 30,000 토큰) 문서를 나눌 때다. 지침과 답의 몫은 600 토큰, 겹침은 256 토큰(D17의 해석)으로 했다. 문서를 `T`, 한 묶음의 본문을 `B`, 겹침을 `O`라 하면 묶음 수는 `T ≤ B`일 때 1이고, 아니면 `1 + ⌈(T − B) ÷ (B − O)⌉`이다.

| 모델 | 문맥 창 | 한 묶음의 본문 `B` | 묶음 수 |
|---|---:|---:|---:|
| Mac 안의 AFM, macOS 26 | 4,096 | 3,496 | 10 |
| Mac 안의 AFM, macOS 27 새 기기 | 8,192 | 7,592 | 5 |
| PCC, macOS 27 | 32,768 | 32,168 | 1 |
| 외부 LLM, 최대치 32K (P19) | 32,768 | 26,214 (80 %) | 2 |
| 외부 LLM, 최대치 128K (P19) | 131,072 | 104,857 (80 %) | 1 |

- 계산 2, Mac 안의 AFM에서 걸리는 시간(어림): Apple이 밝힌 값은 2024년 모델, iPhone 15 Pro 기준으로 입력 토큰당 약 0.6 ms, 출력 초당 약 30 토큰이다. 묶음마다 지침과 질의 300 토큰을 넣고 답(블록 번호) 30 토큰을 받는다고 보았다.
  - macOS 26(4,096): 입력 30,000 + 겹침 9 × 256 + 지침 10 × 300 = 35,304 토큰 → 약 21초. 출력 10 × 30 = 300 토큰 → 약 10초. 합쳐서 약 31초.
  - macOS 27 새 기기(8,192): 입력 30,000 + 4 × 256 + 5 × 300 = 32,524 토큰 → 약 20초. 출력 5 × 30 = 150 토큰 → 약 5초. 합쳐서 약 25초.
  - 문맥 창이 두 배가 되어도 전체 시간은 약 20 %만 준다. 시간의 대부분은 문서 자체를 읽는 데 들기 때문이다. Mac(M4 Pro)은 iPhone 15 Pro보다 빠를 것으로 보이지만 재 봐야 안다. PCC와 외부 LLM은 서버에서 돌므로 이 어림이 맞지 않는다.

### macOS 26과 macOS 27의 AFM

> 요청자의 질문 "macOS 26과 macOS 27의 AFM 성능 차가 꽤 큰 것으로 알고 있는데? macOS 26은 PCC, local 상관없이 4096만 가능한 건가?"에 대한 조사다(2026-10-27).

- macOS 26에서 앱이 쓸 수 있는 것은 Mac 안의 모델(`SystemLanguageModel`)뿐이고, 문맥 창은 4,096 토큰이다. 앱이 PCC를 부르는 API(`PrivateCloudComputeLanguageModel`)는 macOS 27.0부터 있다(macOS 27 SDK에서 확인). 그래서 macOS 26에서 앱이 쓸 수 있는 모델은 4,096 토큰의 Mac 안 모델 하나다.
- 덧붙임(D27을 위한 확인, 2026-10-27): macOS 27 SDK에서 `PrivateCloudComputeLanguageModel`에는 더 이른 OS에서 쓰게 하는 표시(`@backDeployed`)가 없다. SDK의 `@backDeployed` 일곱 개는 모두 PCC와 관계없다. macOS 26에서 PCC는 Apple Intelligence의 시스템 기능(Writing Tools 등)이 쓰는 것이고, 앱이 부를 수 있는 API는 없다.
- macOS 27의 모델은 3세대 Apple Foundation Models다(WWDC26).

| 모델 | 도는 곳 | 크기 | 문맥 창 |
|---|---|---|---|
| AFM 3 Core | Mac 안 | 3B dense. 2025년 모델의 다음 세대 | 4,096 |
| AFM 3 Core Advanced | Mac 안. Apple의 말로 "가장 성능이 좋은 Apple silicon 시스템" | 20B sparse. 요청마다 1–4B만 쓴다 | 8,192(추정) |
| AFM 3 Cloud | PCC | 밝히지 않음 | 32,768 |

- 8,192 토큰이 AFM 3 Core Advanced의 값이라는 것은 Claude의 추정이다. Apple은 "새 기기에서 8,192"라고만 했고, 어떤 기기가 Core Advanced를 쓰는지, Foundation Models 프레임워크가 어느 모델을 주는지는 밝히지 않았다.
- 품질 차이(Apple의 사람 평가, 같은 질문의 두 답 가운데 나은 쪽을 고르게 한 것):
  - AFM 3 Core는 질문의 45.6 %에서 선호되었고, 2025년 모델(macOS 26의 Mac 안 모델)은 23.3 %에서 선호되었다.
  - AFM 3 Cloud(PCC)는 64.7 %, 2025년 서버 모델은 8.7 %였다. 지시 따르기는 21 % 나아졌다.
  - 표준 벤치마크와 처리 속도는 발표되지 않았다. Apple은 기술 보고서를 나중에 내겠다고 했다.
- 요약: 요청자의 말대로 차이가 크다. macOS 27은 모델이 낫고, 새 기기에서는 문맥 창이 두 배이며, PCC(32,768 토큰)를 쓸 수 있다. 속도의 차이는 아직 알 수 없다.

### 겹침이 필요한가

> 요청자의 질문 "겹침이 필요하긴 해? 이것부터 조사해줘"에 대한 조사다(2026-10-27).

- RAG 연구에서 겹침의 효과는 엇갈린다. 겹침은 같은 글을 여러 번 넣어 중복과 비용을 늘리고, 정확도에 주는 도움은 조건에 따라 다르다.
  - Chroma(2024): 겹침을 줄이면 IoU(찾아 온 글 가운데 필요한 글의 비율)가 좋아졌다. 성능이 좋은 Embedding 모델(`text-embedding-3-large`)에서는 겹침이 recall을 올리지 않았다. 예: 400 토큰 조각에서 겹침 200은 88.1, 겹침 0은 89.5. 작은 Embedding 모델(`all-MiniLM-L6-v2`)에서는 겹침이 있어야 recall이 높았다.
  - 다른 연구(화학 문서 RAG 2025, PruningRAG 2024, CRUD-RAG 2024)도 겹침의 효과가 조각 크기와 과제에 따라 다르다고 보고한다. 겹침이 recall을 조금 올리는 대신 precision을 떨어뜨리거나 색인을 키운다는 보고가 있다.
- 이 연구들의 겹침은 글을 일정한 토큰 수로 잘라(문장 중간에서도) 조각마다 Embedding을 만드는 경우다. 잘린 자리에 걸친 내용을 어느 한 조각이 온전히 갖게 하려는 것이다. 이 요청의 방식(P1)은 다르다.
  - 묶음은 블록의 경계에서만 나눈다. 블록 하나가 한 묶음보다 길 때만 블록 안을 자른다.
  - 결과의 단위가 블록이고, 블록마다 그 블록이 속한 제목을 붙여 넘긴다.
  - 그래서 겹침이 없어도 잘리는 블록은 없다. 남는 문제는 묶음 첫머리의 블록이 바로 앞 블록의 문맥(예: "다음 명령을 실행한다" 뒤의 코드 블록)을 못 보는 것이다. 제목이 그 문맥의 상당 부분을 주고, 묶음을 제목 바로 앞에서 나누면 이런 경우가 더 줄어든다.
- 겹침이 꼭 필요한 곳은 블록 하나가 한 묶음보다 길어 그 안을 자를 때다. 잘린 자리에 걸친 문장이 어느 한 조각에 온전히 들어가도록 조각을 겹쳐야 한다.
- 계산 3, 겹침의 비용: 한국어 30,000자 문서를 macOS 26(4,096)에서 묶음 사이 256 토큰씩 겹치면, 묶음 경계 9곳 × 256 = 2,304 토큰을 더 읽는다. 문서의 약 7.7 %이고, 계산 2의 어림으로 약 1.4초다. 같은 블록을 두 묶음에서 판정하므로 결과도 합쳐야 한다.
- 결론(Claude의 판단): 이 방식에서는 묶음 사이의 겹침이 꼭 필요하지 않다. 블록 경계에서, 가능하면 제목 앞에서 나누고 블록마다 제목을 붙이면 된다. 겹침은 긴 블록을 자를 때만 필요하다(P20).

### PCC의 개발자 조건

> 요청자의 질문 "PPC 개발자 계정 조건이 뭔지 알려줘"에 대한 조사다(2026-10-27). 요청자가 쓴 "PPC"는 PCC로 읽었다.

- PCC를 앱에서 쓰려면 개발자가 다음 셋을 모두 갖춰야 한다(Apple의 Accessing Private Cloud Compute).
  1. App Store Small Business Program에 가입되어 있다.
  2. 개발자의 모든 앱을 합쳐 App Store 첫 다운로드가 200만 회 미만이다. 시험 설치는 세지 않는다.
  3. 개발자 계정에 PCC entitlement가 부여되어 있다. Apple의 신청 양식으로 요청한다.
- 비용과 한도: 조건을 갖춘 개발자는 클라우드 API 비용 없이 쓴다. 사용자마다 하루 한도가 있다. Apple은 더 넓은 사용을 앞으로 유료로 열겠다고 했다.
- 배포: 조건을 갖춘 개발자는 App Store로 배포하는 앱에서 PCC를 쓰고, TestFlight나 ad hoc 배포로 시험할 수 있다. Developer ID 배포는 언급이 없고, Apple 개발자 계정 도움말의 macOS 기능(capability) 표에도 PCC가 아직 없다.
- 조건을 벗어나면: 앱의 첫 다운로드가 200만 회를 넘거나 개발자가 Small Business Program에서 빠지면 Apple이 알리고, 개발자는 6개월 안에 다른 방법으로 옮겨야 한다.
- App Store Small Business Program의 조건:
  - Apple Developer Program의 Account Holder여야 하고, App Store Connect에서 Paid Apps 계약을 받아들이고, 관련 개발자 계정(지분의 50 %를 넘게 갖거나 결정권을 가진 계정)을 모두 적어야 한다.
  - 본인과 관련 계정을 합친 App Store 수익(proceeds)이 지난해와 올해 각각 100만 달러 이하여야 한다. App Store에 처음 올리는 개발자도 들어갈 수 있다.
  - 가입하면 App Store 수수료가 15 %로 준다.
- 이 앱에 주는 뜻: 이 앱은 App Store가 아니라 Developer ID로 서명한 pkg로 배포한다(§10). Apple이 Developer ID 배포에서 PCC를 허용하는지 확인되지 않았다. 허용되지 않는다면 PCC를 쓰려면 Mac App Store로 배포해야 하는데, 그것은 §10과 다르다.

### PCC 계정 확인

> 요청자가 자기 Apple 계정으로 PCC를 쓸 수 있는지, 이 앱에 넣을 수 있는지 확인하라고 했다(2026-10-27). 이 Mac에 있는 것만 읽어서 확인했다. Apple 웹 계정에는 로그인하지 않았고, 저장된 자격 증명도 쓰지 않았다.

- 이 Mac에서 확인한 것:
  - Xcode에 유료 개인(Individual) 팀 하나가 로그인되어 있다: 이름 "Sung Gon Yi", 팀 ID `P5WD874MH2`. Xcode 설정에 Apple ID 주소는 보이지 않아, 이 팀이 요청자가 알려 준 계정의 팀인지까지는 확인하지 못했다.
  - 서명 인증서는 "Developer ID Application: Sung Gon Yi (P5WD874MH2)"와 "Apple Development: Sung Gon Yi"가 있다.
  - provisioning profile은 하나도 없고, Xcode에도 PCC 기능의 정의가 없다. 이 팀에 PCC entitlement가 부여되었다는 흔적은 없다.
- 이 Mac에서는 확인할 수 없는 것(Apple 웹 계정에 로그인해야 한다):
  - App Store Small Business Program 가입 여부: App Store Connect의 Business 화면이나 Small Business Program 가입 페이지에서 본다.
  - App Store에 올린 앱과 첫 다운로드 수.
  - PCC entitlement의 신청과 부여: Apple의 신청 양식으로 요청하고, 부여되면 Certificates, Identifiers & Profiles의 App ID 기능 목록에 나타난다.
  - Developer ID 배포에서 PCC를 쓸 수 있는지: Apple에 물어야 한다.
- 판단:
  - 계정 쪽: 유료 개인 계정이므로 본인이 Account Holder다. App Store에 올린 앱이 없다면 수익과 다운로드 조건은 맞는다. Small Business Program에 가입하고 PCC entitlement를 신청해 받으면 계정 조건은 갖출 수 있어 보인다.
  - 앱 쪽: 지금은 entitlement가 없어 이 앱에서 PCC를 부를 수 없다. 받더라도, Apple이 Developer ID 배포를 허용한다고 확인해 주기 전에는 이 앱에 넣을 수 없다. 넣게 되면 entitlement가 든 provisioning profile을 앱 번들에 넣어야 하므로 번들 만들기(`xtask`)와 릴리스 워크플로(서명용 profile)도 바뀐다.
- 그 뒤의 결정(D26): 요청자가 PCC entitlement를 신청한다. 신청할 때 할 일은 다음과 같다.
  1. App Store Connect에서 Small Business Program에 가입한다(Paid Apps 계약 수락, 관련 개발자 계정 목록 작성).
  2. Certificates, Identifiers & Profiles에 이 앱의 App ID(`io.github.skonmeme.markdownviewer`)를 등록한다.
  3. Apple의 PCC 신청 양식으로 entitlement를 요청한다. 이 앱이 App Store 밖에서 Developer ID로 배포된다는 것을 적고, 그렇게 배포해도 쓸 수 있는지 묻는다.
  4. 부여되고 Developer ID에서도 쓸 수 있으면, App ID에서 PCC 기능을 켜고 Developer ID용 provisioning profile을 만든다(P25).
- 주의: 개발자 쪽 조건(첫 다운로드 200만 회 미만)과 달리, 사용자마다 하루 한도는 앱의 사용자 수와 관계없이 있다. 한도는 사용자의 iCloud 계정으로 센다.
- 그 뒤의 결정(D31): PCC를 이 요청에서 뺐다. 위의 신청은 하지 않는다.

### Small Business Program과 사업자 등록

> 요청자의 질문 "Small Business Program 가입할려면 사업자 등록까지 해야하는데.. 이거 굳이 필요해?"에 대한 조사다(2026-10-27).

- 무료 앱이라도 PCC의 조건에는 Small Business Program 가입이 들어 있다. Apple의 PCC 안내에 무료 앱의 예외는 없다. 무료 앱은 수수료가 없으므로, 수수료 때문이라면 가입할 까닭이 없다.
- 가입하려면 Paid Apps 계약을 받아들여야 한다. Apple 도움말에 따르면 이 계약에 서명한 뒤 금융 정보와 세금 정보를 넣고, 개발자 계정의 지역에 따라 세금 양식을 더 내야 할 수 있다. 한국의 개인 개발자에게 사업자 등록이 필요하다는 것은 요청자의 말이고, Apple 자료에서는 확인하지 못했다.
- Apple은 PCC를 더 넓게 쓰는 길을 앞으로 유료로 열겠다고 했다. 그 조건은 아직 발표되지 않았다.
- PCC 없이도 AI 검색은 된다. Mac 안의 AFM이 기본이고(D18), 더 큰 모델이 필요하면 외부 LLM을 쓸 수 있다(D8). PCC가 더하는 것은 비용 없이 쓰는 더 큰 모델(문맥 창 32,768 토큰)이다.

### 출처

- [SystemLanguageModel](https://developer.apple.com/documentation/foundationmodels/systemlanguagemodel), [contextSize](https://developer.apple.com/documentation/foundationmodels/systemlanguagemodel/contextsize)
- [TN3193: Managing the on-device foundation model's context window](https://developer.apple.com/documentation/technotes/tn3193-managing-the-on-device-foundation-model-s-context-window)
- [WWDC26: What's new in the Foundation Models framework](https://developer.apple.com/videos/play/wwdc2026/241/), [WWDC26: LLM search using Core Spotlight](https://developer.apple.com/videos/play/wwdc2026/246/)
- [NLContextualEmbedding](https://developer.apple.com/documentation/naturallanguage/nlcontextualembedding), [Finding similarities between pieces of text](https://developer.apple.com/documentation/naturallanguage/finding-similarities-between-pieces-of-text)
- [Building a search interface for your app (Core Spotlight)](https://developer.apple.com/documentation/corespotlight/building-a-search-interface-for-your-app), [Developer Forums: Core Spotlight semantic search](https://developer.apple.com/forums/thread/793867)
- [OpenAI SDK compatibility (Claude API)](https://platform.claude.com/docs/en/api/openai-sdk)
- [WWDC26: Build with the new Apple Foundation Model on Private Cloud Compute](https://developer.apple.com/videos/play/wwdc2026/319/), [PrivateCloudComputeLanguageModel](https://developer.apple.com/documentation/foundationmodels/privatecloudcomputelanguagemodel), [Accessing Private Cloud Compute](https://developer.apple.com/private-cloud-compute/)
- [Introducing Apple's On-Device and Server Foundation Models (2024)](https://machinelearning.apple.com/research/introducing-apple-foundation-models)
- [Introducing the Third Generation of Apple's Foundation Models (2026)](https://machinelearning.apple.com/research/introducing-third-generation-of-apple-foundation-models)
- [App Store Small Business Program](https://developer.apple.com/app-store/small-business-program/), [Supported capabilities (macOS)](https://developer.apple.com/help/account/reference/supported-capabilities-macos/)
- [App Store Connect 도움말: 세금 양식 개요](https://developer.apple.com/kr/help/app-store-connect/provide-tax-information/tax-forms-overview/), [App Store Connect 도움말: 작업 흐름](https://developer.apple.com/kr/help/app-store-connect/get-started/app-store-connect-workflow/)
- [Chroma: Evaluating Chunking Strategies for Retrieval (2024)](https://www.trychroma.com/research/evaluating-chunking), [Chunk Twice, Embed Once (2025)](https://arxiv.org/html/2506.17277v1), [PruningRAG (2024)](https://arxiv.org/pdf/2409.13694), [CRUD-RAG (2024)](https://arxiv.org/pdf/2401.17043)

## Decisions

> 각 결정의 "근거"에는 요청자의 말을 그대로 옮겼다. P로 시작하는 항목은 Claude의 제안이다. 요청자가 승인한 제안에는 "승인" 줄이 있고, 그 줄이 없는 제안은 아직 승인되지 않았다. "바뀜", "덧붙임", "확인", "없어짐"은 뒤에 나온 결정에 따라 덧붙인 것이다.

### D1. 맥락 검색을 추가하고, AFM으로 한다

- 내용: 글자 그대로 찾는 지금의 검색에 더해, 질의의 뜻으로 찾는 AI 검색(맥락 검색)을 추가한다. AI 검색에는 Apple의 Foundation Models 프레임워크가 제공하는 언어 모델(AFM)을 쓴다. 지금의 검색은 keyword 검색이라고 부른다.
- 근거: "키워드 검색말고 LLM 기반의 맥락 검색이 기능을 추가하는" / "LLM 기반은 Apple Foundation Model (AFM)"
- 덧붙임(D8): AFM 대신 외부 LLM도 쓸 수 있다.
- 덧붙임(D15): AFM은 Mac 안의 모델과 PCC의 모델 가운데 고를 수 있다.

### D2. 설정으로 AI 검색을 켜고 끈다. 기본은 켬

- 내용: 설정에 AI 검색 켬/끔을 두고, 기본값은 켬이다. 켜져 있으면 검색 필드에 아무 표시 없이 AI 검색을 한다. 꺼져 있으면 keyword 검색을 한다. 이것은 검색 필드에 토큰(D4)이 없을 때의 방식, 곧 기본 방식이다.
- 근거: "설정에 AI 검색이 사용 on/off 넣고 기본은 on. 이 경우 검색창에 별도 표시없이 AI 기반 검색 활성화 (off시 keyword 검색 기본)"
- 바뀜(D5, D6): 켬/끔 설정은 없앤다. AI 검색은 `/ai`로만 하고, 기본 방식은 언제나 keyword 검색이다.

### D3. 검색 필드의 명령으로 검색 방식을 고른다

- 내용:

| 명령 | 검색 방식 |
|---|---|
| `/ai` | AI 검색 |
| `/key` | keyword 검색 |
| `/match` | 정규식 검색, 대소문자 무시 |
| `/case` | 정규식 검색, 대소문자 구분 |

- 근거: "검색창에 /ai 입력으로 AI 기반 검색, /key 입력으로 keyword 기반 검색, /match 입력으로 regex 기반 검색 (case-insensitive), /case 입력으로 regex 기반 검색 (case-sensitive)"
- 확인: "/case 검색은 regular expression에 대문자 소문자 구분이라는 뜻임. /match는 regular expression에 대문자 소문자 구분하지 않음". 위의 표와 같다.

### D4. 명령 뒤에 스페이스를 누르면 토큰이 되고, 토큰을 지우면 기본 방식으로 돌아간다

- 내용: 검색 필드에 명령을 쓰고 스페이스를 누르면, 명령이 `/`를 뺀 이름(`ai`, `key`, `match`, `case`)을 둥근 테두리와 배경으로 감싼 토큰으로 바뀐다. 토큰이 있는 동안 그 검색 방식을 쓴다. 토큰을 지우면 기본 방식(D2)으로 돌아간다.
- 근거: "검색창에 /ai 등을 입력하고 스페이스를 누르면 해당 기능 활성화 의미로 ai에 둥근 테두리 배경으로 둘러쌓인 블럭으로 변경 (/ 삭제). 해당 블럭 삭제함으로써 기능 리셋"
- 해석: "기능 리셋"은 기본 방식(D2)으로 돌아간다는 뜻으로 읽었다.
- 바뀜(D6): 기본 방식은 언제나 keyword 검색이므로, 토큰을 지우면 keyword 검색으로 돌아간다.

### D5. AI 검색 켬/끔 설정을 없앤다

- 내용: 설정에서 AI 검색 켬/끔을 없앤다. AI 검색은 쓸 수 있으면 언제나 쓸 수 있다. D2를 대신한다.
- 근거: "생각해보니 아무래도 ai 검색을 기본으로 하는 건 아닌 거 같아." / "설정에서 ai 검색 활성화 on/off 제거 (가능한 경우 항상 활성화)"

### D6. AI 검색은 `/ai`로만 한다

- 내용: LLM으로 찾는 것은 검색 필드에 `/ai`를 입력했을 때(`ai` 토큰이 있을 때)뿐이다. 토큰이 없으면 keyword 검색을 한다.
- 근거: "검색창에 /ai 입력했을 때만 llm 기반 검색"

### D7. keyword 검색만 입력하는 동안 검색하고, 나머지는 Return을 눌러야 검색한다

- 내용: keyword 검색은 Return을 누르지 않아도 입력하는 동안 검색한다(지금과 같다). AI 검색과 정규식 검색(`/match`, `/case`)은 Return을 누른 뒤에 검색한다.
- 근거: "keyword 검색일 때는 enter 치지 않아도 자동 검색. 그 외는 enter 입력 후 검색"

### D8. 설정에 AI 항목을 두고, AFM 대신 외부 LLM을 쓸 수 있게 한다

- 내용: 설정 창의 사이드바에 General과 함께 AI 항목을 둔다. AI 항목에서 AFM 대신 외부 LLM을 쓰도록 할 수 있다. 외부 LLM의 설정 값은 URL, 모델 이름, 모델 옵션, API key이고, 기본값은 없다.
- 근거: "사이드바에 general 에 추가로 AI 설정을 넣고, AFM 대신 외부 LLM 사용할 수 있도록 함. url, model 이름, model option, api key (기본 없음)"
- 해석:
  - 지금 설정 창에는 사이드바도 General도 없다(Current specification). 그래서 설정 창에 사이드바를 새로 두고, 지금의 Fonts, Colors, Margins는 General에 넣는다는 뜻으로 읽었다.
  - "(기본 없음)"은 네 값이 모두 처음에는 비어 있고, 그래서 처음에는 AFM을 쓴다는 뜻으로 읽었다.
- 바뀜(D10): General은 두지 않는다. 사이드바 항목은 Fonts, Colors, Margins, AI다.
- 확인(D18): 기본 모델은 Mac 안의 AFM이다. 둘째 해석과 맞는다.

### D9. 설정 창의 사이드바에 검색 필드를 둔다

- 내용: 설정 창의 사이드바에 검색 필드를 둔다. 검색어를 입력하면 그 검색어를 가진 설정만 보여 준다.
- 근거: "사이드바에 검색창을 두고 검색하는 경우는 해당 키워드를 가진 설정만 보여주는 것으로"

### D10. 설정은 한 목록으로 늘어놓고, 사이드바로 그 위치에 바로 간다

- 내용: 설정을 항목마다 다른 화면으로 나누지 않고 한 목록으로 늘어놓는다. 사이드바의 항목을 고르면 목록에서 그 위치로 바로 옮겨 간다. 사이드바 항목은 Fonts, Colors, Margins, AI이고, General은 두지 않는다.
- 근거: "설정은 완전히 따로 분리해서 보여주는 것보다는 나열식으로 보여주되, 사이드바에 있는 항목을 선택하면 그 위치로 바로 이동하는 방식. 따라서 General이라고 할만한 항목이 없다면 그냥 Fonts, Colors, Margins, AI로 두면 될 듯"
- D8의 첫째 해석(General에 지금의 설정을 넣음)과 P12의 사이드바 구성을 대신한다.

### D11. 모델은 라디오 버튼으로 고르고, 외부 LLM일 때만 입력란을 쓸 수 있다

- 내용: AFM과 외부 LLM 가운데 하나를 라디오 버튼으로 고른다. 외부 LLM을 골랐을 때만 외부 LLM의 입력란을 쓸 수 있다.
- 근거: "AFM, 외부 LLM: 선택 할 수 있도록 라디오 버튼 제공 (외부 LLM을 선택했을 때만 입력창 활성화)"
- P12의 고르는 방법(고르는 칸)을 정한다. P12의 대안(URL이 있으면 외부 LLM)은 쓰지 않는다.

### D12. 외부 LLM은 OpenAI 호환 형식만 지원한다

- 내용: 외부 LLM은 OpenAI 호환 Chat Completions 형식으로만 부른다. Anthropic Messages API 형식은 지원하지 않는다.
- 근거: "OpenAI compatible만"

### D13. 모델 옵션은 키와 값을 줄마다 입력하고, 부를 때 JSON으로 보낸다

- 내용: 모델 옵션은 키 입력란과 값 입력란을 한 줄로 하고, 줄을 더하고 뺄 수 있게 한다. 모델을 부를 때 이 줄들을 JSON으로 만들어 요청에 넣는다.
- 근거: "model options 부분은 key - value 입력창 (추가 제거 가능) -> 모델 호출시 json 형식으로"
- P11의 "모델 옵션은 JSON 객체(글로 입력)"를 대신한다. 값의 형식은 P18.

### D14. 묶음은 서로 겹치게 나눈다

- 내용: 본문을 묶음으로 나눌 때 이웃한 묶음이 일부 겹치게 한다. 묶음의 경계에서 문맥이 끊겨 검색 결과가 이상해지지 않게 하기 위해서다.
- 근거: "4096씩이라면 너무 느릴 것 같은데? 더군다나 4096씩 자르면 경계 검색은 이상하니가 window를 두고 겹쳐야하잖아"
- 겹치는 크기는 D17, 겹치는 방법은 P19.
- 덧붙임: 요청자가 겹침이 꼭 필요한지 조사하라고 했다(D20). 조사는 Investigation의 "겹침이 필요한가", Claude의 제안은 P20이다. 필요하면 모든 모델에 쓴다(D20).
- 바뀜(D21): 묶음 사이는 겹치지 않는다. 블록 하나가 한 묶음보다 길어 자를 때만 겹친다.

### D15. Private Cloud Compute도 고를 수 있게 한다

- 내용: AI 검색의 모델로 Private Cloud Compute(PCC)에서 도는 Apple Foundation Model도 고를 수 있게 한다.
- 근거: "Private Cloud Compute도 선택할 수 있도록 한다."
- 해석: D11의 라디오 버튼은 Mac 안의 AFM, PCC, 외부 LLM의 셋이 된다.
- P2의 "Private Cloud Compute 같은 서버 모델은 쓰지 않는다"를 대신한다.
- 주의: PCC는 macOS 27 이상에서만 쓸 수 있고, 개발자 자격과 entitlement가 필요하다(Investigation). Open questions 1.
- 덧붙임(조사): 이 Mac에서 확인한 계정 상태는 Investigation의 "PCC 계정 확인"에 있다. 지금은 이 앱에 PCC를 넣을 수 있다고 확인되지 않았다. Claude의 제안은 P23, Open questions 1.
- 덧붙임(D26): 요청자가 PCC entitlement를 신청하고, PCC는 이 요청에 남는다. P23은 쓰지 않는다.
- 없어짐(D31): PCC는 이 요청에서 빠진다.

### D16. API key는 입력란에 두고, 기본으로 가린다

- 내용: API key는 Keychain이 아니라 다른 설정 값처럼 설정에 저장한다. 입력란은 기본으로 글자를 `*`로 가리고, 토글로 보이게 할 수 있다.
- 근거: "API key를 keychain에 두는 건 과하다. 그냥 입력필드에 넣게하고 기본은 *처리. 토글로 볼 수 있게 하자."
- P13을 대신한다.

### D17. 외부 LLM은 한 번에 보낼 토큰의 최대치와 겹침을 정할 수 있다

- 내용:
  - 외부 LLM이 한 번에 볼 수 있는 토큰의 최대치를 정할 수 있다. 슬라이더로 32k, 64k, 138k, 256k 같은 값을 고르거나, 입력란에 직접 입력한다.
  - 묶음이 얼마나 겹칠지도 정할 수 있다.
  - 기본값은 256이다.
- 근거: "외부 LLM은 한번에 볼 수 있는 토큰 최대치를 입력할 수 있도록 한다. 32k 64k, 138k, 256k등 슬라이드로 선택할 수 있도록 하고, 입력창으로 직접 입력할 수도 있게 한다. 겹침 오버랩을 얼마로 할지도 지정할 수 있도록 한다. 기본 256"
- 해석(확인 필요): "기본 256"은 바로 앞 문장의 겹침에 대한 것으로 보고 겹침의 기본값을 256 토큰으로 읽었다. "138k"는 흔한 크기인 128k를 뜻하는 것으로 보인다. Open questions 2.
- P14의 "외부 LLM에도 AFM과 같은 크기로 나눠 보낸다"를 대신한다.
- 확인: "겹침 256을 기본으로 하라는 건데". 겹침의 기본값은 256 토큰이다(해석이 맞다). "138k"의 뜻과 최대치의 기본값은 아직 정하지 않았다.
- 바뀜(D21, D22, D24): 겹침은 설정에서 정하지 않고, 긴 블록을 자를 때 256 토큰으로 쓴다. 최대치의 기본값은 32K다. 슬라이더 눈금은 P19대로이므로 "138k" 대신 128K가 들어간다.

### D18. 기본 모델은 Mac 안의 AFM이다

- 내용: AI 검색의 기본 모델은 Mac 안의 AFM이다. PCC와 외부 LLM은 사용자가 골랐을 때만 쓴다.
- 근거: "LLM 기본은 그냥 AFM이야."
- D8의 둘째 해석(처음에는 AFM을 쓴다)과 맞는다.
- 덧붙임(D23): macOS 26에서는 Mac 안의 AFM을 쓸 수 없으므로, 외부 LLM을 설정하기 전까지 AI 검색은 꺼져 있다.
- 바뀜(D25): macOS 26에서도 Mac 안의 AFM을 쓴다. 위의 덧붙임은 없어진다.

### D19. PCC나 외부 LLM을 쓰면 데이터가 밖으로 나갈 수 있음을 알린다

- 내용: PCC나 외부 LLM을 고르면, 검색하는 문서의 데이터가 Mac 밖으로 나갈 수 있다는 것을 사용자에게 알린다.
- 근거: "PCC나 외부 LLM은 데이터가 외부로 나갈 수 있음을 고지해야됨"
- 알리는 방법은 P21.
- 바뀜(D31): PCC가 빠졌으므로 외부 LLM에만 해당한다.

### D20. 겹침이 필요하면 모든 모델에 쓴다

- 내용: 겹침이 필요하다면 Mac 안의 AFM, PCC, 외부 LLM 모두에 쓴다. 겹침이 꼭 필요한지는 먼저 조사한다.
- 근거: "겹침 256을 기본으로 하라는 건데.. 겹침이 필요하긴 해? 이것부터 조사해줘. 필요하다면 모든 모델에서 사용해야지"
- 겹침을 모든 모델에 쓸지(P17의 대안과 Open question)는 이것으로 정해졌다. P17의 대안(외부 LLM에서만 정함)은 쓰지 않는다.
- 조사는 Investigation의 "겹침이 필요한가", Claude의 제안은 P20이다.

### D21. 겹침은 P20대로 한다

- 내용: P20을 승인한다. 묶음 사이는 겹치지 않고, 묶음은 블록의 경계에서 나눈다(묶음이 절반 넘게 찬 뒤에 제목이 나오면 그 제목에서 새 묶음을 시작한다). 블록 하나가 한 묶음보다 길 때만 256 토큰씩 겹치게 자르고, 모든 모델이 같다(D20). 설정에는 Overlap 줄이 없다.
- 근거: "겹침: P20 제안대로 해줘"
- D14의 "이웃한 묶음이 일부 겹치게 한다"와 D17의 "겹침을 정할 수 있다"를 대신한다. 겹침의 크기 256 토큰(D17의 확인)은 긴 블록을 자를 때 쓴다.

### D22. 외부 LLM 최대치의 기본값은 32K다

- 내용: 외부 LLM이 한 번에 볼 수 있는 토큰 최대치의 기본값은 32K(P19의 K = 1,024이므로 32,768 토큰)다.
- 근거: "외부 LLM max token 기본값은 32k"
- P19의 제안과 같다.

### D23. Mac 안의 AFM은 macOS 27부터 쓰고, macOS 26에서는 외부 LLM이 없으면 AI 검색을 끈다

- 내용: Mac 안의 AFM은 macOS 27 이상에서만 쓴다. macOS 26에서는 외부 LLM을 설정했을 때만 AI 검색을 쓸 수 있고, 설정하지 않았으면 AI 검색은 꺼져 있다. PCC는 원래 macOS 27 이상에만 있다.
- 근거: "AFM은 macOS 27부터 적용. 따라서 macOS 26에서는 외부 LLM 설정 없이는 기능을 꺼두어야함"
- 이유: Investigation의 "macOS 26과 macOS 27의 AFM". macOS 26의 Mac 안 모델은 2025년 모델이고 문맥 창이 4,096 토큰이다.
- 끄는 방법은 P22.
- 바뀜(D25): Mac 안의 AFM은 macOS 26에서도 쓴다. macOS 26에서 AI 검색을 끄는 규칙은 없어진다. PCC는 그대로 macOS 27 이상에만 있다.

### D24. 나머지 제안은 제안대로 한다

- 내용: 아직 승인되지 않았던 제안 P1–P8, P10, P11, P15–P19, P21을 제안대로 한다. 각 제안의 대안은 쓰지 않는다. 제안 가운데 뒤의 결정으로 바뀐 부분은 그 결정을 따른다(P2는 D8·D15·D23, P3은 D7, P5는 D5·D6·D8·D15·D23, P7은 D7, P11은 D12·D13, P17은 D18–D21·D23, P19는 D21·D22).
- 근거: "나머지는 제안대로면 될 듯"
- 이것으로 D17의 "138k"는 P19의 눈금(128K)으로 정해졌다. P9, P12, P13, P14는 이미 없어지거나 바뀌었으므로 해당하지 않는다. 이 결정 뒤에 나온 제안(P22, P23)은 아직 승인되지 않았다.

### D25. Mac 안의 AFM은 macOS 26에서도 쓴다

- 내용: 성능이 떨어지더라도 macOS 26에서도 Mac 안의 AFM(2025년 모델, 문맥 창 4,096 토큰)을 쓴다. PCC는 macOS 27 이상에만 있으므로 그대로다. D23의 "Mac 안의 AFM은 macOS 27부터"와 "macOS 26에서는 외부 LLM이 없으면 AI 검색을 끈다"를 대신한다.
- 근거: "미안한데, 성능이 떨어지더라도 macOS 26에서의 AFM도 쓰는 것이 좋겠다."
- P22는 필요 없어진다.

### D26. PCC entitlement를 신청하고, PCC는 이 요청에 남긴다

- 내용: 요청자가 Apple에 PCC entitlement를 신청한다. PCC는 이 요청에 남긴다(D15).
- 근거: "PCC 신청하는 것으로 (많이 공유될 앱이 사실 아니라서, 맘껏 쓸 수 있을 듯)"
- 해석: P23(PCC를 이 요청에서 뺌)은 쓰지 않는다. entitlement를 받기 전이나 PCC를 쓸 수 없는 동안에는 PCC 선택지를 흐리게 표시하고 이유를 보여 주는 P23의 대안을 따르는 것으로 읽었다. 그 세부는 P24다.
- 주의(Investigation): 사용자마다 하루 한도는 앱의 사용자 수와 관계없이 있다. Developer ID 배포에서 PCC를 쓸 수 있는지는 신청 결과로 알게 된다. 쓸 수 없다는 답이 오면 §10과 충돌하므로, 그때 Claude는 멈추고 요청자에게 묻는다(§11.3 단계 2의 5, 단계 3의 2).
- 확인(D27): 요청자는 PCC를 쓰려고 한다. PCC를 이 요청에 남기는 것은 맞다. 흐리게 표시하는 세부(P24)는 아직 승인되지 않았다.
- 확인(D30): 흐리게 표시하는 세부는 P24대로 한다.
- 바뀜(D31): PCC를 이 요청에서 빼고, entitlement도 신청하지 않는다.

### D27. macOS 26에서도 PCC를 쓴다

- 내용: macOS 26에서도 PCC를 쓰려고 한다.
- 근거: "macOS 26의 PCC도 쓸려고 해."
- 확인(Investigation): 지킬 수 없다. 앱이 PCC를 부르는 API는 macOS 27.0부터만 있고, macOS 26에서 쓰게 하는 길도 없다. macOS 26에서 PCC는 Apple의 시스템 기능만 쓴다. Open questions 1.
- 바뀜(D29): macOS 26에서는 PCC를 쓰지 않고, 선택지를 흐리게 표시한다.
- 없어짐(D31): PCC가 빠졌다.

### D28. P25대로 한다

- 내용: P25를 승인한다. PCC용 provisioning profile은 `packaging/`에 두고 커밋하며, 릴리스 워크플로의 secret은 늘리지 않는다. `cargo xtask bundle`은 profile이 있으면 번들에 넣고 entitlement로 서명하고, 없으면 지금처럼 서명한다.
- 근거: "P25 제안대로"
- 없어짐(D31): PCC가 빠졌으므로 provisioning profile과 entitlement 서명도 필요 없다.

### D29. macOS 26에서는 PCC를 흐리게 표시한다

- 내용: macOS 26에서는 AI 묶음의 PCC 선택지를 흐리게 표시하고 "Requires macOS 27 or later."를 보여 준다(P24). macOS 26에서 Mac 밖의 모델을 쓰려면 외부 LLM을 쓴다. D27을 대신한다.
- 근거: Open questions 1(D27을 지킬 수 없으니 macOS 26에서는 P24대로 흐리게 표시할지)에 대한 답 "1. 제안대로"
- 없어짐(D31): PCC가 빠졌다.

### D30. P24대로 한다

- 내용: P24를 승인한다. macOS 26, PCC entitlement가 없는 빌드, PCC를 지원하지 않는 기기에서는 PCC 선택지를 흐리게 표시하고 이유를 보여 준다. 준비 안 됨, 하루 한도, 연결 실패, 서비스 중단 같은 일시적인 문제는 흐리게 표시하지 않고, 검색할 때 "AI unavailable"과 이유를 보여 준다.
- 근거: Open questions 2(P24의 나머지)에 대한 답 "2. P24도 제안대로"
- 없어짐(D31): PCC가 빠졌다.

### D31. PCC는 이 요청에서 뺀다

- 내용: P26을 승인한다. PCC를 이 요청에서 빼고, PCC entitlement도 신청하지 않는다. AI 묶음의 라디오 버튼은 "Apple Foundation Model (on this Mac)"과 "External LLM" 둘이다. PCC에만 필요하던 결정과 제안(D15, D26–D30, P24, P25)은 없어지고, 다른 결정과 제안에서 PCC를 말한 부분도 빠진다. 데이터가 Mac 밖으로 나간다는 확인 창과 안내(D19, P21)는 외부 LLM에만 남는다. `doc/DEVELOPMENT/`에 "PCC 적용" 주제를 남긴다.
- 근거: "PCC 빼.. 하기 어렵고 곤란하다"
- 이유: Investigation의 "Small Business Program과 사업자 등록"과 P26.

### P1. (Claude의 제안) Embedding과 DB 없이, AFM이 본문을 직접 읽어 고른다

- 내용:
  - AI 검색은 Embedding, 색인, DB를 쓰지 않는다. 문서 내용과 검색 결과를 파일로 남기지 않는다.
  - 본문을 블록(§6의 블록)으로 나누고, 블록마다 번호와 그 블록이 속한 제목을 붙인다. 문맥 창(`contextSize`)에 들어가는 만큼씩 묶어 질의와 함께 AFM에 넘기고, 질의와 관련 있는 블록의 번호를 받는다.
  - 블록 하나가 한 묶음보다 길면 나눠서 넘기고, 어느 조각이든 관련 있으면 그 블록을 결과로 한다.
- 근거: 요청 3의 질문 "Embedding이 필요할까? RAG 방식을 쓰려면 매번 DB 구축을 해야할까? AFM에는 Embedding 지원이 있어?" 요청자의 결정은 아직 없다.
- 이유: Investigation. AFM에는 Embedding이 없고, Apple의 다른 Embedding은 한국어나 한·영 혼합에 맞지 않으며, 검색 범위가 문서 하나라 DB가 필요 없다.
- 대안: Investigation의 마지막 항목(질의를 넓혀 고른 뒤 상위 블록만 판정).
- 덧붙임(D8): 외부 LLM을 쓸 때도 같은 방법으로 고른다. 묶음 크기는 P14.
- 덧붙임(D14, D15, D17): PCC에도 같은 방법을 쓴다. 이웃한 묶음은 겹친다. 묶음의 크기와 겹치는 방법은 P19를 따른다.
- 승인(D24). 대안은 쓰지 않는다. 겹침은 D21(P20)을 따른다.

### P2. (Claude의 제안) Mac 안의 모델만 쓴다

- 내용: AI 검색은 Mac 안의 모델(`SystemLanguageModel.default`)만 쓴다. Private Cloud Compute 같은 서버 모델은 쓰지 않는다.
- 이유: 문서 내용이 Mac 밖으로 나가지 않고, 네트워크가 없어도 동작한다.
- 바뀜(D8): 요청자가 설정한 외부 LLM은 쓴다. 그때는 검색하는 문서의 글과 질의가 그 주소로 보내진다. AFM을 쓸 때는 이 제안대로 Mac 안의 모델만 쓴다.
- 바뀜(D15): PCC도 고를 수 있다. PCC를 고르면 문서의 글과 질의가 Apple의 PCC 서버로 간다. Mac 안의 AFM을 고르면 문서는 Mac 밖으로 나가지 않는다.
- 승인(D24): 바뀐 내용대로 한다. Mac 안의 AFM은 macOS 27 이상에서만 쓴다(D23).
- 바뀜(D25): Mac 안의 AFM은 macOS 26에서도 쓴다.
- 바뀜(D31): PCC는 빠졌다. Mac 밖으로 나가는 것은 외부 LLM을 고른 경우뿐이다.

### P3. (Claude의 제안) AI 검색은 Return으로 시작한다

- 내용:
  - AI 방식에서는 입력하는 동안 검색하지 않는다. Return(또는 ⌘G, 다음·이전 버튼)을 누르면 검색을 시작한다. 같은 질의로 이미 검색했으면 지금처럼 다음 결과로 간다(Shift-Return, ⇧⌘G는 이전).
  - 검색하는 동안 카운터에 "Searching…"을 보여 주고, 결과는 찾는 대로 강조한다.
  - 질의를 고치거나, 토큰을 지우거나, 검색 막대를 닫으면 진행 중인 검색을 멈춘다.
- 이유: AFM은 한 번 묻는 데 글자 하나를 입력하는 시간보다 훨씬 오래 걸리므로, 입력할 때마다 검색하면 쓸모없는 검색이 쌓인다.
- 바뀜(D7): Return을 눌러야 검색하는 것은 요청자가 정했고, 정규식 검색에도 적용된다. 나머지(⌘G와 다음·이전 버튼도 검색을 시작함, AI 검색의 "Searching…", 멈추는 경우)는 그대로 제안이다.
- 덧붙임(제안): 묶음이 여럿이면 "Searching… 2/5"처럼 끝난 묶음 수를 보여 준다.
- 승인(D24).

### P4. (Claude의 제안) AI 검색의 결과는 블록이고, 문서 순서로 놓는다

- 내용:
  - 결과 하나는 AFM이 질의와 관련 있다고 고른 블록 하나다. 블록 전체를 keyword 검색과 같은 방법으로 강조한다.
  - 결과는 문서 순서로 놓는다. 카운터("3 of 12", "Not found"), 이동, 끝에서 처음으로 돌기, 접힌 부분 펼치기, Esc/Done 뒤의 선택은 keyword 검색과 같다.
- 대안: 관련도 순서로 놓기, 블록 안에서 관련 있는 구절만 강조하기.
- 승인(D24). 대안은 쓰지 않는다.

### P5. (Claude의 제안) AFM을 쓸 수 없으면 keyword 검색이 기본이다

- 내용:
  - AFM을 쓸 수 없으면(Apple Intelligence가 꺼짐, 지원하지 않는 Mac, 모델 준비 중) 설정이 켜져 있어도 기본 방식은 keyword 검색이다. AFM을 쓸 수 있게 되면 AI 검색으로 돌아간다.
  - 설정 창의 AI search는 그대로 켜고 끌 수 있고, 옆에 쓸 수 없는 이유를 보여 준다.
  - `/ai` 토큰은 만들 수 있다. 검색하면 카운터에 "AI unavailable"을 보여 주고 아무것도 강조하지 않는다.
  - AFM이 일부 묶음을 처리하지 못하면(지원하지 않는 언어, 안전 장치의 거부) 나머지 묶음의 결과만 보여 준다. 모든 묶음이 실패하면 "AI unavailable"을 보여 준다.
- 바뀜(D5, D6, D8):
  - 기본 방식이 언제나 keyword이고 켬/끔 설정이 없으므로, 첫째와 둘째 항목은 없어진다. 셋째와 넷째 항목은 남는다.
  - AI를 쓸 수 없는 경우에 외부 LLM의 문제가 더해진다: URL이 비어 있음, 연결 실패, 인증 실패, 한도 초과, 시간 초과(P14).
  - 쓸 수 없는 이유는 카운터에 마우스를 올리면 보인다. AFM의 상태는 설정 창의 AI 화면에도 보인다(P12).
- 덧붙임(D15): PCC를 쓸 수 없는 경우가 더해진다: macOS 26, 지원하지 않는 기기(`deviceNotEligible`), 준비 안 됨(`systemNotReady`), 하루 한도에 닿음(`quotaLimitReached`, 다시 쓸 수 있는 시각을 함께 보여 줌), 연결 실패(`networkFailure`), 서비스 중단(`serviceUnavailable`). 상태는 설정 창의 AI 묶음에 보인다(P17).
- 덧붙임(D23): macOS 26에서 외부 LLM이 설정되지 않은 경우가 더해진다(P22).
- 승인(D24).
- 바뀜(D25): D23의 덧붙임은 없어진다. macOS 26에서도 Mac 안의 AFM을 쓸 수 없는 경우(Apple Intelligence가 꺼짐 등)는 첫 내용대로다.
- 덧붙임(D26): PCC를 쓸 수 없는 경우에 "앱에 PCC entitlement가 없음"이 더해진다(P24).
- 바뀜(D31): PCC가 빠졌으므로 PCC를 쓸 수 없는 경우(D15, D26의 덧붙임)는 없어진다. 남는 것은 Mac 안의 AFM과 외부 LLM을 쓸 수 없는 경우다.

### P6. (Claude의 제안) 토큰의 세부 동작

- 내용:
  - 명령은 검색 필드의 맨 앞에서, 토큰이 없을 때만 알아본다. 대소문자는 가리지 않는다(`/AI`도 된다).
  - 토큰은 하나만 있다. 토큰 뒤에 쓴 명령과 네 명령이 아닌 것(예: `/foo`)은 일반 글자로 찾는다.
  - 토큰 바로 뒤에서 Delete 키를 누르면 토큰이 지워진다. 검색 필드의 지우기 버튼은 토큰과 글자를 함께 지운다.
  - 토큰을 지워도 뒤의 글자는 남고, 기본 방식으로 다시 검색한다(AI 방식이면 P3에 따라 Return을 기다린다).
  - 토큰과 글자는 검색 막대를 닫았다 열어도 남는다(지금 검색어가 남는 것과 같다). ⌘F로 검색 막대를 열면 토큰 뒤의 글자만 선택된다.
  - 토큰이 없을 때 검색 필드는 지금과 같다(자리 표시 글자 "Find"). 기본 방식이 무엇인지 따로 표시하지 않는다(D2).
- 바뀜(D6): 기본 방식이 keyword이므로, 토큰을 지우면 남은 글자를 바로 keyword로 다시 검색한다.
- 승인(D24).

### P7. (Claude의 제안) 정규식 검색의 규칙

- 내용:
  - 문법은 JavaScript 정규식(유니코드 모드)이다. `/match`는 대소문자를 무시하고 `/case`는 구분한다.
  - 일치는 블록을 넘지 않는다. `^`와 `$`는 블록의 처음과 끝에 맞는다.
  - 문서의 공백 문자는 모두 공백으로 보고 찾는다. 그래서 식의 공백은 keyword 검색처럼 모든 공백 문자에 맞는다.
  - 길이가 0인 일치(예: `x*`가 빈 문자열에 맞는 것)는 결과로 치지 않는다.
  - keyword 검색처럼 입력하는 동안 검색한다. 식이 잘못되었으면 카운터에 "Invalid pattern"을 보여 주고 강조를 지운다.
- 이유: 검색을 페이지 스크립트(WebKit)가 하므로 JavaScript 정규식을 그대로 쓸 수 있다.
- 바뀜(D7): 정규식 검색은 Return을 누르면 검색한다. "Invalid pattern"은 Return을 누를 때 보여 준다.
- 승인(D24).

### P8. (Claude의 제안) Match Case는 keyword 방식에서만 쓴다

- 내용: Match Case는 keyword 방식에서만 쓴다. AI 방식과 정규식 방식에서는 흐리게 표시하고 누를 수 없게 한다. 정규식의 대소문자는 명령(`/match`, `/case`)이 정한다.
- 대안: Match Case를 없애고, keyword 검색은 늘 대소문자를 무시한다.
- 승인(D24). 대안은 쓰지 않는다.

### P9. (Claude의 제안) 설정 창에 Search 묶음을 둔다

- 내용: 설정 창에 네 번째 묶음 Search를 두고 체크박스 "AI search"(기본 켬)를 넣는다. 바꾸면 열린 모든 창에 바로 적용되고 저장된다(§9.1). 토큰이 없는 검색 필드의 방식이 바뀐다.
- 없어짐(D5): 켬/끔 설정이 없어졌다. 설정 창은 P12를 따른다.

### P10. (Claude의 제안) 다른 검색 기능과의 관계

- 내용:
  - ⌘E(Use Selection for Find)는 지금의 토큰을 그대로 두고 선택한 글자로 검색한다. 그때 방식이 AI면 바로 AI 검색을 시작한다.
  - 문서가 다시 읽히면(§4.4) 모든 방식에서 새 내용으로 다시 검색한다. 진행 중인 AI 검색은 멈추고 새로 시작한다.
- 덧붙임(D7): ⌘E는 모든 방식에서 바로 검색한다(Return을 누른 것과 같다).
- 승인(D24).

### P11. (Claude의 제안) 외부 LLM은 OpenAI 호환 형식으로 부른다

- 내용:
  - 외부 LLM은 OpenAI 호환 Chat Completions 형식으로 부른다. 설정의 URL로 `POST` 요청을 보낸다. URL은 요청을 받는 주소 전체다(예: `https://api.openai.com/v1/chat/completions`, `http://localhost:11434/v1/chat/completions`). `http`와 `https`를 모두 받는다.
  - 모델 이름은 요청 본문의 `model`이 된다.
  - 모델 옵션은 JSON 객체이고, 그 키와 값을 요청 본문에 그대로 더한다(예: `{"temperature": 0}`). 비어 있으면 아무것도 더하지 않는다. JSON 객체가 아니면 설정 창에 오류를 보여 주고 저장하지 않는다.
  - API key가 있으면 `Authorization: Bearer <API key>` 헤더로 보낸다. 비어 있으면 이 헤더를 보내지 않는다(로컬 서버).
- 이유: 네 값만으로 OpenAI, Ollama 같은 로컬 서버, 여러 게이트웨이를 쓸 수 있다. Claude도 Anthropic의 OpenAI 호환 주소로 쓸 수 있다(Investigation).
- 대안: Anthropic Messages API 형식도 지원하고, 설정에 형식 선택을 더한다.
- 바뀜(D12, D13): OpenAI 호환 형식만 쓰는 것은 요청자가 정했고, 대안은 쓰지 않는다. 모델 옵션은 키·값 줄로 받는다(D13, P18). URL을 주소 전체로 받는 것, `Authorization: Bearer` 헤더, `http`와 `https`를 모두 받는 것은 그대로 제안이다.
- 승인(D24): 남은 부분(URL은 주소 전체, `Authorization: Bearer` 헤더, `http`와 `https`).

### P12. (Claude의 제안) 설정 창을 사이드바로 나누고 AI 화면을 둔다

- 내용:
  - 설정 창 왼쪽에 사이드바를 두고 General과 AI 두 항목을 둔다. General에는 지금의 Fonts, Colors, Margins가 그대로 들어간다.
  - AI 화면:
    - Model: Apple Foundation Model(기본)과 External LLM 가운데 하나를 고른다. Colors의 Automatic/Custom, Margins의 Automatic/Fixed와 같은 방식이다.
    - URL, Model name, Model options, API key는 External LLM을 골랐을 때만 고칠 수 있다. 네 값은 처음에 비어 있다(D8).
    - Apple Foundation Model을 고르면 AFM의 상태(쓸 수 있음, Apple Intelligence가 꺼짐, 지원하지 않는 Mac, 모델 준비 중)를 보여 준다.
    - External LLM을 고르면 "AI 검색을 하면 문서의 글과 질의가 이 주소로 보내진다"는 안내를 보여 준다.
  - 바꾼 값은 바로 저장되고 모든 창에 적용된다(§9.1).
- 대안: 고르는 칸 없이, URL이 비어 있으면 AFM을, 채워져 있으면 외부 LLM을 쓴다.
- 바뀜(D10, D11, D15–D17): 사이드바와 목록은 D10과 P16, AI 묶음은 P17을 따른다. 고르는 방법은 D11(라디오 버튼)로 정해졌고 대안은 쓰지 않는다.

### P13. (Claude의 제안) API key는 Keychain에 둔다

- 내용: API key는 설정 JSON(`NSUserDefaults`)이 아니라 macOS Keychain에 저장한다. 입력란은 글자를 가린다. 제거 pkg가 사용자의 설정을 지울 때 이 Keychain 항목도 지운다(§10).
- 이유: 설정 파일은 암호화되지 않은 평문이라 API key를 두기에 맞지 않다.
- 없어짐(D16): API key는 다른 설정과 함께 저장한다. 제거 pkg는 지금처럼 설정을 지울 때 API key도 함께 지운다.

### P14. (Claude의 제안) 외부 LLM에도 AFM과 같은 크기로 나눠 보낸다

- 내용: 외부 LLM에도 한 번에 본문 약 3,500 토큰(글자 수로 어림)씩 나눠 보낸다. 요청 하나가 60초 안에 답하지 않으면 실패로 본다(P5).
- 이유: 외부 모델의 문맥 창 크기는 앱이 알 수 없다. 작은 묶음은 문맥 창이 작은 로컬 모델에서도 동작한다. 대신 문맥 창이 큰 모델에는 요청 수가 늘어난다.
- 대안: 설정에 묶음 크기를 더한다.
- 바뀜(D17): 외부 LLM의 최대치는 사용자가 정한다(대안과 같다). 묶음 크기와 시간 제한은 P19를 따른다.

### P15. (Claude의 제안) 설정 검색의 규칙

- 내용:
  - 검색어는 대소문자를 가리지 않고, 설정 줄의 이름(예: "Font size", "URL")과 선택지의 이름(예: "Automatic", "External LLM")에서 찾는다. 묶음 이름(예: "Colors")과 맞으면 그 묶음 전체를 보여 준다.
  - 맞는 줄이 있는 묶음만 묶음 제목과 함께 보여 주고, 사이드바에도 그 묶음만 남긴다. 맞는 것이 없으면 "No Results"를 보여 준다.
  - 검색어를 지우면 모든 설정이 다시 보인다.
- 승인(D24).

### P16. (Claude의 제안) 설정 목록과 사이드바의 동작

- 내용:
  - 목록은 Fonts, Colors, Margins, AI 순서이고, 묶음마다 제목이 있다. 목록이 창보다 길면 스크롤한다.
  - 사이드바의 항목을 고르면 그 묶음의 제목이 목록의 맨 위에 오도록 스크롤한다.
  - 목록을 스크롤하면 맨 위에 보이는 묶음이 사이드바에서 선택된다.
- 승인(D24).

### P17. (Claude의 제안) 설정 창의 AI 묶음

- 내용:
  - Model: 라디오 버튼 셋 "Apple Foundation Model (on this Mac)"(기본), "Apple Foundation Model (Private Cloud Compute)", "External LLM" [D11, D15].
  - 라디오 버튼 아래에 고른 모델의 상태를 보여 준다. 예: "Apple Intelligence is turned off.", "Private Cloud Compute requires macOS 27 or later.", "Daily limit reached." (P5)
  - External LLM의 줄: URL, Model name, Model options(키·값 줄과 +, − 버튼), API key(가림, 보기 토글), Max tokens(슬라이더와 입력란). External LLM을 골랐을 때만 고칠 수 있다 [D11, D13, D16, D17].
  - Overlap(토큰): 모든 모델의 묶음에 쓰이므로 언제나 고칠 수 있다 [D14, D17]. 기본값은 256이다.
  - PCC나 External LLM을 고르면 "AI 검색을 하면 문서의 글과 질의가 Mac 밖으로 보내진다"는 안내를 보여 준다. PCC는 Apple의 서버, 외부 LLM은 설정한 주소다.
  - 바꾼 값은 바로 저장되고 모든 창에 적용된다(§9.1).
- 대안: Overlap도 External LLM일 때만 고칠 수 있게 하고, Mac 안의 AFM과 PCC는 256 토큰으로 고정한다.
- 바뀜(D18–D20): 기본이 Mac 안의 AFM인 것은 요청자가 정했다(D18). 밖으로 나간다는 것을 알리는 것도 정해졌고(D19), 알리는 방법은 P21이다. Overlap은 모든 모델에 쓰고(D20), 대안은 쓰지 않는다. P20이 승인되면 Overlap 줄은 없어진다.
- 바뀜(D21): Overlap 줄은 없다.
- 덧붙임(D23): macOS 26에서는 Mac 안의 AFM과 PCC를 흐리게 표시한다(P22).
- 승인(D24).
- 바뀜(D25, D26): macOS 26에서 흐리게 표시하는 것은 PCC뿐이다. PCC를 흐리게 표시하는 경우는 P24를 따른다.
- 바뀜(D31): PCC가 빠졌다. 라디오 버튼은 "Apple Foundation Model (on this Mac)"과 "External LLM" 둘이고, 흐리게 표시하는 선택지는 없다. 밖으로 나간다는 안내는 External LLM을 골랐을 때만 보여 준다.

### P18. (Claude의 제안) 모델 옵션 값의 형식

- 내용:
  - 값이 JSON 값으로 읽히면(숫자, `true`, `false`, `null`, 따옴표로 감싼 문자열, 배열, 객체) 그 값으로 보내고, 아니면 문자열로 보낸다. 예: `temperature`와 `0` → `{"temperature": 0}`, `reasoning_effort`와 `low` → `{"reasoning_effort": "low"}`.
  - 키가 빈 줄은 보내지 않는다. 같은 키가 여러 줄에 있으면 마지막 줄을 쓴다.
  - 앱이 정하는 `model`과 `messages`는 옵션으로 바꿀 수 없다.
- 승인(D24).

### P19. (Claude의 제안) 최대치와 겹침의 규칙

- 내용:
  - 묶음은 블록 단위로 만든다(P1). 둘째 묶음부터는 앞 묶음의 끝 블록들을 겹침 크기만큼 앞에 다시 넣어 문맥으로 쓴다. 블록 하나가 묶음보다 길면 겹침 크기만큼 겹치게 잘라서 넘긴다. 여러 묶음의 결과는 합친다(어느 묶음에서든 고른 블록이 결과다).
  - 한 묶음의 본문 크기: Mac 안의 AFM과 PCC는 모델이 알려 주는 문맥 창(`contextSize`)에서 지침과 답의 몫을 뺀 만큼이다. 외부 LLM은 설정한 최대치의 80 %이고, 나머지 20 %는 지침, 답, 토큰 어림의 오차 몫이다.
  - 외부 LLM의 토큰은 글자 수로 어림한다: 한글을 비롯한 중·일·한 문자는 1자에 1토큰, 그 밖의 문자는 4자에 1토큰.
  - 최대치: 슬라이더의 눈금은 8K, 16K, 32K, 64K, 128K, 256K, 512K, 1M이다(K = 1,024). 입력란에는 1,024부터 2,000,000까지 입력할 수 있다. 기본값은 32K다.
  - 겹침: 0 이상이고 한 묶음의 본문 크기의 절반보다 작아야 한다. 기본값은 256 토큰이다(D17의 해석).
  - 외부 LLM의 요청 하나가 5분 안에 답하지 않으면 실패로 본다(P5). 그 전에도 사용자는 검색을 멈출 수 있다(P3).
- 이유: 앱은 외부 모델의 토크나이저를 알 수 없으므로 넉넉하게 어림한다. 최대치의 기본값은 눈금 가운데 많은 모델이 받을 수 있는 작은 값으로 했다.
- 덧붙임(P20): P20이 승인되면 둘째 묶음부터 앞 묶음의 끝 블록을 다시 넣는 겹침은 없어지고, 긴 블록을 자를 때의 겹침(256 토큰 고정)만 남는다. 겹침 값의 범위 규칙도 필요 없어진다.
- 승인(D24): 겹침에 관한 부분은 D21(P20)으로 바뀌었다. 최대치의 기본값 32K는 D22로도 정해졌다.

### P20. (Claude의 제안) 묶음 사이는 겹치지 않고, 긴 블록을 자를 때만 겹친다

- 내용:
  - 묶음은 블록의 경계에서 나눈다. 묶음이 절반 넘게 찬 뒤에 제목이 나오면 그 제목에서 새 묶음을 시작한다.
  - 블록마다 그 블록이 속한 제목들(예: "설치 > macOS")을 붙여 넘긴다(P1).
  - 묶음 사이에는 겹침을 두지 않는다.
  - 블록 하나가 한 묶음보다 길면 256 토큰씩 겹치게 잘라서 넘긴다. Mac 안의 AFM, PCC, 외부 LLM 모두 같다(D20).
  - 설정의 Overlap 줄은 없앤다. 겹침 256 토큰은 고정이다.
- 이유: Investigation의 "겹침이 필요한가". 이 방식에서는 잘리는 블록이 없고 제목이 문맥을 주므로, 묶음 사이의 겹침은 읽는 양(계산 3에서 약 7.7 %)과 결과 합치기만 늘린다.
- 대안: D14와 D17대로 묶음 사이도 256 토큰씩 겹치고, 설정에서 바꿀 수 있게 둔다(P19).
- 승인(D21). 대안은 쓰지 않는다.

### P21. (Claude의 제안) 데이터가 밖으로 나간다는 것을 알리는 방법

- 내용:
  - 설정의 AI 묶음에서 PCC나 External LLM을 고르면 확인 창을 띄운다. 예: "AI search will send the text of the document you search and your query to Apple's Private Cloud Compute." 외부 LLM이면 "Apple's Private Cloud Compute" 대신 "the server set below"라고 쓴다. 버튼은 "Use"와 "Cancel"이고, Cancel을 누르면 앞의 선택으로 돌아간다.
  - PCC나 External LLM을 고른 동안에는 AI 묶음에 같은 안내를 늘 보여 준다(P17).
- 대안: 확인 창 없이 AI 묶음의 안내만 보여 준다.
- 승인(D24). 대안은 쓰지 않는다.
- 바뀜(D31): PCC가 빠졌으므로 확인 창과 안내는 External LLM을 고를 때만 보여 준다. 문구는 "AI search will send the text of the document you search and your query to the server set below."다.

### P22. (Claude의 제안) macOS 26에서 AI 검색을 끄는 방법

- 내용:
  - macOS 26에서는 설정의 AI 묶음에서 "Apple Foundation Model (on this Mac)"과 "Apple Foundation Model (Private Cloud Compute)"를 흐리게 표시하고 "Requires macOS 27 or later."를 보여 준다. 고를 수 있는 것은 External LLM뿐이다.
  - 외부 LLM이 설정되었다는 것은 External LLM을 고르고 URL과 Model name을 채운 상태를 말한다. 그렇지 않으면 macOS 26에서 AI 검색은 꺼져 있다.
  - AI 검색이 꺼져 있어도 `/ai` 토큰은 만들어진다. Return을 누르면 카운터에 "AI unavailable"을 보여 주고, 카운터에 마우스를 올리면 "AI search needs macOS 27 or later, or an external LLM in Settings."를 보여 준다(P5).
  - macOS 26에서 기본 선택(Mac 안의 AFM, D18)을 그대로 두었다면, macOS 27로 올린 뒤 따로 바꾸지 않아도 Mac 안의 AFM을 쓴다. External LLM을 골라 두었다면 그 선택이 남는다.
- 이유: 기능이 왜 동작하지 않는지 사용자가 알 수 있다. `/ai`가 Mac마다 다르게 해석되지 않는다.
- 대안: macOS 26에서 외부 LLM이 설정되지 않았으면 `/ai`를 명령으로 알아보지 않고 일반 글자로 찾는다.
- 없어짐(D25): macOS 26에서도 Mac 안의 AFM을 쓰므로 이 제안은 필요 없다. macOS 26에서 PCC를 흐리게 표시하는 것은 P24가 맡는다.

### P23. (Claude의 제안) PCC는 Apple의 확인을 받을 때까지 이 요청에서 뺀다

- 내용: D15(PCC를 고를 수 있게 함)는 이 요청에서 빼고, `doc/DEVELOPMENT/`에 "PCC 적용" 주제를 남긴다. 요청자가 Apple에서 PCC entitlement를 받고 Developer ID 배포에서도 쓸 수 있다는 답을 받으면, 새 요청으로 넣는다. 그때까지 AI 묶음의 라디오 버튼은 Mac 안의 AFM과 External LLM 둘이다.
- 이유: Investigation의 "PCC 계정 확인". 지금은 entitlement가 없어 PCC를 부를 수 없고, Developer ID 배포에서 허용되는지 모른다. 허용되지 않으면 App Store 배포가 필요해 §10과 충돌한다. 쓸 수 없는 선택지를 먼저 만들 까닭이 없다.
- 대안: PCC 선택지를 지금 만들고, entitlement가 없거나 macOS 26이면 흐리게 표시한다(P17, P22).
- 쓰지 않음(D26): 요청자가 PCC entitlement를 신청하고, PCC는 이 요청에 남긴다. 대안(지금 만들고 쓸 수 없으면 흐리게 표시)은 P24로 이어진다.

### P24. (Claude의 제안) PCC를 쓸 수 없을 때의 표시

- 내용:
  - 다음 경우에는 AI 묶음의 "Apple Foundation Model (Private Cloud Compute)"를 흐리게 표시하고 고를 수 없게 하며, 그 아래에 이유를 보여 준다.
    - macOS 26: "Requires macOS 27 or later."
    - 앱에 PCC entitlement가 없음(entitlement를 받기 전의 빌드): "Not available in this version."
    - 기기가 PCC를 지원하지 않음(`deviceNotEligible`): "This Mac doesn't support Private Cloud Compute."
  - PCC를 고른 뒤에 생기는 일시적인 문제(준비 안 됨, 하루 한도, 연결 실패, 서비스 중단)에는 흐리게 표시하지 않는다. 그때는 검색할 때 "AI unavailable"과 이유를 보여 준다(P5).
  - PCC를 고른 상태에서 위의 경우가 되면(예: 다른 빌드), 선택은 그대로 두고 AI 검색은 "AI unavailable"을 보여 준다.
- 이유: 쓸 수 없는 까닭이 바뀌지 않는 경우만 막고, 곧 풀리는 문제는 검색할 때 알린다.
- 덧붙임(D27): 요청자는 macOS 26에서도 PCC를 쓰려고 했지만, Apple이 macOS 26에서는 앱에 PCC를 열어 두지 않았다. 그래서 macOS 26에서 흐리게 표시하는 것은 이 앱이 고를 수 있는 일이 아니다(Open questions 1).
- 승인(D29, D30).
- 없어짐(D31): PCC가 빠졌다.

### P25. (Claude의 제안) PCC용 provisioning profile은 저장소에 둔다

- 내용:
  - PCC entitlement를 앱에 넣으려면 entitlement가 든 Developer ID용 provisioning profile을 앱 번들(`Contents/embedded.provisionprofile`)에 넣고, 같은 entitlement로 서명해야 한다.
  - 이 profile은 `packaging/`에 두고 저장소에 커밋한다. 릴리스 워크플로의 secret은 늘리지 않는다(§11.4의 표는 그대로).
  - `cargo xtask bundle`은 profile이 있으면 번들에 넣고 entitlement로 서명한다. profile이 없으면 지금처럼 서명하고, 그 빌드에서는 PCC를 흐리게 표시한다(P24).
- 이유: provisioning profile은 배포하는 앱 번들마다 들어 있어 비밀이 아니다. 저장소에 두면 로컬 빌드와 CI가 같은 것을 쓴다.
- 대안: profile을 base64로 바꿔 repository secret으로 둔다(§11.4의 표에 한 줄 더한다).
- 승인(D28). 대안은 쓰지 않는다.
- 없어짐(D31): PCC가 빠졌다.

### P26. (Claude의 제안) PCC는 이 요청에서 뺀다

- 내용:
  - D26(PCC entitlement를 신청하고 PCC를 이 요청에 남김)을 거두고, PCC를 이 요청에서 뺀다. AI 묶음의 라디오 버튼은 "Apple Foundation Model (on this Mac)"과 "External LLM" 둘이다.
  - 그러면 PCC에만 필요하던 것도 빠진다: PCC 선택지를 흐리게 표시하는 규칙(P24, D29, D30), PCC용 provisioning profile과 entitlement 서명(P25, D28), §10의 변경, 릴리스 워크플로의 macOS 27 SDK.
  - 데이터가 Mac 밖으로 나간다는 확인 창과 안내(D19, P21)는 외부 LLM에만 남는다.
  - `doc/DEVELOPMENT/`에 "PCC 적용" 주제를 남긴다. Apple이 Small Business Program 없이 쓰는 길(앞으로 열겠다는 유료 사용 등)을 열거나 조건이 바뀌면, 새 요청으로 넣는다.
- 이유: Investigation의 "Small Business Program과 사업자 등록". PCC 없이도 AI 검색은 되고, 더 큰 모델은 외부 LLM으로 쓸 수 있다. PCC를 쓰려면 Small Business Program 가입(요청자의 말로는 사업자 등록까지), entitlement 승인, macOS 27이 모두 필요하고, Developer ID 배포에서 쓸 수 있는지도 모른다. 얻는 것에 비해 부담이 크다.
- 대안: 지금 결정(D26)대로 PCC를 남기고 신청한다.
- 승인(D31). 대안은 쓰지 않는다.

## Resulting specification

> STANDARD가 갖게 될 내용이다. 단계 2에서 영어로 쓴다. [D…]는 요청자가 정한 것이고, [P…]는 요청자가 승인한 Claude의 제안이다(D21, D24, D31). PCC는 이 요청에서 빠졌다(D31). 바뀌거나 없어진 결정과 제안(D2, D15, D23, D26–D30, P9, P12, P13, P14, P22–P25)은 반영하지 않았다.

### §6 Search

- 검색 막대는 지금과 같다: 검색 필드, 이전·다음 버튼, 카운터, Match Case, Done. Match Case는 keyword 방식에서만 쓸 수 있다 [P8].
- 검색 방식은 네 가지다 [D1, D3].

| 방식 | 명령 | 토큰 | 찾는 것 | 대소문자 | 검색 시작 |
|---|---|---|---|---|---|
| Keyword (기본) | `/key` | `key` | 글자 그대로 | Match Case [P8] | 입력하는 동안 [D7] |
| 정규식 | `/match` | `match` | 정규식 [P7] | 무시 [D3] | Return [D7, P3] |
| 정규식 | `/case` | `case` | 정규식 [P7] | 구분 [D3] | Return [D7, P3] |
| AI | `/ai` | `ai` | 질의와 관련 있는 블록 [P1, P4] | 해당 없음 | Return [D7, P3] |

- 기본 방식: 토큰이 없으면 keyword 방식이다 [D6]. 설정으로 바꾸지 않는다 [D5].
- 토큰: 맨 앞에 명령을 쓰고 스페이스를 누르면 명령이 토큰으로 바뀐다. 토큰을 지우면 keyword 방식으로 돌아간다 [D4, D6]. 세부 동작은 P6을 따른다.
- Keyword 검색: 지금 §6의 규칙(블록을 넘지 않음, 질의의 공백은 모든 공백 문자와 맞음 등)을 그대로 둔다.
- 정규식 검색: P7을 따른다.
- AI 검색:
  - 모델: 설정의 AI 묶음에서 고른 하나를 쓴다. Mac 안의 AFM(기본)과 외부 LLM이다 [D1, D8, D11, D18, D31]. 둘 다 macOS 26 이상에서 쓴다. macOS 26의 Mac 안 AFM은 2025년 모델이고 문맥 창이 4,096 토큰이다 [D25]. 외부 LLM은 OpenAI 호환 형식으로 부른다 [D12, P11].
  - 방법: Embedding·색인·DB 없이, 본문을 블록 단위의 묶음으로 나눠 모델이 관련 블록을 고른다. 블록마다 그 블록이 속한 제목을 붙인다. 아무것도 저장하지 않는다 [P1, P20].
  - 묶음: 블록의 경계에서 나누고, 묶음이 절반 넘게 찬 뒤에 제목이 나오면 그 제목에서 새 묶음을 시작한다. 묶음 사이는 겹치지 않는다. 블록 하나가 한 묶음보다 길면 256 토큰씩 겹치게 자른다. 모든 모델이 같다 [D20, D21, P20].
  - 묶음의 크기: Mac 안의 AFM은 모델이 알려 주는 문맥 창에서 지침과 답의 몫을 뺀 만큼이고, 외부 LLM은 설정한 최대치의 80 %다. 외부 LLM의 토큰은 글자 수로 어림한다 [D17, P19].
  - 시작은 Return이다 [D7]. 검색하는 동안 "Searching… 2/5"처럼 진행을 보여 주고, 질의를 고치거나 토큰을 지우거나 검색 막대를 닫으면 멈춘다 [P3]. 결과는 블록이고 문서 순서로 놓는다 [P4]. AI를 쓸 수 없으면 "AI unavailable"과 그 이유를 보여 준다 [P5].
  - 데이터: Mac 안의 AFM을 쓰면 문서는 Mac 밖으로 나가지 않는다. 외부 LLM을 쓰면 검색하는 문서의 글과 질의가 설정한 주소로 간다. 앱은 이것을 사용자에게 알린다 [P2, D19, P21, D31].
- 모든 방식에 공통: 강조, 카운터, 이동(Return, ⌘G, Shift-Return, ⇧⌘G), 끝에서 처음으로 돌기, 접힌 부분 펼치기, Esc/Done, ⌘E, 다시 읽은 뒤 다시 검색 [P4, P10].
- 그림 3: 검색 필드에 토큰(예: `ai`)이 있는 모습으로 바꾼다 [D4]. Match Case는 P8을 따른다.

### §9.1 Settings window

- 설정 창 왼쪽에는 사이드바가 있다. 사이드바에는 검색 필드와 Fonts, Colors, Margins, AI 항목이 있다 [D9, D10].
- 오른쪽에는 모든 설정을 한 목록으로 늘어놓는다. 사이드바의 항목을 고르면 목록에서 그 묶음으로 바로 옮겨 간다 [D10, P16].
- 검색 필드에 검색어를 넣으면 그 검색어를 가진 설정만 보여 준다 [D9, P15].
- AI 묶음 [D11–D13, D16–D19, D21, D22, D31, P17–P19, P21]:
  - Model: "Apple Foundation Model (on this Mac)"과 "External LLM" 가운데 하나를 라디오 버튼으로 고른다. 기본은 Apple Foundation Model (on this Mac)이다 [D11, D18, D31].
  - 라디오 버튼 아래에 고른 모델의 상태를 보여 준다(예: "Apple Intelligence is turned off.") [P17].
  - External LLM을 고르면 확인 창을 띄우고, 고른 동안에는 데이터가 Mac 밖으로 나간다는 안내를 보여 준다 [D19, P21].
  - External LLM: URL, Model name, Model options(키·값 줄, +와 − 버튼), API key(가림, 보기 토글), Max tokens(슬라이더 8K–1M, 입력란 1,024–2,000,000, 기본 32K). External LLM을 골랐을 때만 고칠 수 있다 [D11, D13, D16, D17, D22, P18, P19].
  - AI 검색 켬/끔과 Overlap 줄은 없다 [D5, D21].
- 바꾼 값은 지금처럼 바로 적용되고 저장된다. API key도 다른 설정과 함께 저장된다 [D16].

§10은 바꾸지 않는다. API key는 설정에 들어 있으므로 제거 pkg가 설정과 함께 지운다 [D16, D31].

## Impact

### 사용자에게 보이는 변화

- 토큰 없이 입력한 검색은 지금과 같은 keyword 검색이다 [D6, D7]. 업데이트해도 지금의 검색 동작은 바뀌지 않는다.
- `/ai `로 AI 검색, `/match `와 `/case `로 정규식 검색을 한다. 이 셋은 Return을 눌러야 검색한다 [D7].
- 설정 창에 사이드바(검색 필드, Fonts, Colors, Margins, AI)가 생기고, 설정은 한 목록으로 보인다 [D9, D10].
- AI 검색의 모델은 Mac 안의 AFM(기본)과 외부 LLM 가운데 고른다 [D11, D18, D31]. macOS 26에서도 Mac 안의 AFM을 쓴다. 다만 2025년 모델이고 문맥 창이 4,096 토큰이라 macOS 27보다 느리고 품질이 낮다 [D25]. Mac 안의 AFM은 Apple Intelligence가 켜져 있어야 쓸 수 있다.
- 외부 LLM을 고르면 AI 검색을 할 때 문서의 글과 질의가 Mac 밖으로 간다. 고를 때 확인 창이 뜨고, 설정에 안내가 보인다 [D19, P21]. 외부 LLM은 공급자에 따라 요금이 든다.
- API key는 화면에서는 가려지지만, 설정에는 암호화 없이 저장된다 [D16].
- 메뉴와 단축키(§9.2)는 바뀌지 않는다.

### 바꿀 STANDARD 절과 그림

| 절·그림 | 바꿀 내용 |
|---|---|
| §6 | 검색 방식, 명령과 토큰, keyword·정규식·AI 검색, 공통 동작. 지금의 "Searching starts while the user types"와 "The query is matched as literal text"는 keyword 방식의 규칙이 된다 |
| §9.1 | 사이드바(검색 필드, Fonts, Colors, Margins, AI), 한 목록, AI 묶음 |
| 그림 3 (`spec/STANDARD/window-layout.svg`) | 검색 필드의 토큰, Match Case |

§3(대상 환경), §10(설치와 제거), §11.1(Swift는 꼭 필요한 곳에만), §11.4(릴리스와 secret)는 바꾸지 않는다. AFM은 Swift 전용이므로 §11.1의 꼭 필요한 경우에 해당한다. 앱은 지금처럼 macOS 26 이상에서 돈다(D25).

### 바꿀 소스 코드와 테스트

| 파일 | 바꿀 내용 |
|---|---|
| `crates/core/assets/viewer.js` | 정규식 검색, AI 검색에 넘길 블록 글 내보내기, 블록 단위 결과 강조 |
| `crates/core/src/` (새 모듈) | 명령 해석(P6), 묶음 나누기(블록 경계, 제목 앞, 긴 블록의 겹침)와 토큰 어림(P1, P19, P20), 외부 LLM 요청 본문(모델 옵션 포함)과 응답 읽기(P11, P18), 설정 검색의 거르기(P15). 네트워크와 AFM 없이 테스트할 수 있게 Rust로 둔다 |
| `crates/core/src/settings.rs` | AI 설정 값: 모델 선택, URL, 모델 이름, 모델 옵션(키·값 목록), API key, 최대치 |
| `crates/app/src/find_bar.rs` | 토큰 표시와 지우기, Match Case 켜고 끄기, 새 카운터 문구("Searching… 2/5" 등) |
| `crates/app/src/viewer.rs` | 방식 고르기, Return으로 시작하는 검색, AI 검색의 시작과 취소, 다시 읽은 뒤 다시 검색 |
| `crates/app/src/settings_window.rs` | 사이드바(검색 필드, 항목), 한 목록과 스크롤 이동, 검색 거르기, AI 묶음(라디오 버튼, 상태 줄, 확인 창) |
| `crates/app/src/` (새 모듈) | 외부 LLM HTTP 요청 |
| 새 Swift 코드, `crates/app/build.rs`, `crates/app/Cargo.toml` | Mac 안의 AFM 호출(macOS 26부터)과 그 빌드·링크 |
| `.github/workflows/release.yml` | 러너의 Xcode에 `contextSize`가 든 macOS SDK(26.4 이상)가 있는지 확인하고, 없으면 Xcode를 고르도록 바꾼다 |
| 테스트 | 명령 해석, 설정 기본값과 예전 설정 읽기, 묶음 나누기(긴 블록의 겹침 포함), 토큰 어림, 모델 옵션의 JSON 변환, 요청 본문과 응답 읽기, 설정 검색의 거르기. CI에서는 AFM과 외부 LLM을 쓸 수 없으므로 실제로 부르는 테스트는 두지 않는다 |

### 바꿀 문서

| 문서 | 바꿀 내용 |
|---|---|
| `README.md` | 사용법 표 |
| `doc/ARCHITECTURE.md` | §1("Swift나 Objective-C 코드는 없습니다"), §7 검색, §10 설정과 설정 창의 구조, §13 빌드(Swift 컴파일), 그림 `components.svg` |
| `packaging/help/ko.md`, `en.md` | 검색, 설정 |
| `doc/DEVELOPMENT/` | AI 검색의 속도와 품질(지침 문구, 묶음 크기)을 재는 주제 추가. "PCC 적용" 주제 추가: Apple이 Small Business Program 없이 PCC를 쓰는 길을 열면 다시 다룬다(D31) |

`CLAUDE.md`는 §11.3이 바뀌지 않으므로 그대로다.

## Open questions

> 이전 목록의 2(겹침)는 D21로, 3(최대치)은 D22와 D24로, 4(macOS 26)는 D23과 D25로, 5(나머지 제안)는 D24로 정해졌다. 그다음 목록의 1(PCC)은 D26으로, 2(P22)는 D25로 정해졌다. 그다음 목록의 1(PCC를 이 요청에 남김)은 D27로 확인되었고, 2(P25)는 D28로 정해졌다. 그다음 목록의 1(macOS 26의 PCC)은 D29로, 2(P24)는 D30으로 정해졌다. 마지막 목록의 1(PCC를 남길지)은 D31로 정해져 PCC는 빠졌다.

없음.
