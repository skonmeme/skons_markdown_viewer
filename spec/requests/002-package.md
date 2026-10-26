# Package

## Request

> The original request below is written in Korean and is kept verbatim.

1. 개발자용 스냅샷 모드는 필요없어.
2. 명령줄 인자로 파일 열기는 있으면 좋긴 한 거 같아.
3. /Applications에 설치되어야할 것 같은데? 제대로된 앱 이름을 결정안한 상태야. 설마 skon_markdown_viewer가 공식 이름이라고 하는 건 아니겠지?
4. main에 머지할 때 pkg형태로 배포하자고 하고 있는데, Developer ID 서명 공증은 당연히 해야겠지?
5. 앱 아이콘 당연히 디자인 해야겠고
6. Help에는 README.md 같은 쓸데없는 것 넣을 필요없고, 사용법만 있으면 되는 거 아닐까?

## Decisions

> Answers given later in the conversation, quoted verbatim. The local test steps and the Help document are proposals the requester accepted. The app icon is the final draft the requester approved after several revisions.

- 앱 이름: "앱 표시는 "skon's Markdown Viewer"로 하면 되고, 이름은 그냥 MarkdownViewer면 되지 않을까?" / "앱이름은 MarkdownViewer 이게 좋네"
- 번들 ID: "번들 ID: io.github.skonmeme.markdownviewer"
- 배포용 Secrets: "배포에 쓰이는 secret 변수: DEVELOPER_ID_P12, DEVELOPER_ID_P12_PASSWORD, NOTARY_ISSUE_ID, NOTARY_KEY_ID, NOTARY_KEY 등록하면 되나?" / "Secrets은 등록 완료" / "NOTARY_ISSUER_ID로 변경"
- 릴리스 빌드: "인증서와 서명이 동일하니 로컬에서 만들던 git workflow로 만들던 같은 거 아닌가? main으로 PR 머지할 때 git workflow로 release 배포판을 만들면 됨"
- Help: 앱 안에 사용법 문서를 넣고 Help 메뉴로 그 문서를 이 앱의 창으로 연다. / "help는 시스템 언어를 따르도록"
- 앱 아이콘 (최종 시안, Icon Composer `.icon` 형식, 1024 pt 캔버스):
  - 배경: 밝은 회색 (`automatic-gradient` `extended-srgb:0.95,0.95,0.97`)
  - 배지: 양쪽에 반원이 붙은 직사각형(알약 모양), 880 × 440 pt, 가운데 배치. 색 없는 투명 유리(채움 흰색 alpha 0.05, translucency 0.92). Icon Composer 굴절: 세기 0.15, 깊이 0.1
  - 마크: Markdown 마크 M↓(테두리 없음), 폭 520 pt, 배지 가운데. 유리가 아닌 불투명 레이어. 라이트 모드 검은색, 다크 모드 흰색
  - 배경 글자: "skon's" 소문자(글꼴이 아닌 직접 그린 둥근 획, 굵기 26 pt), 짙은 파랑 `#0033aa`, 불투명도 0.5, 오른쪽 위로 15° 기울임. 크기는 배지의 약 2/3이고 왼쪽 위에 두어, 아랫부분이 배지 위쪽 가장자리에 걸쳐 굴절로 휜다
- 문서: "이번 명세 요청서로 구현할 때 README, ARCHITECTURE, DEVELOPMENT/ 만들어줘"
- 배포 전 로컬 테스트: "배포도 배포인데, 로컬 설치를 해서 테스트를 하고 배포해야하잖아?" / "제안대로"
  1. 로컬에서 서명·공증된 설치 pkg와 제거 pkg를 만든다. 서명에는 키체인의 Developer ID 인증서를, 공증에는 키체인에 저장한 notarytool 자격 증명을 쓴다.
  2. 설치 pkg로 /Applications에 설치하고, 앱을 실행해 기능과 Gatekeeper 통과 여부를 확인한다.
  3. 제거 pkg로 깨끗이 지워지는지 확인한다.
  4. 문제가 없으면 main으로 PR을 머지해 릴리스한다.
