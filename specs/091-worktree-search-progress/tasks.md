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

## 단계 5 — 스테이징 화면 안정화

- [x] T007 `src/ui.rs`에서 쓰기 이후 재조회 중의 목록과 차이 보기 대상 보존을 회귀 테스트한다. FR-007, SC-005
- [x] T008 `src/ui.rs`, `src/diff_viewer.rs`, `src/virtual_list.rs`에서 작업 트리 결과와 차이 보기 갱신을 연결하고 중복 조회를 없앤다. FR-007, FR-008, SC-005
- [x] T009 `src/ui.rs`, `src/progress.rs`, `src/search_panel.rs`, `src/split_button.rs`에서 기존 입력 보호를 보존하며 상태표시줄과 툴바, 저장소 화면의 표시를 안정화한다. FR-006, SC-006
- [x] T010 관련 테스트와 전체 대상 검사, 릴리스 빌드 결과를 `validation.md`에 기록한다. SC-005, SC-006

## 단계 6 — 기존 진행 창 배경 복원

- [x] T011 `src/progress.rs`와 `src/ui.rs`에서 진행 카드 표시와 배경 암전을 다시 함께 적용하고 지연 및 화면 보존을 검증한다. FR-006, SC-006
