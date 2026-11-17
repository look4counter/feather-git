# 작업 목록: 사용 흐름 점검

## 1단계: 준비

- [x] T001 src/commit_history_actions.rs, src/synchronization.rs, src/rebase_actions.rs, src/workspace_backend.rs와 화면 도구 제약을 분석한다.

## 2단계: 사용자 이야기 1

- [x] T002 [US1] target/qa-run/history-probe.rs에서 원본 src/commit_history_actions.rs와 src/repository_search.rs의 Reset/Revert 및 검색을 호출하는 격리된 재현 도구를 만든다. FR-002, FR-003
- [x] T003 [US1] 테스트 저장소에서 전후 HEAD와 오류·반환 결과를 비교하고 target/qa-run에 기록한다. FR-002, FR-004
- [x] T004 [US1] 전체 테스트·빌드·정적 분석과 코드 검토 결과를 docs/qa-2026-11-17.md에 기록한다. FR-001, FR-004

## 3단계: 수렴

- [x] T005 specs/083-usage-audit의 명세·계획·작업과 결과를 수렴 검사한다.

## 의존성과 전략

T001 → T002 → T003 → T004 → T005 순서이다. 앱 소스는 수정하지 않는다. 원본 함수의 실제 동작을 확인하는 재현이 최소 완료 단위이며 전체 보고서까지 완료한다. 별도 파일의 로그 검토와 정적 분석은 독립 실행 가능하다.
