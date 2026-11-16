# 작업 목록: 공통 가상화 목록

입력: spec.md, plan.md, research.md, data-model.md, contracts/list.md.

## 1단계: 준비

- [x] T001 src/ui.rs와 vendor/gpui/src/elements/uniform_list.rs의 호출·데이터·레이아웃 제약을 분석하고 specs/011-shared-virtual-list/plan.md에 반영한다.

## 2단계: 공통 기반

- [x] T002 src/virtual_list.rs에 대량·빈·단일·끝·감소·역전 범위 테스트를 먼저 작성하고 실패를 확인한다. (FR-002, FR-005, SC-002)
- [x] T003 src/virtual_list.rs에 범위 제한 및 공통 가상화 컴포넌트를 구현하고 src/main.rs에 등록한다. (FR-001, FR-002, FR-005)

## 3단계: 사용자 스토리 1 — 많은 항목 탐색

독립 검증: 요청 범위만 생성하는 자동 테스트 및 세 호출 지점 확인.

- [x] T004 [US1] src/ui.rs의 history를 공통 목록에 연결하고 기존 캐시·스크롤을 유지한다. (FR-001, FR-004)
- [x] T005 [US1] src/ui.rs의 branches/files를 현재 데이터의 요청 범위만 읽도록 연결하고 기존 행 스타일을 유지한다. (FR-001, FR-002, FR-003)

## 4단계: 사용자 스토리 2 — 기존 기능 유지

독립 검증: 기존 패널·프로젝트·커밋 회귀 테스트 및 정적 호출 검토.

- [x] T006 [US2] src/ui.rs의 식별자 분리·빈 상태·오류·전환 경계 테스트를 추가하고 기존 패널 및 커밋 회귀 테스트를 실행한다. (FR-003, FR-004, FR-005)

## 5단계: 검증 및 정리

- [x] T007 cargo test, cargo check, cargo fmt --check, cargo clippy --all-targets를 실행하고 specs/011-shared-virtual-list/validation.md에 결과와 미검증 범위를 기록한다. (SC-001, SC-003)

## 의존성 및 실행 전략

T001 → T002 → T003 → T004 → T005 → T006 → T007 순서로 수행한다. T004/T005는 같은 파일을 수정하므로 순차 실행한다. 공통 컴포넌트를 먼저 검증하고 각 화면 연결 후 전체 검증한다. 병렬 예시는 문서 검토와 소스 읽기이며 구현 작업 간 병렬 실행은 없다. 최소 기능은 스토리 1이며 최종 완료에는 스토리 2도 포함한다.

## 6단계: 사용자 스토리 3 — 공통 행 선택

- [x] T008 [US3] src/virtual_list.rs에 단일 선택·변경·반복·해제·빈 목록·범위 초과 테스트를 먼저 추가한다. (FR-006, FR-008)
- [x] T009 [US3] src/virtual_list.rs에 SingleSelection과 공통 행 클릭·선택 배경·호버 및 제어형 콜백을 구현한다. (FR-006, FR-007)
- [x] T010 [US3] src/ui.rs의 세 목록을 선택 API에 연결하고 커밋의 중복 이벤트·스타일을 제거하며 패널 선택을 데이터 갱신 시 해제한다. (FR-006, FR-007, FR-008)
- [x] T011 [US3] src/ui.rs의 패널 선택 독립성·해제·커밋 선택 가드 테스트와 전체 검증을 실행하고 specs/011-shared-virtual-list/validation.md에 기록한다. (FR-008, SC-004)

확장 순서: T008 → T009 → T010 → T011. 같은 파일 의존성 때문에 순차 수행한다. 독립 검증은 공통 선택 및 패널·커밋 회귀 테스트이다.

## 7단계: 사용자 스토리 4 — 즉시 선택 및 선택 우선 표시

- [x] T012 [US4] src/virtual_list.rs에서 미선택 행에만 호버 스타일을 등록하고 왼쪽 버튼 누름으로 선택 콜백을 연결한다. (FR-009, FR-010)
- [x] T013 [US4] src/virtual_list.rs의 조건·이벤트 연결과 기존 테스트·타입·포맷·Clippy·빌드를 검증하고 specs/011-shared-virtual-list/validation.md에 결과를 기록한다. (SC-005)

T012 → T013 순서이며 병렬 구현은 없다. 독립 수용 기준은 선택 행 호버 배경 유지와 왼쪽 버튼 누름 시점의 단일 콜백이다.

## 8단계: 사용자 스토리 5 — 공통 행 높이

- [x] T014 [US5] src/virtual_list.rs에 공통 기본 ROW_HEIGHT와 row_height 속성을 추가하고 공통 행 높이와 축소 방지를 적용한다. (FR-011, FR-012)
- [x] T015 [US5] src/ui.rs의 세 목록에 기존 높이를 전달하고 행 내용을 부모 높이에 맞추며 src/history_graph.rs에서 공통 상수를 재노출한다. (FR-011, FR-012)
- [x] T016 [US5] 기존 그래프·목록 테스트와 타입·포맷·Clippy·실행 파일 빌드를 검증하고 specs/011-shared-virtual-list/validation.md에 결과를 기록한다. (SC-006)

T014 → T015 → T016 순서로 수행한다. 독립 수용 기준은 목록별 기존 높이 유지와 그래프 연결점·노드 배치 회귀 없음이다. 같은 구현의 의존성 때문에 병렬 작업은 없다.

## 9단계: 공통 높이 자동 적용

앞선 T014~T016은 이전 요청에 대한 완료 기록이며 현재 구현은 다음 작업이 대체한다.

- [x] T017 [US5] src/virtual_list.rs의 개별 높이 인자를 제거하고 ROW_HEIGHT를 직접 적용하며 src/ui.rs 세 호출부의 높이 전달을 제거한다. (FR-013)
- [x] T018 [US5] 전체 테스트·타입·포맷·Clippy·빌드를 검증하고 specs/011-shared-virtual-list/quickstart.md와 validation.md를 갱신한다. (SC-007)

순서는 T017 → T018이며 병렬 작업은 없다. 독립 수용 기준은 모든 목록이 호출자 높이 지정 없이 24픽셀을 사용하는 것이다.
