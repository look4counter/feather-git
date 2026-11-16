# 조사 및 결정

- GPUI 공식 examples/input.rs의 EntityInputHandler와 TextElement를 재사용한다. 앱에 기존 입력이 없어 별도 구현이 필요하다. old의 대형 CommitEditor는 diff/멀티라인 의존성이 많아 가져오지 않는다.
- git2 기존 의존성으로 초기화/열기/복제와 상태·이력·브랜치를 읽는다. RepositoryOpenFlags::NO_SEARCH와 RepositoryInitOptions::no_reinit로 기존/부모 저장소를 보호한다.
- 경로는 실제 대상, 이름은 별도 표시값이라는 사용자 답변을 적용한다. 이름 편집으로 경로를 바꾸지 않는다.
- Git 작업을 GPUI background_executor에서 실행하고 창 컨텍스트로 결과를 반영한다. 네이티브 폴더 탐색은 prompt_for_paths를 사용한다.
- 기존 정적 샘플 직접 접근을 유지하면 새 저장소도 가짜 이력이 보이므로 공통 Project 모델로 변환한다. 이번 요청을 충족하는 필수 영향 범위이다.
