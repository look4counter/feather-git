# 작업 목록

## 1단계: 기반
- [x] T001 src/committing.rs의 전송 함수를 재사용 가능하게 하고 src/workspace_backend.rs에 툴바 작업 요청/결과와 재조회 경로를 추가한다.

## 2단계: Push (사용자 이야기 1)
- [x] T002 [US1] src/branch_actions.rs에 현재 브랜치 비강제 푸시와 최초 upstream 설정 및 격리 원격 테스트를 구현하고 src/repository.rs, src/backend.rs에 연결한다.
- [x] T003 [US1] src/ui.rs와 src/split_button.rs에 Push 버튼·비동기 실행·전역 진행·결과 재조회 및 오래된 결과 검증을 연결한다.

## 3단계: New Branch (사용자 이야기 2)
- [x] T004 [US2] src/branch_actions.rs에 이름·중복·HEAD·진행 상태 검사, 브랜치 생성/선택적 전환 및 파일/인덱스 보존 테스트를 구현한다.
- [x] T005 [US2] src/branch_dialog.rs와 src/main.rs에 공용 입력·체크박스·버튼·키보드 처리 팝업을 구현한다.
- [x] T006 [US2] src/ui.rs와 src/split_button.rs에 Branch 버튼·팝업 수명·포커스·초안 보존·부분 성공 안내를 연결하고 결과 처리를 검증한다.

## 4단계: 통합 검증
- [x] T007 전체 테스트·포맷·엄격한 clippy·별도 실행파일 빌드를 실행하고 specs/067-push-new-branch/quickstart.md에 결과를 기록한다.
- [x] T008 specs/067-push-new-branch/spec.md와 plan.md 및 구현을 수렴 점검하여 불일치를 해결한다.

## 의존성 및 실행 전략
T001→T002→T003→T004→T005→T006→T007→T008 순서로 구현한다. Push를 먼저 검증하고 Branch를 통합한다. 독립 조사만 병렬 가능하며 코드 변경은 순차 수행한다. 사용자 저장소의 실제 Git 작업은 실행하지 않는다.
