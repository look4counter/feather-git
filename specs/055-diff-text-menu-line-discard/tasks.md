# 작업 목록

## 1단계: 분석

- [x] T001 텍스트 hit·복사·부분 Stage·쓰기·확인 Dialog·다중 이벤트를 분석하고 명세·명확화·계획·작업 일관성을 확인한다.

## 2단계: 텍스트 메뉴

- [x] T002 src/diff_viewer.rs의 패널 Right를 제거하고 실제 텍스트 hit·공통 복사·조건부 체크를 연결해 경계를 검증한다. (FR-001~003)

## 3단계: 부분 폐기

- [x] T003 src/partial_stage.rs·src/diff_git.rs·src/backend.rs에서 공통 알고리즘/검증과 안전한 부분 폐기를 구현하고 임시 저장소 테스트를 추가한다. (FR-004~006)
- [x] T004 src/diff_viewer.rs·src/ui.rs에 버튼·확인 이벤트/다중 전달·기존 Dialog·취소/세대/실행/갱신을 연결한다. (FR-004~006)

## 4단계: 검증

- [x] T005 전체 테스트·서식·Clippy·빌드와 GUI/클립보드 검증 한계를 validation.md에 기록한다.

의존성: T001 → T002 → T003 → T004 → T005. 구현 뒤 수렴을 점검한다.
