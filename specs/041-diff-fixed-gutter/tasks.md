# 작업 목록: 고정 여백과 정렬

## 1단계: 준비

- [x] T001 src/diff_viewer.rs·src/virtual_list.rs의 렌더·좌표·스크롤·키 설정 흐름을 분석한다.

## 2단계: 기반

- [x] T002 specs/041-diff-fixed-gutter/spec.md·plan.md·contracts/ui.md의 일관성과 사용자 정정을 검토한다.

## 3단계: 사용자 이야기 1

독립 검증: 기호·번호·본문은 같은 글꼴·행 높이·기준선을 사용한다.

- [x] T003 [US1] src/diff_viewer.rs의 번호·기호를 본문과 같은 캔버스 텍스트로 렌더링한다. (FR-001)

## 4단계: 사용자 이야기 2

독립 검증: 왼쪽 고정 영역은 움직이지 않고 스크롤·선택·커서가 본문에서 일치한다.

- [x] T004 [US2] src/virtual_list.rs에 기본 동작을 보존하는 고정 여백 옵션과 본문 스크롤 범위·스크롤바 inset을 추가한다. (FR-002, FR-003, FR-005)
- [x] T005 [US2] src/diff_viewer.rs에서 본문 오프셋·클리핑·클릭·드래그·커서 노출을 연결한다. (FR-002~005)
- [x] T006 [US2] src/virtual_list.rs와 src/diff_viewer.rs의 가로 기하·경계·좌표·커서 노출 테스트를 추가한다. (FR-002~005)

## 5단계: 사용자 이야기 3

독립 검증: 저장된 Enter도 명령을 실행하지 않고 Stage 버튼은 유지된다.

- [x] T007 [US3] src/diff_viewer.rs의 Enter 기본·저장 바인딩과 안내를 제거하고 버튼 및 다른 키를 유지하며 회귀 테스트를 추가한다. (FR-006, FR-007)

## 6단계: 검증 및 수렴

- [x] T008 cargo fmt --check·cargo test --offline·cargo clippy --offline --all-targets·cargo build --offline 결과를 specs/041-diff-fixed-gutter/validation.md에 기록한다.
- [x] T009 specs/041-diff-fixed-gutter/spec.md·plan.md·tasks.md와 구현의 수렴을 확인하고 잔여 작업을 완료한다.

## 의존성과 전략

T001 → T002 → T003 → T004 → T005 → T006 → T007 → T008 → T009 순서로 수행한다. 같은 파일 변경은 순차 수행하고 새 의존성은 도입하지 않는다. 사용자 이야기 1은 렌더 기준 확인, 2는 가로 스크롤과 좌표, 3은 키 명령과 버튼을 독립 검증한다. 테스트 실행과 문서 검토는 독립적으로 할 수 있다.
