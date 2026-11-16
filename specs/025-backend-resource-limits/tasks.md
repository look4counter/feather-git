# 작업 목록: 백엔드 자원 사용 제한

## 1단계: 준비

- [x] T001 기존 호출·데이터 흐름과 헌법 템플릿 및 검증 환경을 specs/025-backend-resource-limits/plan.md에 분석한다.

## 2단계: 공통 기반

- [x] T002 src/backend_worker.rs에 실행 상한·취소·교체·완료 깨우기·패닉 복구 재현 테스트를 먼저 작성한다. FR-001~004, FR-007, SC-001~003.
- [x] T003 src/backend_worker.rs와 src/backend.rs에 전용 풀과 취소 가능한 gateway 지연을 구현한다. FR-001~004.

## 3단계: 사용자 이야기 1

- [x] T004 [US1] src/ui.rs에 패널·참조 조회 슬롯을 연결하고 무효화·선택 변경 취소 및 회귀 테스트를 추가한다. FR-002~004.
- [x] T005 [US1] src/diff_viewer.rs에 조회 슬롯을 연결하고 대상 변경·비활성화 시 취소 및 기존 결과 검증을 유지한다. FR-002~004.

## 4단계: 사용자 이야기 2

- [x] T006 [US2] vendor/gpui/src/platform/windows/dispatcher.rs에서 일반 우선순위를 사용한다. FR-005.
- [x] T007 [US2] src/diff_git.rs에서 상태 조회 내부 작업 수를 제한한다. FR-006.

## 5단계: 검증

- [x] T008 전체 테스트·빌드·포맷·정적 분석을 수행하고 specs/025-backend-resource-limits/validation.md에 결과와 한계를 기록한다. FR-007, SC-001~003.
- [x] T009 specs/025-backend-resource-limits의 명세·계획·작업과 코드를 수렴 점검한다.

## 의존성과 실행 전략

T001 → T002 → T003 → T004 → T005 → T006/T007 → T008 → T009 순서다. 공통 기반 후 US1은 최신 조회만 남기는지 독립 검증하고 US2는 실행 상한과 일반 작업 보존을 검증한다. T006과 T007은 별도 파일이므로 병렬 실행 가능하나 이번 작업은 순차 수행한다.
