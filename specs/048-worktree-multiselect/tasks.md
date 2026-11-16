# 작업 목록

## 1단계: 준비

- [x] T001 src/virtual_list.rs·src/ui.rs·src/diff_viewer.rs·src/workspace_backend.rs의 선택·조회·쓰기·포커스 흐름을 분석한다.

## 2단계: 기반

- [x] T002 specs/048-worktree-multiselect/spec.md·plan.md·contracts/ui.md를 두 명확화 답변과 대조해 일관성을 확인한다.

## 3단계: 사용자 이야기 1

독립 검증: 수정키별 선택 집합과 강조·단일 모드·잘못된 입력이 정확하다.

- [x] T003 [US1] src/virtual_list.rs에 MultiSelection과 선택·범위·해제·복원 경계 테스트를 추가한다. (FR-001, FR-002)
- [x] T004 [US1] src/virtual_list.rs의 다중 강조·마우스 이벤트 API를 연결하고 기존 단일 API를 보존한다. (FR-001, FR-007)
- [x] T005 [US1] src/ui.rs의 작업 트리 수정키 선택과 커밋 기록 단일 선택을 연결하고 상태·stale 이벤트를 테스트한다. (FR-001, FR-002, FR-007)

## 4단계: 사용자 이야기 2

독립 검증: 여러 파일 대상과 자식별 diff·포커스·조회·쓰기 경계가 보존된다.

- [x] T006 [US2] src/ui.rs·src/diff_viewer.rs에 목록 순서의 Files 대상과 파일별 뷰어 구성을 추가하고 대상 대응을 테스트한다. (FR-003)
- [x] T007 [US2] src/diff_viewer.rs에 자식 재사용·이벤트 전달·busy·안내·차단 및 쓰기 중 수명 보존을 연결한다. (FR-004, FR-007)

## 5단계: 사용자 이야기 3

독립 검증: 선택 파일 전체 토글·중복 경로 Stage 우선·일부 실패·선택 복원이 정확하다.

- [x] T008 [US3] src/workspace_backend.rs에 파일별 배치 토글·중복 제거·결과와 실제 저장소 테스트를 추가한다. (FR-005, FR-006)
- [x] T009 [US3] src/workspace_backend.rs·src/ui.rs의 선택 식별자를 복수로 전환하고 재정렬 후 복원을 테스트한다. (FR-006)
- [x] T010 [US3] src/ui.rs의 Space를 비동기 배치와 성공·실패 안내에 연결하고 기존 커밋·그래프 갱신 차단을 유지한다. (FR-005~007)

## 6단계: 검증 및 수렴

- [x] T011 전체 테스트·서식·Clippy·빌드 결과와 GUI 한계를 specs/048-worktree-multiselect/validation.md에 기록한다.
- [x] T012 specs/048-worktree-multiselect/spec.md·plan.md·tasks.md와 코드 수렴을 확인하고 남은 작업을 완료한다.

## 의존성과 전략

T001 → T002 → T003 → T004 → T005 → T006 → T007 → T008 → T009 → T010 → T011 → T012 순서로 수행한다. 선택 모델을 먼저 검증하고 뷰어와 배치를 순서대로 연결한다. 독립 파일의 선택 모델·배치 테스트 작성은 병렬 가능하나 이번 작업은 순차 수행한다.

## 7단계: 수렴 보완

- [x] T013 src/diff_viewer.rs에서 선택에서 빠진 쓰기 중 자식의 실패 메시지도 부모에 보존하고 다중 화면에서 표시한다. (FR-004, 부분 구현)

## 8단계: 다중 선택 헤더 단순화

- [x] T014 후속 요구사항을 spec.md·plan.md·contracts/ui.md·quickstart.md에 반영하고 명확화 및 일관성을 확인한다. (FR-003)
- [x] T015 src/diff_viewer.rs에서 다중 선택 자식의 경로 헤더와 Stage 상태 행을 숨기고 개수 헤더 및 단일 선택 헤더를 유지한다. (FR-003)
- [x] T016 서식·전체 테스트·Clippy·빌드를 실행하고 validation.md에 결과를 기록한다. (FR-003, FR-004, FR-007)

의존성: T014 → T015 → T016. 이후 수렴 점검에서 FR-003의 표시 정책과 기존 본문/단일 선택 보존을 확인한다.

## 9단계: 헤더 디자인 통일

- [x] T017 기존 두 헤더를 분석하고 FR-008의 명세·명확화·계획·작업 일관성을 확인한다.
- [x] T018 src/diff_viewer.rs의 개수 헤더 스타일을 파일명 헤더와 맞추고 하단 선을 추가한다. (FR-008)
- [x] T019 서식·Clippy 검증과 FR-008 수렴 점검 결과를 validation.md에 기록한다.

의존성: T017 → T018 → T019.
