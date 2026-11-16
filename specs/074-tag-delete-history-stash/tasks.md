# 작업 목록

## 1단계 준비
- [x] T001 기존 삭제·Push·리스트·툴바·팝업 흐름을 분석하고 명세·계획에 반영한다.

## 2단계 원격 태그 삭제 이야기
- [x] T002 [US1] src/tag_actions.rs의 원격 선택을 공용화하고 src/reference_actions.rs에 원격 태그 삭제 옵션·기존 객체 보호·서버 미존재 처리를 추가한다. src/repository.rs, src/backend.rs, src/workspace_backend.rs를 연결하고 격리 원격 테스트를 추가한다. FR-001~002.
- [x] T003 [US1] src/ui.rs 태그 삭제 팝업에 기본 해제 체크박스·키보드 포커스·옵션 전달을 구현하고 상태 테스트를 추가한다. FR-001~002.

## 3단계 더블클릭 이야기
- [x] T004 [US2] src/ui.rs의 커밋 기록을 multi_virtual_list로 연결하고 더블클릭 대상 검증·기존 detached 확인 재사용 및 클릭 횟수 회귀 테스트를 추가한다. FR-003.

## 4단계 Stash 이야기
- [x] T005 [US3] src/stash_actions.rs에 이름·상태 검사 및 KEEP_INDEX 스태시 생성과 내용·오류 테스트를 작성하고 src/repository.rs, src/backend.rs, src/workspace_backend.rs에서 노출한다. FR-005~006.
- [x] T006 [US3] src/branch_dialog.rs의 Stash 모드와 src/split_button.rs의 아이콘, src/ui.rs의 Branch 오른쪽 버튼·팝업 생명주기·완료 처리를 연결한다. FR-004~006.

## 5단계 검증
- [x] T007 전체 테스트·형식·컴파일·엄격한 clippy·별도 빌드 후 quickstart.md에 결과·미검증을 기록하고 수렴 점검한다.

## 의존성과 실행 전략
T001 → T002 → T003 → T004 → T005 → T006 → T007. 공유 파일과 상태가 있으므로 순차 구현하며 각 이야기 테스트 후 통합한다. 독립 병렬 구현은 없다.
