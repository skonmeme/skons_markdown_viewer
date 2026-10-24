# Initial specification

## Request

> The original request below is written in Korean and is kept verbatim; only its headings are nested under this section.

맥용 Markdown Viewer를 만들려고 해.

### 이름
skon's Markdown Viewer (skon은 소문자)
- 번들 파일명과 ID는 skons_markdown_viewer
- 

### 대상 환경
macOS 26 이상, Apple Silicon 전용

### 기능
0. Markdown viewer가 갖춰야하는 기본 기능
1. 좌우 여백 조절 자동 (창의 폭이 작으면 자동으로 여백을 줄이고 폭이 넓으면 상대적으로 여유롭게). 수동으로 변경 가능
2. 검색
3. 글꼴 변경 (기본 글꼴 있으나, 한글, 영문 글꼴을 별도로 지정 가능. 글꼴 셋 구성 가능)
4. 글자색, 배경색 변경

### 설치
- 설치툴, 제거툴 필요
  - pkg 방식
- 설치 위치: 모든 사용자를 위한 위치

### 개발 방식
spec/STANDARD.md: 프로젝트에서 정한 모든 스펙을 적어둔 문서
1. 오직 이 문서만이 스펙문서임. 다른 어떤 문서도 스펙과 충돌하면 이 문서가 참
2. 기본적인 글과 이해를 돕기 위한 그림을 포함함 (spec/STANDARD/ 폴더에 보충 파일을 담을 수 있음)
3. 언어적 혼란을 피하기 위해 이 문서에서는 영어만 사용
4. spec/requests에서 만들어지는 명세 요청서를 통해 이 문서를 갱신할 수 있음. main / develop 브랜치에서는 작업 불가. 작업은 항상 명세 요청서가 commit된 브랜치에서 함
5. 스펙문서 갱신 과정에서 기능에 부합하는 소스코드를 생성 및 수정함.
6. 스펙문서 갱신한 후 명세 요청서는 spec/merged로 이동
7. 과정에서 문제가 발생하지 않은 경우 자동으로 작업 브랜치 내에서 commit 함

README.md: 사용 설명용 문서. 한글로 작성. 간결하게 설치 및 사용법 설명

doc/DEVELOPMENT/ : 개발 과정에 필요한 논의사항. 파일별로 관리되며 해소되면 삭제
doc/ARCHITECTURE.md : 앱의 작동 방식에 대해 상세히 설명. 그림과 동영상도 첨부하고 보충파이들은 doc/ARCHITECTURE/에 저장
doc/ 문서들은 특별한 이유가 없는 이상 한글로 작성

### 배포
- develop 브랜치에서 개발
- main에 PR 머지시 release 배포

### 개발 언어
가급적이면 Rust 였으면 좋겠음. Swift가 꼭 필요하다면.. 혼용허용
