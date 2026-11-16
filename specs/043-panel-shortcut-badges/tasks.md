# 작업 목록

## 1단계: 준비

- [x] T001 src/ui.rs·src/diff_viewer.rs·src/diff_git.rs·src/backend.rs의 포커스·하단·입력·쓰기 흐름을 분석한다.

## 2단계: 기반

- [x] T002 specs/043-panel-shortcut-badges/spec.md·plan.md·contracts/ui.md의 요구사항과 일관성을 확인한다.

## 3단계: 사용자 이야기 1

독립 검증: 파일 패널 안내와 Stage/Unstage 왕복·작업 파일 보존이 일치한다.

- [x] T003 [US1] src/diff_git.rs와 src/backend.rs에 잠금·stale 검사·기존 필터를 이용한 전체 파일 Stage 토글을 추가하고 실제 저장소 회귀 테스트를 작성한다. (FR-003, FR-004)
- [x] T004 [US1] src/ui.rs에 파일 패널 Space 가드·비동기 실행·busy·오류·선택 보존 갱신을 연결하고 요청 정책을 테스트한다. (FR-002~004, FR-007)

## 4단계: 사용자 이야기 2

독립 검증: 뷰어 안내는 구간 선택 가능한 직접 포커스에만 보이며 기존 내부 문구는 없다.

- [x] T005 [US2] src/diff_viewer.rs에 읽기 전용 안내 정책과 busy 접근자를 추가하고 기존 내부 안내를 제거하며 정책을 테스트한다. (FR-005, FR-006)
- [x] T006 [US2] src/ui.rs에 포커스 구독·뷰어 관찰·하단 키 배지와 설명을 연결하고 기존 상태 표시를 보존한다. (FR-001, FR-005, FR-007)

## 5단계: 검증 및 수렴

- [x] T007 전체 테스트·서식·Clippy·빌드 결과와 GUI 한계를 specs/043-panel-shortcut-badges/validation.md에 기록한다.
- [x] T008 specs/043-panel-shortcut-badges/spec.md·plan.md·tasks.md와 구현의 수렴을 확인하고 미완료 작업을 완료한다.

## 의존성 및 구현 전략

T001 → T002 → T003 → T004 → T005 → T006 → T007 → T008 순서로 진행한다. UI와 백엔드의 경계를 유지하고 파일 동작을 먼저 구현한 뒤 안내를 연결한다. T003과 T005의 독립 테스트는 병렬 가능하나 순차 수행한다. 두 이야기 완료 후 회귀와 수렴을 확인한다.

## 6단계: 콜론 제거 보완

- [x] T009 specs/043-panel-shortcut-badges/spec.md·plan.md·contracts/ui.md에 표시 형식 요구사항을 반영하고 모호성·일관성을 확인한다. (FR-001)
- [x] T010 src/ui.rs의 공통 단축키 안내에서 콜론 접두부를 제거한다. (FR-001)
- [x] T011 서식·컴파일과 코드 수렴을 검증하고 specs/043-panel-shortcut-badges/validation.md에 기록한다.

보완 작업은 T009 → T010 → T011 순서로 수행한다.
