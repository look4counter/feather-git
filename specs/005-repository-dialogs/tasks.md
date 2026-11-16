# 작업 목록: 저장소 설정 팝업

## 1단계: 준비

- [x] T001 기존 UI/데이터, GPUI 입력과 old 탐색 흐름을 조사하고 specs/005-repository-dialogs/research.md에 기록한다.

## 2단계: 저장소 서비스

- [x] T002 [US1] src/repository.rs에 요청/프로젝트 모델과 경로·생성·열기·복제/오류·파일 보존 테스트를 먼저 작성한다. FR-002, FR-004~008 대응.
- [x] T003 [US1] src/repository.rs의 비파괴 검증·Git 작업과 실제 스냅샷 로딩을 구현하고 src/main.rs에 등록한다. SC-002, SC-004 대응.

## 3단계: 공통 입력과 폼

- [x] T004 [US3] src/text_input.rs에 GPUI 단일행 입력/IME/클립보드/변경 이벤트와 경계 테스트를 구현한다. FR-003 대응.
- [x] T005 [US3] src/repository_dialog.rs에 모드별 공통 폼, 경로→이름 구독, 탐색·확인·취소와 포커스/키보드·busy를 구현한다. FR-001~003, FR-007, FR-009 대응.

## 4단계: 앱 연결

- [x] T006 [US1] src/ui.rs에 동적 프로젝트와 실제 패널·그래프·빈 상태를 연결하고 기존 샘플을 같은 모델로 유지한다. FR-008 대응.
- [x] T007 [US2] src/ui.rs의 세 시작 버튼·폴더 탐색·백그라운드 작업·Alert·성공 전환을 연결하고 전환/중복 테스트를 추가한다. FR-004~009 대응.

## 5단계: 검증

- [x] T008 포맷·컴파일·전체 테스트·빌드를 수행하고 specs/005-repository-dialogs/validation.md에 요구사항 대응과 미검증 범위를 기록한다. SC-001~005 대응.

T001 → T002 → T003 → T004 → T005 → T006 → T007 → T008 순서로 진행한다. 같은 데이터/모듈 등록에 영향이 있어 순차 수행한다.

## 6단계: 수렴 검증

- [ ] T009 실제 Windows 창에서 src/repository_dialog.rs와 src/text_input.rs의 세 팝업 배치, 폴더 탐색/취소, 경로→이름 갱신·수동 이름, IME/클립보드/긴 경로, Tab/Shift+Tab·Enter/Esc, busy 중복 차단 및 오류 Alert 복귀/성공 탭 전환을 확인하고 specs/005-repository-dialogs/validation.md에 기록한다. SC-001, SC-003, SC-005 실제 화면 검증 미완료에 대응한다. 기존 화면 도구 실행 파일 사용 승인 제한이 남아 있다.
- [ ] T010 테스트 가능한 원격 Git 환경에서 src/repository.rs의 HTTP/SSH 소스 검증·인증 및 성공/실패 Clone을 확인하고 specs/005-repository-dialogs/validation.md에 기록한다. FR-006~007과 SC-005의 원격 환경 검증 미완료에 대응한다. 현재 로컬/file 주소 복제만 자동화 검증했다.

## 7단계: Open 성공 안내 제거

- [x] T011 [US1] src/ui.rs의 Open 성공 Alert 분기만 제거하고 직접 프로젝트 전환과 실패 Alert를 유지한다. FR-010, SC-006 대응.
- [x] T012 포맷·컴파일·기존 전체 테스트·정적 검사·빌드를 수행하고 specs/005-repository-dialogs/validation.md에 결과와 미검증 범위를 기록한다.

T011 → T012 순서로 수행한다. 기존 수렴 검증의 미완료 시각/원격 항목은 완료 처리하지 않는다.
