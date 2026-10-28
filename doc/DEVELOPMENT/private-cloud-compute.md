# Private Cloud Compute 적용

## 현재 상태

명세 요청서 005에서 Private Cloud Compute(PCC)를 AI 검색의 모델로 넣으려다 뺐습니다(D31). 지금 AI 검색의 모델은 Mac 안의 AFM과 외부 LLM 두 가지입니다(명세 §6.6).

- PCC(`PrivateCloudComputeLanguageModel`)는 macOS 27부터 있고, 문맥 창이 32,768 토큰입니다. Mac 안의 AFM(4,096 또는 8,192 토큰)보다 크고, 비용이 들지 않습니다. 사용자마다 하루 한도가 있습니다.
- 앱에서 쓰려면 개발자가 App Store Small Business Program에 가입하고, PCC entitlement를 신청해 받아야 합니다. 무료 앱에도 예외가 없습니다.
- Small Business Program에 가입하려면 Paid Apps 계약을 받아들이고 금융·세금 정보를 넣어야 합니다. 요청자에 따르면 한국의 개인 개발자는 사업자 등록까지 해야 합니다.
- Apple의 안내는 App Store, TestFlight, ad hoc 배포만 말하고, 이 앱처럼 Developer ID로 배포하는 경우(명세 §10)는 말하지 않습니다.

## 논의할 점

- Apple이 Small Business Program 없이 PCC를 쓰는 길(예고한 유료 사용 등)을 열거나, Developer ID 배포에서도 쓸 수 있다고 밝히면 다시 다룹니다.
- 넣으려면 명세 요청서가 필요합니다. 모델 선택과 안내 문구(문서가 Apple 서버로 감)가 바뀌고, entitlement가 든 provisioning profile을 번들에 넣어야 하므로 `xtask`와 릴리스 워크플로도 바뀝니다.
