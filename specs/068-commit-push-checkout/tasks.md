# 작업 목록

## 1단계: Commit 확인 (사용자 이야기 1)
- [x] T001 [US1] src/commit_dialog.rs의 확인 버튼을 항상 활성화하고 준비 상태·실패 안내와 중복/IME 차단을 실행 시 처리한다.

## 2단계: Push 무시 (사용자 이야기 2)
- [x] T002 [US2] src/branch_actions.rs에 초기 커밋 없음·동일·behind-only 판정과 원격 객체 조회를 구현하고 no-op·분기 거부·오류 회귀 테스트를 갱신한다.

## 3단계: 참조 체크아웃 (사용자 이야기 3)
- [x] T003 [US3] src/branch_actions.rs에 안전한 참조 체크아웃과 로컬/원격/태그·충돌·잠금·진행 상태 테스트를 구현하고 src/repository.rs, src/backend.rs에 노출한다.
- [x] T004 [US3] src/workspace_backend.rs에 Checkout 작업과 재조회 통합 테스트를 추가한다.
- [x] T005 [US3] src/ui.rs 저장소 목록을 이벤트 보존 가상 목록으로 연결하고 단일/더블클릭 라우팅·참조 검증·진행·결과 오류를 처리하며 관련 테스트를 추가한다.

## 4단계: 통합 검증 및 수렴
- [x] T006 전체 테스트·fmt·엄격한 Clippy·별도 빌드를 실행하고 specs/068-commit-push-checkout/quickstart.md에 결과를 기록한다.
- [x] T007 specs/068-commit-push-checkout의 명세·계획과 구현을 수렴 점검하여 미완료 항목을 해결한다.

## 의존성·실행 전략
T001→T002→T003→T004→T005→T006→T007 순서로 진행한다. 독립 분석만 병렬 가능하며 코드 변경과 테스트는 순차 수행한다. Commit 확인과 Push를 먼저 검증한 뒤 checkout backend와 UI를 통합한다. 사용자 저장소의 실제 Git 상태는 변경하지 않는다.
