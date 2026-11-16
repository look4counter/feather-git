# 작업 목록: 재사용 alert 팝업

## 1단계: 준비

- [x] T001 기존 버튼, 렌더링·포커스 API를 조사하고 specs/004-alert-component/research.md에 기록한다.

## 2단계: 재사용 컴포넌트

- [x] T002 [US2] src/theme.rs에 기존 색상을 추출하고 src/alert.rs에 사용자 지정 내용, 모달 UI·확인·키보드 닫기를 구현한다. src/main.rs에 등록한다. FR-001, FR-003~005 대응.

## 3단계: Create 연결

- [x] T003 [US1] src/ui.rs에 alert 단일 메모리 상태, Create 진입, 포커스 이동·복원과 마지막 오버레이 렌더링을 구현한다. FR-002~004 대응.
- [x] T004 [US1] src/ui.rs와 src/alert.rs에 상태·문자열·키보드 닫기 판별 테스트를 추가한다. SC-001, SC-003 대응.

## 4단계: 검증

- [x] T005 포맷·컴파일·전체 테스트·빌드를 수행하고 specs/004-alert-component/validation.md에 명세 일치와 미검증 범위를 기록한다. SC-001~004 대응.

T001 → T002 → T003 → T004 → T005 순서로 진행한다. 동일 모듈 등록과 UI 상태에 영향이 있어 순차 수행한다.

## 5단계: 수렴 검증

- [ ] T006 화면 확인 가능한 환경에서 src/alert.rs와 src/ui.rs의 Create 팝업, 확인·Enter·Esc 닫기, 배경 클릭 차단, 긴 메시지 레이아웃과 이전 포커스 복원을 실제 조작하고 specs/004-alert-component/validation.md에 기록한다. SC-001~002 및 SC-004 실제 표시·입력 검증 미완료에 대응한다. 기존 화면 도구 실행 파일 사용 승인 제한이 남아 있다.
