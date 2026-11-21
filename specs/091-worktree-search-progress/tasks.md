# 실행 작업 — 전역 진행 표시

## 단계 1 — 작업 트리

- [x] T001 [US1] `src/ui.rs`에서 파일 작업과 기존 전역 진행 카드의 표시 및 입력 차단을 연결한다. FR-001, FR-003, FR-004
- [x] T002 [US1] `src/diff_viewer.rs` 및 `src/ui.rs`에서 부분 스테이징·폐기 작업 진행 상태를 관찰한다. FR-001, FR-003, FR-004

## 단계 2 — 찾기

- [x] T003 [US2] `src/search_panel.rs` 및 `src/ui.rs`에서 현재 저장소의 검색 대기 상태를 관찰한다. FR-002, FR-003, FR-004

## 단계 3 — 검증

- [x] T004 `src/ui.rs`, `src/diff_viewer.rs`의 관련 상태 테스트, 컴파일 검사 및 릴리스 빌드를 실행하고 `validation.md`에 기록한다. SC-001, SC-002

## 단계 4 — 표시 지연

- [x] T005 `src/ui.rs`와 `src/progress.rs`에서 기존 공통 150ms 지연으로 카드 표시를 결정하고 작업 종료 시 타이머를 취소한다. FR-005, SC-003, SC-004
- [x] T006 `src/ui.rs`의 경계 테스트와 컴파일·릴리스 빌드를 실행하고 `validation.md`에 기록한다. SC-003, SC-004
