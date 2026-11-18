# 작업 목록: 찾기 목록 공통 스크롤

## 1단계: 준비

- [x] T001 `src/search_panel.rs`, `src/virtual_list.rs`, `src/ui.rs`의 호출과 상태 수명을 분석하고 `plan.md`에 기록한다.

## 2단계: 기반 검증

- [x] T002 `spec.md`, `plan.md`, `tasks.md`의 요구사항 대응과 기존 구조 준수를 분석한다.

## 3단계: 사용자 이야기 1

독립 검증 기준: 많은 결과에서 스크롤과 연속 방향키 선택이 유지된다.

- [x] T003 [US1] `src/search_panel.rs`에 공통 핸들을 보관하고 목록 렌더에 전달한다. (FR-001, FR-002)
- [x] T004 [US1] `src/search_panel.rs`의 새 검색 초기화와 활성 상태 변경에 핸들 수명을 연결한다. (FR-003, FR-004)
- [x] T005 [US1] 기존 `src/search_panel.rs`, `src/virtual_list.rs` 테스트를 포함한 자동 검증과 빌드·정적 분석을 실행하고 `validation.md`에 결과 및 수동 미검증 범위를 기록한다.

## 4단계: 마무리

- [x] T006 `src/search_panel.rs`와 명세·계획·작업을 수렴 점검하고 `validation.md`에 남은 차이를 기록한다.

## 의존성과 구현 전략

T001 → T002 → T003 → T004 → T005 → T006 순서로 수행한다. 단일 파일 수명을 함께 변경하므로 병렬 구현은 없다. 사용자 이야기 1 전체가 최소 제공 범위다. 기존 공통 컴포넌트와 테스트를 재사용한다.

## 5단계: 사용자 이야기 2 — 검색 컨트롤 크기

- [x] T007 [US2] `src/ui.rs`, `src/form_controls.rs`, `src/text_input.rs`, `src/dialog.rs` 크기와 데이터 흐름을 분석하고 `spec.md`, `plan.md`를 갱신한다.
- [x] T008 [US2] `src/text_input.rs`에 기존 기본값을 보존하는 작은 입력 생성자를 추가하고 `src/search_panel.rs`의 버튼과 입력창을 탭 크기에 맞춘다. (FR-005~FR-007)
- [x] T009 [US2] 검색·입력·IME 회귀 테스트와 빌드·정적 분석을 실행하고 `validation.md`에 결과를 기록한다.
- [x] T010 `src/search_panel.rs`, `src/text_input.rs`와 갱신된 명세를 수렴 점검한다.

추가 의존성: T007 → 일관성 분석 → T008 → T009 → T010. 같은 검색 UI 변경이므로 순차 구현한다. 사용자 이야기 2 전체가 이번 요청의 최소 제공 범위다.

## 6단계: 사용자 이야기 3 — 결과 종류 표식 색상

- [x] T011 [US3] `src/search_panel.rs`, `src/theme.rs`, `src/ui.rs`의 결과 표시와 색상을 분석하고 명세·계획을 갱신한다.
- [x] T012 [US3] `src/search_panel.rs`의 표식과 본문을 분리해 F에 민트, C에 노랑을 적용하고 본문 생략과 이벤트를 유지한다. (FR-008, FR-009)
- [x] T013 [US3] 검색 테스트, 빌드·정적 분석 및 diff 검사를 실행하고 `validation.md`에 결과를 기록한다.
- [x] T014 `src/search_panel.rs`와 추가 요구사항을 수렴 점검한다.

의존성: T011 → 일관성 분석 → T012 → T013 → T014. 단일 렌더링 수정이므로 순차 수행한다. 사용자 이야기 3이 이번 최소 제공 범위다.
