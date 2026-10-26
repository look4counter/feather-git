# 작업 목록: 단일 화면 Git 클라이언트

**입력**: [명세](spec.md), [계획](plan.md), [조사](research.md), [모델](data-model.md), [계약](contracts/ui.md)

## 1단계: 준비

- [x] T001 Cargo.toml, .gitignore에서 Rust 의존성과 빌드 제외 규칙을 구성한다.

## 2단계: 공통 기반

- [x] T002 src/model.rs, src/lib.rs에서 조회 결과와 프로젝트별 선택 모델을 정의한다.
- [x] T003 src/git.rs에서 저장소 열기, 오류 및 크기·종류 제한 기반을 구현한다.

## 3단계: 프로젝트 화면 (US1)

**목표**: 프로젝트 두 개를 독립적인 상태로 전환한다.
**독립 검증**: 두 저장소를 열고 탭 전환·닫기·빈 상태를 확인한다.

- [x] T004 [US1] src/main.rs에서 창과 저장소 실행 인자를 구성한다. FR-001, FR-008.
- [x] T005 [US1] src/ui.rs에서 프로젝트 열기·닫기·전환, 네 영역, 배경 조회, 로딩·오류를 구현한다. FR-001, FR-002, FR-005, FR-007, FR-010.

## 4단계: 참조와 이력 (US2)

**목표**: 실제 참조와 커밋을 탐색한다.
**독립 검증**: Local/Remote/Stash/Tag와 참조별 이력, 빈 저장소를 확인한다.

- [x] T006 [US2] src/git.rs에서 참조·stash reflog·커밋 이력 조회를 구현한다. FR-003, FR-006, FR-009.
- [x] T007 [US2] src/ui.rs에서 참조 탭·선택 및 커밋 목록·선택을 연결한다. FR-003, FR-006.

## 5단계: 변경과 차이 (US3)

**목표**: 작업 변경과 커밋 변경의 파일 및 차이를 확인한다.
**독립 검증**: Stage 양쪽 문맥과 History에서 파일 선택 및 Diff를 확인한다.

- [x] T008 [US3] src/git.rs에서 상태, 첫 부모 트리 비교, 파일 Diff, 바이너리·크기·특수 파일 처리를 구현한다. FR-004, FR-006, FR-007, FR-011.
- [x] T009 [US3] src/ui.rs에서 Stage/History 파일 목록과 행 번호·색상 Diff를 연결한다. FR-004, FR-006, FR-011.

## 6단계: 마무리

- [x] T010 tests/repository.rs에서 임시 실제 저장소로 읽기 전용 조회와 경계 조건을 검증한다.
- [x] T011 README.md와 specs/001-git-client/quickstart.md에 실행 방법·제약·실제 검증 결과를 기록하고 fmt/check/clippy/test를 수행한다.
- [x] T012 specs/001-git-client/tasks.md를 기준으로 speckit-converge를 수행하고 남은 작업을 완료한다.

## 의존성과 실행 전략

T001 → T002 → T003 → US1 → US2 → US3 → 마무리 순으로 수행한다. 최소 동작 화면을 먼저 구성하고 실제 조회를 연결한다. 같은 파일을 다루는 구현은 순차 실행한다. 독립 파일 작업인 README 작성과 조회 통합 검증은 병렬 가능하지만 이번 구현은 순차 진행한다. 검증 명령은 [실행 안내](quickstart.md)를 따른다.

## 7단계: 수렴 보완

- [x] T013 src/git.rs에서 커밋을 최신 시각순으로 정렬하고 tests/repository.rs에서 200개 제한을 확인한다. FR-009 (partial).
- [x] T014 src/git.rs에서 변경 구간 헤더를 포함한 Diff 5,000줄 상한을 보장하고 tests/repository.rs에서 특수 파일·충돌·크기 제한 검증을 보강한다. FR-011 (partial).
- [x] T015 src/ui.rs에서 조회 실패 시 이전 조회 결과를 비워 오류 문맥의 혼동을 방지한다. FR-006, FR-007 (partial).

## 8단계: 여백 제거 및 크기 조절 (US1)

**목표**: 네 영역을 맞붙이고 공통 경계선으로 크기를 조절한다.
**독립 검증**: 가로·세로 드래그, 최소 크기, 드래그 종료, 창 크기 변경과 프로젝트 전환 시 비율 유지를 확인한다.

- [x] T016 [US1] src/layout.rs에서 분할 비율, 좌표 변환 및 가용 공간별 최소 크기 계산과 수치 검증을 구현한다. FR-014.
- [x] T017 [US1] src/ui.rs와 src/main.rs에서 본문 여백·패널 테두리·둥글림을 제거하고 정렬된 분할 레이아웃을 연결한다. FR-012, SC-005.
- [x] T018 [US1] src/ui.rs에서 실제 본문 Bounds, 7px 경계선 판정, 방향 커서, 드래그 시작·이동·종료 및 창 공통 상태를 구현한다. FR-013, FR-014.
- [x] T019 README.md와 specs/001-git-client/quickstart.md에 조절 방법 및 검증 결과를 기록하고 fmt/check/clippy/test를 수행한다.

**실행 순서**: T016 → T017 → T018 → T019. 같은 UI 파일을 다루므로 순차 수행한다. 기존 조회 기능은 회귀 테스트로 검증한다.

## 9단계: 전체 높이 탭 바와 빈 탭 (US1)

**목표**: 상단을 도구 버튼 없는 탭 바로 바꾸고 오른쪽 +로 빈 탭을 연다.
**독립 검증**: 빈 탭 추가·선택·닫기 및 기존 저장소 선택 상태 보존을 확인한다.

- [x] T020 [US1] src/ui.rs에서 WorkspaceTab의 빈 탭·저장소 탭 구분과 고유 ID, 선택·닫기·마지막 탭 복구를 구현한다. FR-017.
- [x] T021 [US1] src/ui.rs에서 상단을 44px 전체 높이의 사각 탭과 고정 +로 구성하고 로고·새로고침·저장소 열기 및 폴더 선택 처리를 제거한다. FR-008, FR-015, FR-016.
- [x] T022 [US1] src/ui.rs에서 빈 탭 본문을 패널·상태줄 없는 페이지로 표시하고 저장소 비동기 조회의 대상을 저장소 탭으로 유지한다. FR-002, FR-005, FR-016.
- [x] T023 README.md와 specs/001-git-client/quickstart.md에 새 동작 및 검증 결과를 기록하고 fmt/check/clippy/test를 수행한다.

**실행 순서**: T020 → T021 → T022 → T023. 같은 파일의 상태와 렌더링을 다루므로 순차 수행한다. 기존 전체 테스트 14개로 조회 및 분할 동작을 회귀 검증한다.

## 10단계: 닫기 hover와 저장소 시작 (US4)

목표: 세 선택지로 저장소를 시작한다. 독립 검증: 생성·열기·로컬 Clone·오류·취소·탭 전환을 확인한다.

- [x] T024 [US4] src/setup.rs와 src/main.rs에 기존 경로 보호 및 Git 초기화·Clone과 로컬 통합 검증을 구현한다. FR-020, FR-021.
- [x] T025 [US4] src/ui.rs에 24px 닫기와 세 선택지·URL·폴더 선택·탭별 진행/오류 및 같은 ID 결과 반영을 구현한다. FR-018~FR-022.
- [x] T026 [US4] README.md와 quickstart.md를 갱신하고 fmt/check/clippy/test 및 수렴 검토로 SC-007을 확인한다.

의존성: T024 → T025 → T026. 같은 상태를 수정하므로 순차 진행한다. 문서 검토는 독립적으로 가능하다.

## 11단계: 커밋 그리드·그래프·상세 (US5)

목표: 커밋 관계를 그리드에서 탐색하고 History 고정 항목으로 상세를 읽는다. 독립 검증: 병합 저장소와 긴 메시지·빈 저장소에서 그래프 및 상세 전환을 확인한다.

- [x] T027 [US5] src/model.rs·src/graph.rs·src/lib.rs에 부모 및 그래프 모델과 zit 레인 알고리즘을 구현하고 선형·병합·여러 부모·조회 제한 경계를 검증한다. FR-024, SC-008.
- [x] T028 [US5] src/git.rs와 tests/repository.rs에서 선택 커밋 상세를 조회하고 초기·병합·긴 메시지·빈 상태를 검증한다. FR-026, FR-027, SC-009.
- [x] T029 [US5] src/history_view.rs·src/main.rs·src/ui.rs에서 공통 다섯 열·24px 행·곡선 그래프 캐시와 스크롤 및 행 선택을 구현한다. FR-023, FR-024.
- [x] T030 [US5] src/model.rs·src/ui.rs에서 History 고정 커밋상세와 상세/파일 상호 선택·세대 보호·상세 렌더링을 구현한다. FR-025~FR-027.
- [x] T031 [US5] README.md·quickstart.md에 사용법·검증 결과를 기록하고 fmt/check/clippy/test 및 speckit-converge를 수행한다. SC-008, SC-009.

의존성: T027 → T028 → T029 → T030 → T031. 모델과 UI 변경은 순차 수행한다. 그래프 단위 검증과 저장소 통합 검증은 독립 실행 가능하다.

## 12단계: 전체 이력·성능·열 조절 (US6)

목표: 전체 이력을 가상화하고 선택 비용을 줄이며 전체 폭에서 열을 조절한다. 독립 검증: 다중 브랜치·200개 초과 이력, 대규모 렌더 range, 열 드래그 및 날짜를 확인한다.

- [x] T032 [US6] src/model.rs·src/git.rs·src/graph.rs에서 모든 참조/HEAD의 전체 이력과 브랜치 tip 이름·시간대 및 상세/파일 전용 조회를 구현하고 tests/repository.rs에서 분기·원격·태그·detached·200개 초과·선택 독립성을 검증한다. FR-029, FR-030, FR-032, SC-010.
- [x] T033 [US6] src/history_view.rs에서 공통 비율 기반 열 폭·이웃 열 조절·최소 폭·날짜 시분 계산과 자동 검증을 구현한다. FR-030, FR-031, SC-011.
- [x] T034 [US6] src/ui.rs에서 uniform_list·프로젝트별 스크롤·가시 그래프 캐시·범위별 행 생성·브랜치 tip·전용 선택 조회를 연결한다. FR-028, FR-029, FR-032.
- [x] T035 [US6] src/ui.rs에서 실제 폭 측정과 열 경계 드래그·해제·창 변경 처리를 연결하여 헤더와 행을 100% 정렬한다. FR-031.
- [x] T036 [US6] examples/history_profile.rs·README.md·quickstart.md에 원인과 측정·검증 결과를 기록하고 fmt/check/clippy/test 및 speckit-converge를 수행한다. SC-010, SC-011.

의존성: T032 → T033 → T034 → T035 → T036. 같은 모델·UI 작업은 순차 진행한다. 순수 열 수치 검증과 저장소 통합 검증은 독립 실행 가능하다.

## 13단계: 공통 스크롤·점진 조회 (US7)

목표: 네 목록의 가상화와 스크롤 조작을 통일하고 커밋을 200개씩 읽는다.

- [x] T037 [US7] src/paging.rs·src/git.rs·src/model.rs·src/graph.rs·src/lib.rs에 200개 iterator 세션·연속 레인·현재 브랜치·종류 인덱스와 Stage 행 매핑을 구현하고 tests/repository.rs에서 페이지·그래프·체크아웃을 검증한다. FR-034, FR-036, SC-012.
- [x] T038 [US7] src/scrolling.rs·src/main.rs·src/ui.rs에 공통 스크롤 손잡이 계산·프레임·휠·클릭·전역 드래그와 수치 검증을 구현한다. FR-033, SC-013.
- [x] T039 [US7] src/ui.rs·src/history_view.rs에 하단 페이지 로드·중복 보호·오류 재시도·라벨 삭제·Graph 여백·현재 브랜치 표시를 연결한다. FR-034~FR-036.
- [x] T040 [US7] src/ui.rs에서 브랜치·파일·Diff·커밋상세를 가상화하고 공통 스크롤을 적용하며 Stage 그룹과 Diff 색상·줄번호를 보존한다. FR-037.
- [x] T041 [US7] examples/history_profile.rs·README.md·quickstart.md에 결과를 기록하고 fmt/check/clippy/test 및 speckit-converge로 누락을 점검한다. SC-012, SC-013.

의존성: T037 → T038 → T039 → T040 → T041. 같은 UI 상태는 순차 처리하며 수치 검증과 저장소 통합 검증은 독립 실행 가능하다.

## 14단계: 수렴 보완

- [x] T042 src/ui.rs에서 파일 Diff 오류가 History 파일 목록을 숨기지 않게 하고 이미 조회된 커밋상세를 선택하면 파일 Diff 오류를 해제한다. FR-037 (partial).

## 15단계: 목록 배치와 상세 정리 (US8)

- [x] T043 [US8] src/scrolling.rs·src/ui.rs에서 실제 프레임/콘텐츠 크기의 조건부 바·폭 캐시·헤더 정렬과 수치 검증을 구현한다. FR-038, SC-014.
- [x] T044 [US8] src/ui.rs에서 페이지 진행 오버레이·공통 24px 전체 폭 행·네 영역 제목 헤더 제거를 구현한다. FR-039, FR-040, FR-042.
- [x] T045 [US8] src/ui.rs에서 상위 항목과 시간·메시지 표시를 수정하고 초기/병합·시간대·빈 줄 회귀 검증을 수행한다. FR-041, SC-015.
- [x] T046 [US8] README.md·quickstart.md를 갱신하고 fmt/check/clippy/test 및 speckit-converge를 수행한다. SC-014, SC-015.

의존성: T043 → T044 → T045 → T046. 같은 UI 파일 작업은 순차 수행한다. 순수 스크롤 계산과 상세 문자열 검증은 독립 실행 가능하다.
