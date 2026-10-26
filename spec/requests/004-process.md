# Process

## Request

> The original request below is written in Korean and is kept verbatim. Blank lines were added only so that it renders correctly as Markdown.

이 답변 사라지면 어쩔려고 맘대로 처리하지?

1. 명세 요청서 관리
2. STANDARD 업데이트
3. 구현 및 후속 작업
4. commit

이 처리 단계를 스펙문서인 STANDARD에 명확하게 반영되서 작동되도록 명세 요청서를 추가해줘

## Background

- 요청서 003을 처리하면서 Claude는 요청자의 말만 요청서에 옮기고, 결정의 구체 내용·근거·영향은 대화에만 남겼다. 이를 지적받자 요청서 작성 규칙을 저장소가 아니라 Claude의 메모리에 스스로 적었다. 메모리는 저장소 밖에 있어 사라질 수 있다.
- 지금 STANDARD §11.3과 그림 4는 처리 순서만 정한다. 각 단계를 언제 시작하는지, 요청서에 무엇을 적는지, 언제 무엇을 커밋하는지는 정하지 않았다.
- 다음 규칙은 STANDARD에 없고 Claude의 메모리에만 있다.

| # | 규칙 | 출처 |
|---|---|---|
| 1 | 요청서 파일 이름은 `spec/requests/NNN-title.md`(3자리 번호)다. | 요청서 001 작업 중 요청자의 지시 (2026-10-24) |
| 2 | 요청서는 지금 체크아웃된 브랜치에 만들고 커밋한다. Claude는 브랜치를 만들지 않는다. 지금 브랜치가 `main`이나 `develop`이면 요청서를 만들지 않고, 브랜치를 바꿔야 한다고 요청자에게 묻는다. | 요청서 001 작업 중, Claude가 브랜치를 마음대로 만든 일에 대한 요청자의 지적 (2026-10-24) |
| 3 | Claude는 요청자가 명시적으로 요청한 것만 한다. 코드, 브랜치, 파일, 문서, 추가 수정 등 그 밖의 일은 먼저 묻는다. STANDARD 갱신도 요청이 있을 때만 한다. | 요청서 001 작업 중 요청자의 지적 "누구 맘대로 만들고 난리야"와 메모리에 두라는 지시 (2026-10-24) |
| 4 | 요청서에는 결정 내용, 근거, 영향, 미정 사항을 적는다. | 요청서 003 작업 중 요청자의 지적. 메모리에는 Claude가 스스로 적음 (2026-10-26) |

## Current specification

STANDARD §11.3:

> 1. This document is updated only through specification requests, which are written in `spec/requests/`.
> 2. No work is done on the `main` and `develop` branches. Work is always done on the branch where the specification request is committed.
> 3. While this document is updated, the source code is created or changed so that it matches this document.
> 4. After this document has been updated, the specification request is moved to `spec/merged/`.
> 5. If no problem occurs during the process, the changes are committed automatically on the working branch.

그림 4(`spec/STANDARD/process.svg`)는 "1. Commit the request in spec/requests/ → 2. Update STANDARD.md and the source code → 3. Move the request to spec/merged/ → 4. Commit automatically if no problem occurs"를 보여 준다. `doc/ARCHITECTURE.md` §15도 같은 절차를 한국어로 적고 있다.

## Decisions

> 각 결정의 "근거"에는 요청자의 말을 그대로 옮겼다.

### D1. CLAUDE.md에 STANDARD의 관리 방법을 넣는다

- 내용: 스펙 문서는 STANDARD.md 하나뿐이다. Claude가 이 문서의 관리·갱신 방법을 항상 알고 따르도록, 그 방법을 `CLAUDE.md`에 넣는다. 두 문서가 다르면 STANDARD가 맞다(§1.1).
- 근거: "스펙문서가 오직 STANDARD.md 하나뿐이고, 이 문서의 관리나 갱신방법을 항상 숙지하게 하기 위해서 CLAUDE.md에 관리방법을 넣어두어야해." / "STANDARD를 자꾸 무시하는 이유가 이거였구만."
- 이유: Claude는 세션마다 `CLAUDE.md`와 자기 메모리는 자동으로 읽지만 STANDARD는 읽지 않는다. 관리 방법이 STANDARD에만 있으면 Claude는 그것을 모른 채 작업한다.

## Proposal

> Claude의 제안이며 아직 결정되지 않았다. 요청자가 정한 것은 Decisions 절로 옮긴다.

### 단계 1. 명세 요청서 관리

- 만들기: 요청자가 요청하면 지금 브랜치에 `spec/requests/NNN-title.md`를 만든다. `NNN`은 `spec/requests/`와 `spec/merged/`를 통틀어 다음 번호이고, `title`은 짧은 영어 이름이다. 브랜치는 Background의 규칙 2를 따른다.
- 구성:

| 절 | 내용 |
|---|---|
| Request | 요청자의 원래 요청. 원문 그대로 |
| Decisions | 결정마다 내용(구체 값, 공식, 동작), 근거(요청자의 말 원문), 이유. Claude가 제안했지만 요청자가 명시적으로 승인하지 않은 것은 그렇다고 표시 |
| Resulting specification | 결정에 따라 STANDARD가 갖게 될 내용 |
| Impact | 사용자에게 보이는 변화, 바뀌는 STANDARD 절과 그림, 소스 코드, 테스트, 문서 |
| Open questions | 아직 정하지 않은 것 |

- 갱신: 대화에서 결정이 나오거나 바뀌면 그때 바로 요청서를 고친다. 결정, 계산 결과, 제안을 대화에만 두지 않는다. 결정이 바뀌어도 이전의 말은 지우지 않고 새 말을 덧붙인다.

### 단계 2. STANDARD 업데이트

- 요청자가 지시할 때만 시작한다. 요청서에 Open questions가 남아 있으면 시작하지 않는다.
- 요청서의 결정을 빠짐없이 반영하고, 결정과 관계없는 부분은 바꾸지 않는다.
- 영어로 쓰고, 바뀐 내용에 맞게 그림(`spec/STANDARD/`)도 고친다.
- 결정이 STANDARD의 다른 부분과 충돌하면 멈추고 요청자에게 묻는다.

### 단계 3. 구현 및 후속 작업

- STANDARD에 맞게 소스 코드와 테스트를 고친다. `cargo test --workspace`가 통과해야 한다.
- 문서를 STANDARD에 맞춘다: `README.md`, `doc/ARCHITECTURE.md`와 `doc/ARCHITECTURE/`, `doc/DEVELOPMENT/`(논의할 주제는 추가하고 해소된 주제는 지움), Help(`packaging/help/`), `CLAUDE.md`(§11.3이 바뀐 경우, D1).
- 모두 끝나면 요청서를 `spec/merged/`로 옮긴다.

### 단계 4. Commit

- 각 단계가 문제없이 끝나면 작업 브랜치에 자동으로 커밋한다. 빌드나 테스트가 실패하거나 STANDARD 안에서 충돌이 생기면 커밋하지 않고 요청자에게 알린다.
- 커밋은 작업 브랜치에만 한다. push, `develop` 머지, `main` PR은 요청자가 지시할 때만 한다(§11.4).
- 커밋 메시지는 지금까지의 이력을 따른다.

| 작업 | 커밋 메시지 |
|---|---|
| 요청서 추가 | `spec: add request NNN-title` |
| 요청서 갱신 | `spec: <바뀐 내용> to request NNN-title` (예: `spec: add decisions to request 002-package`) |
| STANDARD 갱신 | `spec: update STANDARD.md from request NNN-title` |
| 구현 | `Implement STANDARD from request NNN-title` |
| 문서 | `docs: <바뀐 내용>` |
| 요청서 이동 | `spec: move request NNN-title to merged` |

## Impact

- `spec/STANDARD.md` §11.3을 위의 네 단계로 다시 쓴다. 지금 §11.3의 다섯 항목은 그 안에 들어간다.
- 그림 4(`spec/STANDARD/process.svg`)를 네 단계와 각 단계의 시작 조건, 커밋 지점에 맞게 고친다.
- `CLAUDE.md`에 STANDARD의 관리 방법, 곧 새 §11.3의 내용을 넣는다(D1).
- `spec/STANDARD.md` §11.2의 문서 표에 `CLAUDE.md`와 그 역할을 넣는다(D1).
- `doc/ARCHITECTURE.md` §15를 STANDARD에 맞춘다.
- Claude 메모리에서, STANDARD로 옮긴 규칙의 메모를 지운다.
- 요청서 003은 이 요청이 반영된 뒤 새 절차를 따른다.
- 앱 동작과 소스 코드는 바뀌지 않는다.

## Open questions

1. 단계 2를 요청자의 지시로만 시작할지(지금 Claude 메모리의 규칙 3), 요청서에 Open questions가 없어지면 바로 시작할지. 그리고 단계 3을 단계 2에 바로 이어서 할지. 지금 §11.3의 3번은 STANDARD를 갱신하면서 코드도 고친다고 정한다.
2. 요청서를 고칠 때마다 커밋할지, 요청서 단계가 끝날 때 한 번에 커밋할지.
3. 한 브랜치에 요청서를 여러 개 둘 수 있는지. 이 요청서는 지금 브랜치 `feat/margin`에 요청서 003과 함께 있다.
