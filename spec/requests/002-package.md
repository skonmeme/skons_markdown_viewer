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

> Answers given later in the conversation, quoted verbatim. The local test steps are the proposal the requester accepted with "제안대로".

- 앱 이름: "앱 표시는 "skon's Markdown Viewer"로 하면 되고, 이름은 그냥 MarkdownViewer면 되지 않을까?" / "앱이름은 MarkdownViewer 이게 좋네"
- 번들 ID: "번들 ID: io.github.skonmeme.markdownviewer"
- 배포용 Secrets: "배포에 쓰이는 secret 변수: DEVELOPER_ID_P12, DEVELOPER_ID_P12_PASSWORD, NOTARY_ISSUE_ID, NOTARY_KEY_ID, NOTARY_KEY 등록하면 되나?" / "Secrets은 등록 완료"
- 앱 아이콘: "앱 아이콘 .icon 형식으로 시안 보여줘" / "아이콘 시안은 맘에 안들었어. Markdown 아이콘에 배경으로 분홍색 skon's를 사선으로 넣어주고 유리 느낌을 확실하게 표시해줘"
- 문서: "이번 명세 요청서로 구현할 때 README, ARCHITECTURE, DEVELOPMENT/ 만들어줘"
- 배포 전 로컬 테스트: "배포도 배포인데, 로컬 설치를 해서 테스트를 하고 배포해야하잖아?" / "제안대로"
  1. 로컬에서 서명·공증된 설치 pkg와 제거 pkg를 만든다. 서명에는 키체인의 Developer ID 인증서를, 공증에는 키체인에 저장한 notarytool 자격 증명을 쓴다.
  2. 설치 pkg로 /Applications에 설치하고, 앱을 실행해 기능과 Gatekeeper 통과 여부를 확인한다.
  3. 제거 pkg로 깨끗이 지워지는지 확인한다.
  4. 문제가 없으면 main으로 PR을 머지해 릴리스한다.
