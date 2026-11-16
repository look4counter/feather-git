# 작업 목록

## 1단계: 분석

- [x] T001 diff/부분 쓰기의 원문 비교·토크나이저·EOF·텍스트 hit를 분석하고 명세·명확화·계획·작업 일관성을 확인한다.

## 2단계: 비교와 쓰기

- [x] T002 src/diff_git.rs·src/partial_stage.rs의 교차 개행 재현 테스트를 추가해 실패를 확인한다. (FR-001~003)
- [x] T003 src/line_endings.rs·src/main.rs·src/diff_git.rs·src/partial_stage.rs에 공통 비교 정규화와 원문/개행 보존을 구현하고 검증한다. (FR-001~003)

## 3단계: 빈 줄

- [x] T004 src/diff_viewer.rs의 빈/공백 본문 행 hit를 허용하고 경계 테스트를 갱신한다. (FR-004)

## 4단계: 검증

- [x] T005 전체 테스트·서식·Clippy·빌드와 GUI 한계를 validation.md에 기록한다.

의존성: T001 → T002 → T003 → T004 → T005. 구현 후 수렴 점검을 수행한다.
