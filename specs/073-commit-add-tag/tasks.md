# 작업 목록

## 1단계 준비
- [x] T001 기존 메뉴·이름 팝업·태그·Push·작업 완료 경로를 조사하고 plan.md에 반영한다.

## 2단계 태그 생성 이야기
- [x] T002 [US1] src/tag_actions.rs에 정확한 커밋의 경량 태그 생성·이름 및 중복 검증과 격리 테스트를 추가하고 src/repository.rs, src/backend.rs에서 노출한다. FR-003.
- [x] T003 [US1] src/branch_dialog.rs의 태그 모드와 태그 확인 이벤트·Push 체크박스·고정 대상 표시를 구현한다. FR-002.
- [x] T004 [US1] src/ui.rs에서 커밋 메뉴·팝업 소유자·대상 고정·취소·busy·완료 처리를 연결하고 src/workspace_backend.rs에 CreateTag를 통합한다. 상태 검증 테스트를 추가한다. FR-001, FR-005.

## 3단계 원격 이야기
- [x] T005 [US2] src/committing.rs의 단일 참조 Push를 태그에도 재사용하고 src/tag_actions.rs에서 원격 정책·부분 성공을 구현한다. 로컬 bare 원격으로 태그 하나 전송·설정 원격·충돌·로컬 보존을 검증한다. FR-004.

## 4단계 검증
- [x] T006 형식·전체 테스트·컴파일·엄격한 clippy·별도 실행 파일 빌드 후 quickstart.md에 결과와 한계를 기록하고 수렴 점검한다.

## 의존성과 실행 전략
T001 → T002 → T003 → T004 → T005 → T006. 테스트를 포함한 기능 단계 순차 구현이며 공용 Push 경로 변경 후 기존 Push 회귀도 검사한다. 독립 병렬 구현은 없다.
