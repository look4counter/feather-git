# 작업 목록

## 1단계: 준비

- [x] T001 src/ui.rs·src/diff_viewer.rs·src/text_selection.rs의 호출 관계와 좌표·드래그 처리를 조사한다.

## 2단계: 기반

- [x] T002 specs/040-viewer-word-selection/spec.md·plan.md·contracts/ui.md의 요구사항과 경계 조건을 일관성 분석한다.

## 3단계: 사용자 이야기 1

독립 검증: 같은 기록 행 클릭 전후 파일·조회·뷰어·스크롤이 보존된다.

- [x] T003 [US1] src/ui.rs에 동일 행 재선택 보존 및 최초 전환 회귀 테스트를 추가해 기존 실패를 확인한다. (FR-001, FR-002)
- [x] T004 [US1] src/ui.rs의 select_history_row에 재선택 차단을 추가하고 변경된 요구사항에 영향받는 기존 테스트를 갱신한다. (FR-001, FR-002)

## 4단계: 사용자 이야기 2

독립 검증: 단어 범위와 복사 문자열이 유니코드·기호·탭·빈 줄에서 일치한다.

- [x] T005 [US2] src/text_selection.rs에 단어 선택·복사 경계 테스트를 추가하고 구현 전 실패를 확인한다. (FR-003~005)
- [x] T006 [US2] src/text_selection.rs에 문자 단위 단어 선택을 구현한다. (FR-003, FR-005)
- [x] T007 [US2] src/diff_viewer.rs에서 더블클릭 좌표 변환·선택과 드래그 차단을 연결하고 탭·한글 좌표 변환을 테스트한다. (FR-003~006)

## 5단계: 검증 및 수렴

- [x] T008 전체 테스트·서식·정적 분석·빌드를 실행하고 specs/040-viewer-word-selection/validation.md에 결과와 미검증 범위를 기록한다.
- [x] T009 specs/040-viewer-word-selection/spec.md·plan.md·tasks.md와 구현의 수렴을 검토하고 잔여 작업을 완료한다.

## 의존성과 구현 전략

T001 → T002 → T003 → T004 → T005 → T006 → T007 → T008 → T009 순서로 진행한다. 사용자 이야기 1을 먼저 완결한 뒤 사용자 이야기 2를 구현한다. T003과 T005의 테스트 작성은 서로 다른 파일에서 독립 가능하지만 이번에는 순차 수행한다. 전체 검증 후 미해결 항목을 다시 구현한다.
