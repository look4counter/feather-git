# 작업 목록

## 1단계: 준비

- [x] T001 src/diff_git.rs·src/workspace_backend.rs·src/ui.rs·src/diff_viewer.rs·src/partial_stage.rs의 조회·표시·선택·쓰기 흐름을 분석한다.

## 2단계: 기반

- [x] T002 specs/042-staged-first-hunk-select/spec.md·plan.md·contracts/ui.md의 일관성을 분석한다.

## 3단계: 사용자 이야기 1

독립 검증: Staged가 경로와 무관하게 먼저 표시되며 같은 경로의 두 상태와 diff 대응이 보존된다.

- [x] T003 [US1] src/diff_git.rs에 실제 저장소 정렬·표시 행 대응 테스트를 추가하고 구현 전 실패를 확인한다. (FR-001~003)
- [x] T004 [US1] src/diff_git.rs의 작업 트리 정렬 우선순위를 Staged → 경로로 변경한다. (FR-001~003)

## 4단계: 사용자 이야기 2

독립 검증: 구간 선택은 해당 구간 체크만 바꾸고 실제 Stage는 기존 버튼으로 실행한다.

- [x] T005 [US2] src/diff_viewer.rs의 체크 토글 로직을 재사용 가능한 순수 함수로 추출하고 구간 선택·해제·부분 체크·다른 구간 보존·경계 테스트를 추가한다. (FR-004~006)
- [x] T006 [US2] src/diff_viewer.rs의 버튼을 구간 선택으로 바꾸고 hunk_rows → toggle로 연결하며 Stage는 체크 집합만 처리하도록 변경한다. (FR-004~006)

## 5단계: 검증 및 수렴

- [x] T007 전체 테스트·서식·Clippy·빌드 결과와 미검증 범위를 specs/042-staged-first-hunk-select/validation.md에 기록한다.
- [x] T008 specs/042-staged-first-hunk-select/spec.md·plan.md·tasks.md와 구현의 수렴을 확인하고 남은 작업을 완료한다.

## 의존성 및 전략

T001 → T002 → T003 → T004 → T005 → T006 → T007 → T008 순서로 수행한다. 사용자 이야기 1을 먼저 검증하고 사용자 이야기 2를 연결한다. T003과 T005의 테스트 작성은 서로 다른 파일에서 독립 가능하지만 순차 수행한다. 기본 기능을 추가한 뒤 전체 회귀와 문서 일치를 확인한다.
