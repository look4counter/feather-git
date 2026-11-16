# 작업 목록: 공통 진행 표시

## 1단계: 준비

- [x] T001 폼 진행 UI와 탐색/Git 호출 관계를 조사하고 specs/007-shared-progress/research.md에 기록한다.

## 2단계: 공통 표시

- [x] T002 [US1] src/progress.rs에 메시지 지정 가능한 ProgressOverlay를 작성하고 src/main.rs에 등록한다. FR-001~002 대응.

## 3단계: 폼 연결

- [x] T003 [US2] src/repository_dialog.rs의 활동 상태/가드/포커스와 진행 컴포넌트를 연결하고 상태 계약 테스트를 추가한다. FR-002~003, SC-002 대응.
- [x] T004 [US2] src/ui.rs의 탐색/실제 작업/종료에 활동 상태를 적용하고 기존 Alert 및 결과 흐름을 유지한다. FR-003~004 대응.

## 4단계: 검증

- [x] T005 포맷·컴파일·전체 테스트·정적 검사·빌드를 실행하고 specs/007-shared-progress/validation.md에 결과와 제한을 기록한다. SC-001~003 대응.

T001 → T002 → T003 → T004 → T005 순서로 수행한다.

## 5단계: 공통 Dialog 기반

- [x] T006 [US3] src/dialog.rs에 재사용 모달 껍데기와 제목/본문/footer/overlay·포커스/키 연결 API를 작성하고 src/main.rs에 등록한다. FR-005 대응.
- [x] T007 [US3] src/repository_dialog.rs와 src/alert.rs를 공통 Dialog 기반으로 전환하고 기존 업무 이벤트·닫기·진행 표시를 유지한다. FR-006, FR-008 대응.
- [x] T008 [US3] src/ui.rs의 타이틀바와 본문 호스트를 분리하고 모든 모달을 본문 안에 배치한다. FR-007 대응.
- [x] T009 전체 검사·테스트·빌드와 구조 수렴 검토를 수행하고 specs/007-shared-progress/validation.md에 결과와 시각 미검증을 기록한다. SC-004~005 대응.

기존 T002~T005 완료 후 T006 → T007 → T008 → T009 순서로 진행한다.

## 6단계: 수렴 검증

- [ ] T010 화면 확인 가능한 환경에서 src/dialog.rs·src/progress.rs·src/ui.rs의 Alert/RepositoryDialog/중첩 오류 Alert와 진행 표시를 실제 조작해 타이틀바 비침범·창 크기 변경·창 제어, footer 우측 정렬, 키/포커스/입력 차단 및 탐색 시 진행 숨김을 확인하고 specs/007-shared-progress/validation.md에 기록한다. SC-003~005 실제 시각·입력 검증 미완료에 대응한다. 기존 화면 도구 실행 파일 사용 승인 제한이 남아 있다.

## 7단계: 깃털/물결 진행 애니메이션

- [x] T011 [US4] src/progress.rs에 깃털 낙하·수면 접촉·시간차 물결의 프레임 계산과 반복 벡터 렌더링을 적용하고 경계 테스트를 추가한다. FR-009~010, SC-006 대응.
- [x] T012 포맷·컴파일·전체 테스트·정적 검사·최종 실행 파일 빌드를 수행하고 specs/007-shared-progress/validation.md에 결과와 시각 미검증을 기록한다. 기존 Open 성공 Alert 제거도 최종 빌드에 포함한다.

T011 → T012 순서로 수행한다. 이전 T010 실제 시각 검증은 유지한다.

## 8단계: 애니메이션 수렴 검증

- [ ] T013 실제 작업 중 src/progress.rs의 깃털 낙하/수면 접촉/3겹 물결과 반복 경계의 시각적 자연스러움, 작업 종료 시 즉시 숨김을 확인하고 specs/007-shared-progress/validation.md에 기록한다. SC-006 실제 화면 검증 미완료에 대응한다. 기존 T010과 함께 확인하며 화면 도구 승인 제한이 남아 있다.
