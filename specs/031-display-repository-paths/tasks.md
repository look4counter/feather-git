# 작업 목록: 표시용 저장소 경로

## 1단계: 준비

- [x] T001 src/ui.rs·src/diff_viewer.rs·src/session.rs의 표시/입력/저장/재열기 경로를 plan.md에 분석한다.

## 2단계: 사용자 이야기 1

- [x] T002 [US1] src/path_display.rs에 기존 표시 동작과 드라이브·UNC·일반/경계·원본 보존 테스트를 먼저 연결하여 기대 변환 실패를 확인한다. src/main.rs에 모듈을 등록한다. FR-001~003/005, SC-001.
- [x] T003 [US1] 표시 전용 변환을 구현하고 src/ui.rs의 최근/현재 경로 및 src/diff_viewer.rs의 명시적 저장소 경로 문구에 연결한다. FR-001~004.

## 3단계: 사용자 이야기 2

- [x] T004 [US2] src/ui.rs 테스트에서 표시 안내와 내부 entry_request/session_snapshot의 경로 보존을 함께 확인한다. FR-004~005, SC-002.

## 4단계: 검증과 수렴

- [x] T005 전체 테스트·포맷·정적 분석·빌드를 실행하고 specs/031-display-repository-paths/validation.md에 결과와 제한을 기록한다. SC-003.
- [x] T006 명세·계획·작업과 실제 코드의 수렴을 점검한다.

## 의존성과 실행 전략

T001 → 일관성 분석 → T002 → T003 → T004 → T005 → T006 순서다. 표시용 함수만 먼저 검증하고 UI를 연결한다. 저장/입력 데이터와 백엔드는 변경하지 않는다.
