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

## 16단계: 날짜·해시 기본 고정폭 (US9)

- [x] T047 [US9] src/history_view.rs에서 날짜·해시 기본 폭과 앞 세 열의 여유 공간 배분을 수정한다. FR-043.
- [x] T048 [US9] src/ui.rs에서 날짜·해시 셀에 고정폭 글꼴을 적용한다. FR-044.
- [x] T049 [US9] src/history_view.rs의 기존 레이아웃 검증을 확장하고 fmt/check/clippy/test 및 수렴 점검 결과를 specs/001-git-client/quickstart.md에 기록한다. SC-016.

의존성: T047 → T048 → T049. 동일 화면의 작은 변경이므로 순차 수행한다. 독립 검증은 창 확대 시 기본 폭 유지와 기존 수동 조절·좁은 창 테스트이다.

## 17단계: 세션 복구 (US10)

- [x] T050 [US10] src/session.rs와 Cargo.toml에 영속 세션·최근 목록·경로 표시 및 임시 파일 검증을 구현한다. FR-046, FR-047, FR-049, SC-017.
- [x] T051 [US10] src/main.rs와 src/ui.rs에 높이 790px·순서 보존 복구·탭 변경 저장을 연결한다. FR-045, FR-046.

## 18단계: 최근 저장소 관리 (US11)

- [x] T052 [US11] src/setup.rs·src/paging.rs에 삭제 대상 검사·조회 세션 종료·휴지통 이동을 구현하고 임시 경로 거절 사례를 검증한다. FR-048, SC-018.
- [x] T053 [US11] src/ui.rs에 최근 목록 다시 열기·목록 제거·삭제 확인·성공 시 탭 제거를 연결한다. FR-047, FR-048.

## 19단계: 도움말·fetch·진행 표시 (US12)

- [x] T054 [US12] src/setup.rs에 fetch를 구현하고 실제 로컬 원격에서 원격 정보 갱신·작업 트리 보존을 검증한다. FR-051, SC-018.
- [x] T055 [US12] src/ui.rs에 일반 표시 경로·하단 도움말·모달 다이얼로그와 키 처리 및 fetch 재조회를 연결한다. FR-049~FR-051.
- [x] T056 [US12] src/ui.rs에 모든 비동기 작업을 포괄하는 Progress 오버레이와 클릭·키·드래그 차단을 구현한다. FR-052, SC-019.
- [x] T057 fmt/check/clippy/test를 실행하고 specs/001-git-client/quickstart.md에 결과를 기록하며 수렴 점검한다. SC-017~SC-019.

의존성: T050 → T051 → T052 → T053 → T054 → T055 → T056 → T057. 소스 변경은 순차 진행한다. 독립 검증은 US10 상태 왕복, US11 삭제 대상 거절, US12 로컬 fetch·오버레이 검토이다. 모듈별 검증은 독립 가능하지만 UI 편집은 순차 수행한다.

## 20단계: 목록 키보드 탐색 (US13)

- [x] T058 [US13] src/ui.rs에 목록별 포커스와 ↑/↓ 선택·스크롤·그룹 건너뛰기를 구현하고 경계 계산을 검증한다. FR-053, SC-020.

T058은 T056 뒤 수행하고 T057의 최종 검증에 포함한다. 세 목록의 클릭과 기존 상세 조회를 재사용한다.

## 21단계: 읽기 전용 텍스트 선택 (US14)

- [x] T059 [US14] src/text_selection.rs와 src/ui.rs에 커서·Unicode 선택·드래그·키보드 범위·복사를 구현하고 원문 보존을 검증한다. FR-054, SC-021.

T059는 T058 뒤 수행하며 T057 검증과 수렴에 포함한다.

## 22단계: 수렴 보완

- [x] T060 [US13] src/ui.rs에서 고정 커밋상세 행 클릭에 파일 목록 포커스를 부여하고 ↑/↓로 첫 파일과 상호 이동하도록 보완한다. FR-053, SC-020 (partial).

## 23단계: 히스토리 진행 범위와 브랜치 위치 (US15)

- [x] T061 [US15] src/model.rs와 src/ui.rs에서 history_loading을 분리하고 Progress/입력 차단을 히스토리 로딩으로 한정한다. FR-055, SC-022.
- [x] T062 [US15] src/ui.rs 도움말을 `[단축키]`와 `f - 패치`만 남기고 닫기 버튼을 제거한다. FR-056.
- [x] T063 [US15] src/model.rs·src/ui.rs에 공통 브랜치 선택·대기 대상·200개 연속 페이지·성공 스크롤·오류 재시도·세대 보호를 연결한다. FR-057, FR-058.
- [x] T064 [US15] tests/repository.rs에서 페이지 경계·기존/없는 대상·재시도 상태를 검증하고 fmt/check/clippy/단위·통합 결과를 specs/001-git-client/quickstart.md에 기록하며 수렴 점검한다. SC-022, SC-023.

의존성: T061 → T062 → T063 → T064. 같은 UI 상태는 순차 편집한다. 최소 완료 범위는 세 요청 전체이다.


## 24단계: 정확한 참조와 Stage 변경 준비 (US16)

- [x] T065 [US16] src/git.rs에서 심볼릭 참조 원본 이름을 보존하고 tests/repository.rs에서 원격 HEAD·packed refs·커밋 배지를 검증한다. FR-059, SC-024.
- [x] T066 [US16] src/model.rs·src/ui.rs의 작업 디렉터리 헤더·두 칸 상태·staged/unstaged 색상을 수정하고 기존 행 매핑 검증을 갱신한다. FR-060, FR-061, SC-025.
- [x] T067 [US16] src/setup.rs·src/git.rs·src/ui.rs에 선택 파일 Space 스테이징·상태 재조회·선택 복원·중복/실패 처리를 연결하고 임시 저장소에서 실제 인덱스 변경을 검증한다. FR-062, SC-025.
- [x] T068 [US16] src/ui.rs에 F5 로컬 새로고침과 도움말 등록을 연결한다. FR-063, SC-026.
- [x] T069 fmt/check/clippy/단위·통합 검증 및 수렴 점검을 수행하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-024~SC-026.

의존성: T065 → T066 → T067 → T068 → T069. 파일 매핑과 UI 변경은 순차 처리한다. 참조와 실제 인덱스 검증은 서로 독립이다. 완료 범위는 추가 F5 요청까지 전체이다.


## 25단계: 헤더 없는 Stage와 Space 전환 (US17)

- [x] T070 [US17] src/ui.rs의 Stage 초기/갱신 매핑에서 스테이징 헤더를 제거한다. FR-064.
- [x] T071 [US17] src/setup.rs·src/ui.rs에 선택 경로 unstage와 양방향 선택 복원을 연결한다. FR-065, FR-066.
- [x] T072 [US17] src/setup.rs의 임시 저장소 검증을 왕복/부분 staged/unborn/literal 경로까지 확장하고 fmt/check/clippy/단위·통합 및 수렴 결과를 specs/001-git-client/quickstart.md에 기록한다. SC-027.

의존성: T070 → T071 → T072. UI 변경은 순차 수행하며 백엔드 검증은 다른 저장소 통합 검사와 독립이다.


## 26단계: 안정 정렬·지연 조회·커밋 메시지 (US18)

- [x] T073 [US18] src/git.rs에서 경로 우선 상태 정렬과 실제 저장소 검증을 구현한다. FR-067, SC-028.
- [x] T074 [US18] src/session.rs·src/model.rs·src/ui.rs의 시작/탭 활성화/닫기 경로에 최초 활성 탭 지연 조회를 구현하고 검증한다. FR-068, SC-028.
- [x] T075 [US18] src/ui.rs의 History 커밋상세 행 글자 크기를 파일 행과 맞춘다. FR-069.
- [x] T076 [US18] src/commit_editor.rs·src/main.rs에 한글/여러 줄 입력·선택·클립보드·IME 범위와 검증을 구현한다. FR-070, SC-029.
- [x] T077 [US18] src/setup.rs·src/ui.rs에 staged 커밋·c 도움말·메시지 모달·제출 키·하단 안내·성공/실패 처리를 연결하고 임시 저장소에서 검증한다. FR-070, FR-071, SC-029, SC-030.
- [x] T078 fmt/check/clippy/단위·통합 및 수렴 검사를 수행하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-028~SC-030.

의존성: T073 → T074 → T075 → T076 → T077 → T078. 입력기와 Git 커밋 검증은 독립 가능하나 UI 편집은 순차 진행한다. 완료 범위는 네 요청 전체이다.


## 27단계: 커밋 팝업 조건과 입력 스크롤 (US19)

- [x] T079 [US19] src/ui.rs·src/setup.rs에 staged 없는 커밋 차단을 추가하고 임시 저장소에서 검증한다. FR-072.
- [x] T080 [US19] src/ui.rs의 두 제목 크기·우측 버튼·Ctrl+S 전용 제출·소문자 안내를 수정한다. FR-073, FR-074.
- [x] T081 [US19] src/commit_editor.rs에 필요한 축의 스크롤바·클릭/드래그와 고정 입력/하단을 연결한다. FR-075.
- [x] T082 fmt/check/clippy/단위·통합 및 수렴 검사를 수행하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-031.

의존성: T079 → T080 → T081 → T082. 동일 UI 상태는 순차 편집하며 백엔드 검사는 스크롤 검토와 독립이다.


## 28단계: 작성자 메일과 Stage 다중 선택 (US20)

- [x] T083 [US20] src/model.rs·src/paging.rs·src/ui.rs에 Author 이름/메일 표시를 연결하고 tests/repository.rs에서 검증한다. FR-076, SC-032.
- [x] T084 [US20] src/model.rs·src/ui.rs에 범위/개별 선택·강조·대표 Diff·키보드 범위 확장과 선택 모델 검증을 구현한다. FR-077, SC-032.
- [x] T085 [US20] src/setup.rs·src/ui.rs에 복수 경로 상태 반전·중복 통합·선택 복원·오류 후 재조회를 연결하고 실제 임시 저장소에서 검증한다. FR-078, FR-079, SC-033.
- [x] T086 fmt/check/clippy/단위·통합 및 수렴 검사를 수행하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-032, SC-033.

의존성: T083 → T084 → T085 → T086. UI/모델 변경은 순차 수행하며 실제 작성자와 인덱스 검증은 독립이다.

## 29단계: 선택한 변경사항 폐기 (US21)

- [x] T087 [US21] src/setup.rs에 경로 검증·HEAD 복원·신규 파일 한정 삭제를 구현하고 임시 저장소에서 검증한다. FR-081, SC-034.
- [x] T088 [US21] src/ui.rs에 d 도움말·다중 선택 대상 확인창·실행/취소·오류 재조회를 연결하고 fmt/check/clippy/단위·통합 및 수렴 점검을 수행한다. FR-080, FR-081, SC-034.

T087은 T085의 공통 경로 검증을 사용하고 T088은 T084/T087 이후 수행한다. 최종 검증은 T086과 함께 전체 요청에 적용한다.

## 30단계: 풀·푸시 단축키 (US22)

- [x] T089 [US22] src/setup.rs·src/ui.rs에 원격 작업 공통 처리·p/Shift+p·도움말·상태 라벨·재조회를 연결하고 임시 로컬 원격에서 검증한다. FR-082, FR-083, SC-035.

T089는 기존 원격 명령과 입력/모달 차단을 재사용하며 최종 T086 검증에 포함한다.

## 31단계: 커밋 후 푸시 키 (US22)

- [x] T090 [US22] src/setup.rs·src/ui.rs에 Ctrl+P 커밋 후 푸시·오류 구분·버튼 제거·하단 안내를 연결하고 임시 로컬 원격에서 검증한다. FR-084, SC-036.

T090은 T089의 일반 push와 기존 commit을 재사용하며 최종 T086 검증에 포함한다.


## 32단계: 폐기 확인 키와 전체 선택 (US23)

- [x] T091 [US23] src/ui.rs의 폐기 파일 목록/버튼을 제거하고 Ctrl+S/Esc 및 소문자 안내를 연결한다. FR-085, FR-086.
- [x] T092 [US23] src/model.rs·src/ui.rs에 Stage Ctrl+A와 대표 Diff 유지·입력 격리 및 선택 모델 검증을 구현한다. FR-087, SC-037.
- [x] T093 fmt/check/clippy/단위·통합 및 수렴 검사를 수행하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-037.

의존성: T091 → T092 → T093. UI 변경은 순차 수행하며 선택 모델 검증은 기존 실제 저장소 검사와 독립이다.

## 33단계: 하단 오류 자동 해제 (US24)

- [x] T094 [US24] src/ui.rs에 하단 오류의 일시 표시·3초 타이머·세대 보호·일반 상태 복원을 연결하고 정적/기존 검증 및 수렴 점검을 수행한다. FR-088, SC-038.

T094는 원본 오류 상태를 유지하고 T093의 최종 검사에 포함한다.

## 34단계: 포커스 표시 (US25)

- [x] T095 [US25] src/ui.rs의 브랜치·이력·파일 선택 행에 실제 포커스 기반 활성/비활성 색과 선택 호버 보존을 적용한다. FR-089, FR-090, SC-039.

독립 검증: 목록 간 이동과 텍스트/모달 진입 시 선택과 색을 확인한다.

## 35단계: 도움말과 새로고침 (US26)

- [x] T096 [US26] src/ui.rs 도움말을 포커스 적용 범위별로 구분하고 소문자 f5/ctrl+r 새로고침 안내와 실행을 연결한다. FR-091, FR-092, SC-040, SC-041.

독립 검증: 도움말을 키 처리 분기와 대조하고 입력/모달 차단을 확인한다.

## 36단계: 검증

- [x] T097 src/ui.rs 변경의 fmt/check/clippy와 기존 테스트를 실행하고 specs/001-git-client/quickstart.md에 결과를 기록한다.

의존성: T095 → T096 → T097. 같은 파일 변경이므로 순차 구현한다. 최소 제공 범위는 US25이며 두 이야기를 모두 완료한다. 별도 병렬 작업은 없다. 기존 프로젝트와 포커스 모델이 준비되어 추가 초기화 작업은 없다.

## 37단계: 새 탭 좌우 배치 (US27)

- [x] T098 [US27] src/ui.rs의 시작 헤더를 제거하고 작업 폼과 최근 저장소를 좌우로 배치하며 고정 높이 한 줄 가상 목록과 탭별 스크롤을 연결한다. FR-093, FR-094, SC-042.

독립 검증: 좌우 배치, 빈 목록 안내, 범위 기반 행 생성, 긴 경로 말줄임과 스크롤을 확인한다.

## 38단계: 삭제 동작 선택 (US28)

- [x] T099 [US28] src/ui.rs에 휴지통 아이콘과 이벤트 전파 차단, 기록 제거/저장소 삭제 선택창 및 기존 최종 삭제 확인을 연결한다. FR-095, FR-096, SC-043.

독립 검증: 아이콘에서 저장소가 열리지 않고 선택창이 뜨며 제거는 기록만 바꾼다. 실제 삭제는 최종 확인에서만 연결된다.

## 39단계: 새 탭 최종 검증

- [x] T100 src/ui.rs의 fmt/check/clippy와 기존 단위·통합 테스트 및 수렴 검토를 수행하고 specs/001-git-client/quickstart.md에 결과와 한계를 기록한다.

의존성: T098 → T099 → T100. 기존 인프라가 있어 별도 초기화는 없으며 같은 파일을 순차 수정한다. 최소 제공 범위는 US27이고 최종적으로 US28까지 완료한다. 병렬 작업은 없다.

## 40단계: 커밋상세 행 포커스 보정 (US29)

- [x] T101 [US29] src/ui.rs의 files_panel에 Window를 전달하고 커밋상세 행에 공통 선택색과 선택 호버 보존을 적용한다. FR-097, FR-098, SC-044.
- [x] T102 src/ui.rs의 fmt/check/clippy·기존 단위 테스트와 수렴 검토를 수행하고 specs/001-git-client/quickstart.md에 결과를 기록한다.

T101 → T102 순서로 수행한다. 기존 인프라를 재사용하므로 추가 초기화나 병렬 작업은 없다. 독립 검증은 커밋상세의 선택을 보존하며 포커스 이동과 호버 시 색을 대조하는 것이다.

## 41단계: 시작 빈 탭 소비 보정 (US30)

- [x] T103 [US30] src/ui.rs에 빈 탭 대체·이미 열린 저장소 재사용·다른 빈 탭 보존·닫힌 시작 탭 회귀 테스트를 추가한다. FR-099, FR-100, SC-045.
- [x] T104 [US30] src/ui.rs의 탭 전환 함수와 공통 완료 메서드를 구현하고 최근 저장소·생성·열기·복제에 연결해 시작 ID·중복 방지·현재 선택·실패/취소 보존을 적용한다. FR-099, FR-100, SC-045.
- [x] T105 src/ui.rs의 fmt/check/clippy·단위 테스트 및 수렴 검토를 수행하고 specs/001-git-client/quickstart.md에 검증 결과를 기록한다.

T103 → T104 → T105로 순차 진행한다. 기존 기반을 사용하며 병렬 작업은 없다. 독립 검증은 시작 빈 탭만 소비되고 다른 탭·현재 선택·취소/실패가 유지되는지 확인하는 것이다.

## 42단계: 도움말 섹션 정리 (US31)

- [x] T106 [US31] src/ui.rs의 도움말을 다섯 섹션과 개별 단축키 행으로 재구성하고 제외 안내 제거 및 긴 목록 스크롤을 적용한다. FR-101, FR-102, SC-046.
- [x] T107 src/ui.rs의 fmt/check/clippy와 표시 항목·키 동작 보존을 점검하고 specs/001-git-client/quickstart.md에 결과를 기록한다.

T106 → T107 순차 수행한다. 기존 기반을 사용하며 별도 초기화·병렬 작업은 없다. 독립 검증은 다섯 섹션·제외 항목·개별 행·스크롤과 닫기 동작의 확인이다.

## 43단계: 설정 기반

- [x] T108 src/settings_git.rs·src/credentials.rs의 격리 설정/원격/URL/GCM 대역 테스트를 먼저 추가한다. SC-047~SC-049.

## 44단계: 사용자 설정 (US32)

- [x] T109 [US32] src/settings_git.rs에 사용자 조회·범위별 저장·전역 사용·복구·검증을 구현하고 src/commit_editor.rs에 단일 행 입력 지원을 추가한다. FR-104, FR-105, SC-047.
- [x] T110 [US32] src/settings.rs와 src/ui.rs·src/main.rs에 사용자/원격 탭 오버레이·하단 진입·f12·모달 키 격리·저장/닫기를 연결한다. FR-103, FR-105, SC-047.

## 45단계: 원격 로그인 (US33)

- [x] T111 [US33] src/credentials.rs에 GCM 탐지·HTTPS 검증·브라우저 get/store·상태·erase·configure·취소/제한·비밀 비노출을 구현하고 src/settings.rs에 로그인 관리 UI를 연결한다. FR-106~FR-108, SC-048.

## 46단계: 원격 정보 (US34)

- [x] T112 [US34] src/settings_git.rs에 복수 fetch/push URL과 브랜치/추적 조회를 구현하고 src/settings.rs 원격 탭에 비밀 제거 표시·빈 상태·오류를 연결한다. FR-109, FR-110, SC-049.

## 47단계: 설정 최종 검증

- [x] T113 fmt/check/clippy와 신규·기존 단위/통합 테스트 및 설정 요구의 수렴 검토를 수행하고 specs/001-git-client/quickstart.md에 결과와 시각/실인증 검증 한계를 기록한다.

T108 → T109 → T110 → T111 → T112 → T113 순차 실행한다. 기반 테스트 이후 사용자 설정이 최소 제공 범위이며 원격 로그인·정보까지 모두 완료한다. 파일 간 논리 의존성을 고려해 별도 병렬 작업은 없다. 독립 검증은 각 이야기의 격리 설정 적용, 대역 인증, 복수 원격 조회다.

## 48단계: 브랜치·태그 회귀 기반

- [x] T114 src/branches.rs와 tests/repository.rs에 선택 커밋 생성·중복/이름·Local/Remote 전환·확인 전 무변경·충돌 보존·페이지별 태그 회귀 테스트를 먼저 추가한다. SC-050~SC-052.

## 49단계: 커밋에서 브랜치 생성 (US35)

- [x] T115 [US35] src/branches.rs에 생성 검증을 구현하고 src/ui.rs·src/main.rs에 b 포커스 조건·이름 입력 오버레이·확인/취소·오류·중복 차단·도움말을 연결한다. FR-111, FR-112, SC-050.

## 50단계: 더블클릭 체크아웃 (US36)

- [x] T116 [US36] src/branches.rs에 Local/Remote 준비·정확한 로컬 재사용·원격 추적 생성·변경 보호를 구현하고 src/ui.rs에 더블클릭·확인창·진행·성공/실패 재조회를 연결한다. FR-113~FR-115, SC-051.

## 51단계: 태그 배지 (US37)

- [x] T117 [US37] src/model.rs·src/paging.rs에 커밋별 태그 목록을 전달하고 src/ui.rs Description에 별도 색 배지를 표시한다. FR-116, FR-117, SC-052.

## 52단계: 브랜치·태그 최종 검증

- [x] T118 fmt/check/clippy와 단위·통합 테스트 및 수렴 점검을 수행하고 specs/001-git-client/quickstart.md에 검증 결과와 한계를 기록한다.

T114 → T115 → T116 → T117 → T118 순차 수행한다. 기존 입력·Git·페이징 기반을 사용하고 별도 병렬 작업은 없다. 생성만·체크아웃·태그 장식을 각각 임시 저장소에서 독립 검증한다.

## 53단계: 전체 변경 스태시 (US38)

- [x] T119 [US38] src/setup.rs에 전체 저장·인덱스 복원 왕복 테스트를 먼저 추가한다. SC-053.
- [x] T120 [US38] src/setup.rs에 전체 저장·빈 상태 오류를 구현한다. FR-120.
- [x] T121 [US38] src/ui.rs에 s 도움말·확인창·모달 격리·비동기 실행·오류·파일 재조회를 연결한다. FR-118~FR-120.
- [x] T122 [US38] fmt/check/clippy·테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-053.

T119 → T120 → T121 → T122 순차 수행한다.

## 54단계: 스태시 목록 갱신과 복원 (US39)

- [x] T123 [US39] src/setup.rs에 복수 stash 선택·인덱스 복원·목록 보존·충돌·대상 검증 테스트를 먼저 추가한다. SC-054.
- [x] T124 [US39] src/setup.rs에 해시 검증과 삭제 없는 인덱스 복원을 구현한다. FR-122, FR-123.
- [x] T125 [US39] src/ui.rs에 저장 후 참조 재조회와 Stash 더블클릭·진행 격리·상태 재조회를 연결한다. FR-121~FR-123.
- [x] T126 [US39] fmt/check/clippy·단위 테스트와 수렴 점검 결과를 quickstart.md에 기록한다. SC-054.

T123 → T124 → T125 → T126 순차 수행한다.

## 55단계: 이름 입력과 확인 삭제 (US40)

- [x] T127 [US40] src/setup.rs의 스태시 테스트에 이름·빈 입력·선택 삭제·대상 불일치·무관 데이터 보존 검증을 먼저 추가한다. SC-055.
- [x] T128 [US40] src/setup.rs에 이름 인수·검증과 selector/해시 검증 후 삭제를 구현한다. FR-124, FR-126.
- [x] T129 [US40] src/ui.rs에 이름 편집·IME 보호·Stash 포커스의 d 확인 삭제·도움말·재조회·오류를 연결한다. FR-124~FR-126.
- [x] T130 [US40] fmt/check/clippy·단위 테스트·수렴 점검을 수행하고 quickstart.md에 결과를 기록한다. SC-055.

T127 → T128 → T129 → T130 순차 수행한다.

## 56단계: 참조 작업·설정 간소화 (US41)

- [x] T131 [US41] src/branches.rs에 태그 생성·삭제·Local 안전 삭제·무관 파일 보존 테스트를 먼저 추가한다. SC-056.
- [x] T132 [US41] src/branches.rs에 이름/대상 검증·태그 생성·참조 삭제를 구현한다. FR-127, FR-128.
- [x] T133 [US41] src/ui.rs에 t 태그 입력·참조 목록 d 확인·재조회·도움말을 연결한다. FR-127, FR-128.
- [x] T134 [US41] src/settings.rs의 고정 라벨·설명/저장 버튼/GCM 표시 제거를 구현한다. FR-129, FR-130.
- [x] T135 [US41] Remote 명확화 답변을 반영하고 해당 범위 구현/테스트를 완료한다. FR-127.
- [x] T136 [US41] fmt/check/clippy·테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-056.

T131 → T132 → T133 → T134 순차 진행한다. Remote 명확화에 의존하는 T135와 추가 그래프 작업 T137 이후 T136으로 마무리한다.

- [x] T137 [US41] src/model.rs·src/git.rs·src/ui.rs·src/history_view.rs에 실제 HEAD 해시 기반 빈 중앙·굵은 테두리 그래프 표시를 구현하고 일반/Detached/unborn 조회를 검증한다. FR-131.

## 57단계: 입력·원격 생성·커밋 체크아웃 (US42)

- [x] T138 [US42] src/branches.rs에 선택 브랜치 원격 생성/upstream/덮어쓰기 거절 및 detached 체크아웃/충돌 보존 테스트를 먼저 추가한다. SC-058.
- [x] T139 [US42] src/branches.rs에 원격 생성·검증과 커밋 체크아웃 함수를 구현한다. FR-133, FR-134.
- [x] T140 [US42] src/ui.rs에 Local u·원격 확인창·Tag/History 더블클릭·진행·오류·재조회를 연결한다. FR-133, FR-134.
- [x] T141 [US42] src/commit_editor.rs·src/settings.rs·src/ui.rs의 단일 행 높이와 입력 래퍼 클리핑을 수정한다. FR-132.
- [x] T142 [US42] src/history_view.rs·src/ui.rs의 HEAD 흰색 채움·2px 선을 구현한다. FR-135.
- [x] T143 [US42] fmt/check/clippy·단위/통합 테스트 및 수렴 점검 후 quickstart.md에 결과를 기록한다. SC-058.

T138 → T139 → T140 → T141 → T142 → T143 순차 수행한다.

## 58단계: 히스토리 브랜치 우선 (US43)

- [x] T144 [US43] src/branches.rs에 후보 없음/단일/복수·심볼릭 제외·선택 체크아웃·대상 변경과 Local 불일치 보호 테스트를 먼저 추가한다. SC-059.
- [x] T145 [US43] src/branches.rs에 후보 조회 구조와 선택 커밋 검증 체크아웃을 구현한다. FR-136, FR-138.
- [x] T146 [US43] src/ui.rs에 히스토리 전용 분기·브랜치 선택 팝업·선택/확인/취소·Remote 생성·재조회를 연결한다. FR-136~FR-138.
- [x] T147 [US43] fmt/check/clippy·단위 테스트·수렴 점검 결과를 quickstart.md에 기록한다. SC-059.

T144 → T145 → T146 → T147 순차 수행한다.

## 59단계: 브랜치 후보 간소화 (US44)

- [x] T148 [US44] src/branches.rs 후보 테스트에 중복 Local/Remote·다른 커밋 Local·중첩 이름 필터를 먼저 추가한다. SC-060.
- [x] T149 [US44] src/branches.rs의 전체 Local 이름 기반 원격 후보 필터를 구현한다. FR-139.
- [x] T150 [US44] src/ui.rs에 단일 후보 즉시 전환·클릭 즉시 적용·확인/취소 제거·Enter 보호를 연결한다. FR-140, FR-141.
- [x] T151 [US44] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-060.

T148 → T149 → T150 → T151 순차 수행한다.

## 60단계: 팝업 공통화 (US45)

- [x] T152 [US45] src/popup.rs에 13px 제목·본문 스크롤·선택적 하단/헤더 동작·높이 제한의 공통 프레임을 추가하고 src/main.rs에 등록한다. FR-142, FR-143.
- [x] T153 [US45] src/ui.rs의 모든 팝업을 제목·본문·하단 분리 구조로 전환하고 기존 클릭/키/IME/진행 동작을 보존한다. FR-142~FR-144.
- [x] T154 [US45] fmt/check/clippy·단위 테스트·모든 팝업 코드 검토·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-061.

T152 → T153 → T154 순차 수행한다.

## 61단계: 현재 브랜치 무시 (US46)

- [x] T155 [US46] src/branches.rs에 현재 브랜치 판별과 기존 테스트의 일반/detached 검증을 추가한다. FR-146, SC-062.
- [x] T156 [US46] src/ui.rs에 세 진입점의 현재 브랜치 무시·재조회 생략과 선택창 설명 제거를 적용한다. FR-145, FR-146.
- [x] T157 [US46] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-062.

T155 → T156 → T157 순차 수행한다.

## 62단계: 전체 라이트 테마 (US47)

- [x] T158 [US47] src/theme.rs의 의미별 라이트 팔레트와 src/main.rs 모듈 등록을 추가한다. FR-147~FR-149.
- [x] T159 [US47] src/ui.rs·src/popup.rs·src/settings.rs·src/commit_editor.rs·src/history_view.rs의 색상을 공통 팔레트로 전환한다. FR-147~FR-149.
- [x] T160 [US47] 대비 계산·어두운 색 잔존 검색·fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-063.

T158 → T159 → T160 순차 수행한다.

## 63단계: 탭·배지·그래프 (US48)

- [x] T161 [US48] src/history_view.rs에 열 폭/레인별 노드 경계 테스트를 먼저 추가한다. SC-064.
- [x] T162 [US48] src/ui.rs·src/theme.rs에 탭 경계·11px 배지·벡터 아이콘·원격 붉은색을 적용한다. FR-150, FR-151.
- [x] T163 [US48] src/history_view.rs·src/ui.rs에 전체 레인 수 기반 좌표와 캐시 키를 구현한다. FR-152.
- [x] T164 [US48] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-064.

T161 → T162 → T163 → T164 순차 수행한다.

## 64단계: 히스토리·상세 보완 (US49)

- [x] T165 [US49] src/ui.rs 상세 테스트에 시간 이름·라벨 범위와 메시지 보존 검증을 추가한다. SC-065.
- [x] T166 [US49] src/theme.rs·src/ui.rs의 태그 순서·파란 선택·목록 크기·커밋 아이콘을 구현한다. FR-153, FR-155.
- [x] T167 [US49] src/history_view.rs·src/ui.rs의 대칭 원·일반 노드 무테두리·헤더 전용 컬럼 선을 구현한다. FR-154.
- [x] T168 [US49] src/ui.rs의 메타데이터 굵은 라벨·동일 측정 run·시간 표기를 구현한다. FR-156.
- [x] T169 [US49] fmt/check/clippy·단위 테스트·대비·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-065.

T165 → T166 → T167 → T168 → T169 순차 수행한다.

## 65단계: 강조·크기 조정 (US50)

- [x] T170 [US50] src/theme.rs·src/ui.rs의 색상 역할·배지 테두리·18px 목록 아이콘을 구현한다. FR-157, FR-159.
- [x] T171 [US50] src/history_view.rs의 HEAD를 15px로 확대하고 경계 테스트를 갱신한다. FR-158, SC-066.
- [x] T172 [US50] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-066.

T170 → T171 → T172 순차 수행한다.

## 66단계: 확대 대상 정정 (US51)

- [x] T173 [US51] src/ui.rs의 현재 브랜치 배지 원을 16px 요소로 분리하고 src/history_view.rs의 HEAD/경계 테스트를 5.5px로 복구한다. FR-160.
- [x] T174 [US51] fmt/check/clippy·단위 테스트·수렴 점검 결과를 quickstart.md에 기록한다. SC-067.

T173 → T174 순차 수행한다.

## 67단계: 원 높이·정렬 (US52)

- [x] T175 [US52] src/ui.rs의 고정 원 helper·18px 배지·목록 중앙 정렬을 구현한다. FR-161, FR-162.
- [x] T176 [US52] fmt/check/clippy·단위 테스트·수렴 점검 결과를 quickstart.md에 기록한다. SC-068.

T175 → T176 순차 수행한다.

## 68단계: 원격 HEAD 표시 숨김 (US53)

- [x] T177 [US53] tests/repository.rs의 원격 HEAD 테스트를 표시 숨김/실제 참조 보존 검증으로 갱신한다. SC-069.
- [x] T178 [US53] src/model.rs·src/git.rs·src/paging.rs에 표시 필터를 적용한다. FR-163.
- [x] T179 [US53] fmt/check/clippy·단위/통합 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-069.

T177 → T178 → T179 순차 수행한다.

## 69단계: 그래프 선명도 (US54)

- [x] T180 [US54] src/history_view.rs에 DPI별 선 두께/좌표 경계 테스트를 먼저 추가한다. SC-070.
- [x] T181 [US54] src/history_view.rs에 픽셀 정렬·수직선 채움·DPI 캐시를 구현한다. FR-164, FR-165.
- [x] T182 [US54] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-070.

T180 → T181 → T182 순차 수행한다.

## 70단계: 상세 굵기·여백 수정 (US55)

- [x] T183 [US55] src/ui.rs에 라벨 BOLD/값 NORMAL 및 길이 검증 테스트를 먼저 추가한다. SC-071.
- [x] T184 [US55] src/ui.rs의 렌더/측정을 shape_text 기반 helper로 바꾸고 선택 좌표를 유지한다. FR-166.
- [x] T185 [US55] src/ui.rs의 빈 뷰어 헤더를 생략하고 안내 여백을 줄인다. FR-167.
- [x] T186 [US55] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-071.

T183 → T184 → T185 → T186 순차 수행한다.

- [x] T187 [US55] src/ui.rs Diff 표식의 줄높이를 22px로 맞추고 코드 검토한다. FR-168. T185 이후 수행한다.

## 71단계: 뷰어 밖 드래그 (US56)

- [x] T188 [US56] src/ui.rs에 세로 경계 clamp 테스트를 먼저 추가한다. SC-072.
- [x] T189 [US56] src/ui.rs에 범위 밖 이동·선택·스크롤·전역 해제와 격리를 구현한다. FR-169, FR-170.
- [x] T190 [US56] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-072.

T188 → T189 → T190 순차 수행한다.

- [x] T191 [US56] src/ui.rs의 HEAD 행을 굵게 하되 배지는 NORMAL로 고정하고 코드 검토한다. FR-171.

## 72단계: 단축키 배지 (US57)

- [x] T192 [US57] src/shortcut.rs의 키 구문 판별과 공통 배지 렌더·검증을 추가하고 src/main.rs에 등록한다. FR-172, FR-173, SC-073.
- [x] T193 [US57] src/ui.rs의 도움말·상태줄·버튼·안내를 공통 키캡으로 전환한다. FR-172, FR-173.
- [x] T194 [US57] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-073.

T192 → T193 → T194 순차 수행한다.

## 73단계: 생성 팝업·파일 글자 (US58)

- [x] T195 [US58] src/branches.rs에 선택적 원격 생성 조합과 임시 원격 검증을 구현한다. FR-175, SC-074.
- [x] T196 [US58] src/ui.rs·src/commit_editor.rs에 파일명/팝업 글자·원격 체크박스·우측 확인·취소 제거를 연결한다. FR-174~FR-176.
- [x] T197 [US58] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-074.

T195 → T196 → T197 순차 수행한다.

## 74단계: 체크박스·입력 (US59)

- [x] T198 [US59] src/controls.rs·src/main.rs·src/settings.rs·src/ui.rs에 고정 체크박스를 적용한다. FR-177.
- [x] T199 [US59] src/branches.rs·src/ui.rs에 선택적 새 브랜치 체크아웃과 테스트를 추가한다. FR-178.
- [x] T200 [US59] src/commit_editor.rs에 단일 행 중앙 정렬·입력 밖 전역 드래그/해제 및 오프셋 테스트를 추가한다. FR-179.
- [x] T201 [US59] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-075.

T198 → T199 → T200 → T201 순차 수행한다.

- [x] T202 [US59] src/ui.rs의 Clone URL을 공통 CommitEditor로 전환하고 Enter/IME/작업 중 차단을 보존한다. FR-179.

## 75단계: 체크 여백 (US60)

- [x] T203 [US60] src/ui.rs·src/settings.rs의 빈 라벨 생략·체크 항목 패딩을 수정한다. FR-180.
- [x] T204 [US60] fmt/check/clippy·기존 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-076.

T203 → T204 순차 수행한다.

## 76단계: 공통 팝업 입력 이동 (US61)

- [x] T205 [US61] src/popup.rs의 포커스 순환 helper·인덱스 테스트를 추가한다. FR-182, SC-077.
- [x] T206 [US61] src/ui.rs·src/settings.rs의 Tab/역순/체크 Space·포커스 표시를 공통화한다. FR-182.
- [x] T207 [US61] src/ui.rs 선택 커밋 행 제거와 src/controls.rs 공통 패딩 6px를 적용한다. FR-181.
- [x] T208 [US61] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-077.

T205 → T206 → T207 → T208 순차 수행한다.

## 77단계: Space 중복 수정 (US62)

- [x] T209 [US62] src/ui.rs·src/settings.rs의 수동 Space 전환을 제거한다. FR-183.
- [x] T210 [US62] fmt/check/clippy·기존 단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-078.

T209 → T210 순차 수행한다.

## 78단계: 숫자 섹션 포커스 (US63)

- [x] T211 [US63] src/ui.rs에 포커스 매핑·단축키·도움말·테스트를 추가한다. FR-184, FR-185.
- [x] T212 [US63] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-079.

T211 → T212 순차 수행한다.

## 79단계: 옵션·체크아웃·선택 유지 (US64)

- [x] T213 [US64] src/session.rs·src/settings.rs·src/ui.rs에 옵션 복원/저장·호환 검증을 구현한다. FR-186.
- [x] T214 [US64] src/ui.rs에 o와 태그 창 정리를 적용한다. FR-187, FR-188.
- [x] T215 [US64] src/git.rs·src/ui.rs에 선택 커밋/페이지 유지와 통합 검증을 구현한다. FR-189.
- [x] T216 [US64] fmt/check/clippy·단위/통합 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-080.

T213 → T214 → T215 → T216 순차 수행한다.

## 80단계: 탐색 동작 (US65)

- [x] T217 [US65] src/ui.rs에 ijkl·숫자 후보 매핑과 테스트를 추가한다. FR-190, FR-192.
- [x] T218 [US65] src/ui.rs에 목록 선택/Space 탐색 분리·생성 후 포커스 복원을 적용한다. FR-191, FR-193.
- [x] T219 [US65] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-081.

T217 → T218 → T219 순차 수행한다.

## 81단계: 체크아웃 포커스·이름 (US66)

- [x] T220 [US66] src/ui.rs의 강제 섹션/탭 이동을 제거하고 체크아웃 후 포커스를 복원한다. FR-194.
- [x] T221 [US66] src/branches.rs의 후보 이름·테스트를 변경한다. FR-195.
- [x] T222 [US66] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-082.

T220 → T221 → T222 순차 수행한다.

## 82단계: Reset·선택 유지 (US67)

- [x] T223 [US67] src/branches.rs에 ResetMode·reset 및 모드별 상태/무효 대상 테스트를 추가한다. FR-198, SC-083.
- [x] T224 [US67] src/ui.rs에 x·공통 팝업·selectbox·설명·실행/오류/재조회를 연결한다. FR-197, FR-198.
- [x] T225 [US67] src/ui.rs의 체크아웃 선택 초기화를 제거하고 src/popup.rs 공통 하단/확인 정렬을 적용한다. FR-196, FR-199.
- [x] T226 [US67] fmt/check/clippy·단위 테스트·수렴 점검 후 quickstart.md에 결과를 기록한다. SC-083.

T223 → T224 → T225 → T226 순차 수행한다.


## 83단계: 히스토리 작업 확장 (US68)

- [x] T227 [US68] src/branches.rs에 revert/merge/rebase와 검증·충돌 보호 및 임시 저장소 테스트를 추가한다. FR-201~FR-203, SC-084.
- [x] T228 [US68] src/ui.rs Reset 라디오·숫자키 및 Revert 체크/기억을 src/session.rs와 연결한다. FR-200~FR-201.
- [x] T229 [US68] src/ui.rs Merge/Rebase 독립 히스토리·선택·확인·재조회·포커스를 연결한다. FR-202~FR-203.
- [x] T230 [US68] fmt/check/Clippy/테스트와 수렴 점검 후 결과를 기록한다. SC-084.

T227 → T228 → T229 → T230 순서로 수행한다. 명세 분석: FR-200~FR-203 및 SC-084는 위 작업에 모두 연결되며 충돌·중복·기존 선택 보존 요구와 계획 사이에 모순이 없다.


## 84단계: Reset 대상 표시 정리 (US69)

- [x] T231 [US69] src/ui.rs의 Reset 대상 커밋 표시 행만 제거한다. FR-204.
- [x] T232 [US69] 포맷/컴파일 및 실행 대상 보존을 점검하고 수렴 결과를 기록한다. SC-085.

T231 → T232 순서로 수행한다. 명세 분석: FR-204와 SC-085는 두 작업과 계획에 모두 연결되며 모순과 미해결 질문은 없다.


## 85단계: Merge·Rebase 확인 간소화 (US70)

- [x] T233 [US70] src/ui.rs Integrate를 선택 커밋 고정 상태와 확인 문구로 변경하고 전용 히스토리 코드/입력을 제거한다. FR-205~FR-206.
- [x] T234 [US70] 포맷/컴파일/Clippy·기존 작업 테스트와 수렴 점검 결과를 기록한다. SC-086.

T233 → T234 순서로 수행한다. 명세 분석: FR-205~FR-206 및 SC-086은 두 작업에 모두 연결되고 FR-202의 기존 대상 선택 요구는 명시적으로 대체되었다. 계획과 모순·미해결 질문은 없다.


## 86단계: 단축키 통합 설정 (US71)

- [x] T235 [US71] src/shortcut.rs에 기능 목록·두 슬롯·영역 충돌·명령 변환과 회귀 테스트를 구현한다. FR-208~FR-210.
- [x] T236 [US71] src/session.rs 및 src/ui.rs에 설정 저장·복원과 실제 키 처리 연동을 구현한다. FR-209~FR-210.
- [x] T237 [US71] src/ui.rs에 F11/하단 버튼·다섯 영역 표·등록·지우기·기본값·동적 도움말을 구현한다. FR-207~FR-209.
- [x] T238 [US71] 포맷/컴파일/Clippy/테스트와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-087.

T235 → T236 → T237 → T238 순서로 수행한다. 동일 파일 의존성이 있어 순차 실행한다. 독립 검증은 F11 열기·두 키 편집·실제 실행·세션 복원이다.

구현 전 분석: FR-207~FR-210 및 SC-087을 네 작업에서 모두 다룬다. 기존 문서와 충돌·누락·미해결 질문은 없다.


## 87단계: 방향 선택 확장 목록 정리 (US72)

- [x] T239 [US72] src/ui.rs 공용 단축키 표에서 방향 선택 확장 네 항목의 표시를 제외한다. FR-211.
- [x] T240 [US72] 포맷·컴파일·단축키 테스트 및 수렴 점검을 수행하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-088.

T239 → T240 순차 진행한다. 설정과 도움말의 공통 필터 및 기존 키 처리 보존을 독립 검증한다.


## 88단계: 확인창 공통 키 통합 (US73)

- [x] T241 [US73] src/shortcut.rs에 공통 확인·취소·다음입력·이전입력 정의와 창별 해석·충돌 및 회귀 테스트를 구현한다. FR-212~FR-213.
- [x] T242 [US73] src/ui.rs의 공통 키 표시·설정창 취소·선택 후보 확인을 연동한다. FR-212~FR-213.
- [x] T243 [US73] 포맷·컴파일·Clippy·관련 테스트와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-089.

T241 → T242 → T243 순서로 실행한다. 명세·계획·작업 분석 결과 네 공통 기능·기존 창 실행·키 재정의·조합 보호·검증 요구를 모두 포함하며 모순과 미해결 질문이 없다.


## 89단계: 확인 표시·커밋 팝업 구조 (US74)

- [x] T244 [US74] src/ui.rs 확인 버튼에서 키 표기를 제거하고 커밋 팝업 본문·체크·우측 확인을 구성한다. FR-214~FR-215.
- [x] T245 [US74] src/ui.rs 및 src/shortcut.rs에 체크 상태 기반 단일 제출·포커스·키 전환·조합 및 실행 보호를 연결한다. FR-215~FR-216.
- [x] T246 [US74] 포맷·컴파일·Clippy·관련 테스트 및 수렴 점검을 수행하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-090.

T244 → T245 → T246 순차 수행한다. 분석 결과 FR-214~FR-216 및 SC-090의 배치·실행·보호·검증은 모두 위 작업과 계획에 연결되며 미해결 질문과 모순은 없다.


## 90단계: 단축키 설정 탭·명칭 (US75)

- [x] T247 [US75] src/ui.rs에서 안내를 제거하고 선택 탭·그룹 필터·열기 초기화·전환 시 등록 취소를 구현한다. FR-217.
- [x] T248 [US75] src/shortcut.rs의 세 커밋히스토리 기능명을 변경하고 포맷·컴파일·관련 테스트·수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. FR-218, SC-091.

T247 → T248 순서로 실행한다. 명세·계획·작업 분석 결과 안내 제거·다섯 탭·영문명·설정 보존 및 검증을 모두 포함하며 모순·미해결 질문이 없다.


## 91단계: 히스토리 대상 행 제거 (US76)

- [x] T249 [US76] src/ui.rs Revert·Integrate 렌더 분기에서 대상 표시 행만 제거하고 실행 대상은 유지한다. FR-219.
- [x] T250 [US76] 포맷·컴파일·Clippy·실행 대상 경로 및 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-092.

T249 → T250 순서로 수행한다. 분석 결과 표시 삭제와 실행 보존이 명세·계획·두 작업에 연결되며 기존 대상 표시 요구는 명시적으로 대체되어 모순과 미해결 질문은 없다.


## 92단계: 단축키 일괄 승인·헤더 (US77)

- [x] T251 [US77] src/ui.rs에 임시 전 탭 설정과 등록·해제·복원·취소 상태를 분리한다. FR-220~FR-222.
- [x] T252 [US77] src/ui.rs 및 src/session.rs에 저장 성공 후 전체 적용하는 확인과 우측 복원·확인 버튼 및 회색 표 헤더를 구현한다. FR-220~FR-223.
- [x] T253 [US77] 포맷·컴파일·Clippy·단축키/세션 테스트 및 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-093.

T251 → T252 → T253 순차 수행한다. 분석 결과 승인·전체 탭 공유·취소·복원·저장 실패·기존 키 사용·회색 헤더 요구가 계획과 작업에 모두 연결되고 과거 즉시 적용 요구는 명시적으로 대체되어 모순과 미해결 질문은 없다.


## 93단계: 단축키 설정 헤더 제거 (US78)

- [x] T254 [US78] src/ui.rs에서 설정 표 헤더를 제외하고 기존 셀 여백·너비·편집 동작을 유지한다. FR-224.
- [x] T255 [US78] src/popup.rs 및 src/ui.rs에서 설정 전용 고정 높이·본문 확장·화면 제한을 적용한다. FR-225.
- [x] T256 [US78] 포맷·컴파일·Clippy·헤더/높이 경로·수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-094~SC-095.

T254 → T255 순차 수행한다. 사용자 정정에 따라 미착수 패딩 작업을 헤더 제거로 갱신했다. 분석 결과 FR-224·SC-094의 헤더 제거·패딩 미추가·기존 동작 보존이 명세·계획·작업에 연결되고 기존 헤더 요구는 대체되어 모순과 미해결 질문은 없다.

추가 높이 요청 분석: FR-225·SC-095를 T255·T256에 연결하고 T254 → T255 → T256 순서로 수행한다. 고정 높이·화면 상한·기존 본문 스크롤과 다른 팝업 보존 계획에 모순·누락은 없다.


## 94단계: 공통 방향·전환 통합 (US79)

- [x] T257 [US79] src/shortcut.rs에 공통 방향·입력전환 정의와 적용 범위·충돌 및 회귀 테스트를 구현한다. FR-226~FR-228.
- [x] T258 [US79] src/ui.rs에 공통 전환 키의 포커스별 체크 상태 전환과 기존 입력 보호를 연결한다. FR-227~FR-228.
- [x] T259 [US79] 포맷·컴파일·Clippy·관련 테스트 및 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-096.

T257 → T258 → T259 순서다. 분석 결과 공통 범위 한정·전용 Space 보존·공유 키·포커스·승인 보존·검증 요구가 명세·계획·작업에 모두 연결되며 모순·미해결 질문은 없다.


## 95단계: Stage 파일목록 커밋·스태시 (US80)

- [x] T260 [US80] src/shortcut.rs에 두 기능의 파일목록 그룹·Stage 범위 및 상속·충돌·회귀 테스트를 적용한다. FR-229~FR-230.
- [x] T261 [US80] src/ui.rs에서 활성 파일목록 포커스·Stage 탭 조건으로 키 해석과 커밋/스태시 실행을 제한한다. FR-229.
- [x] T262 [US80] 포맷·컴파일·Clippy·관련 테스트와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-097.

T260 → T261 → T262 순차 수행한다. 분석 결과 범위·그룹·기존 키/기능 보존·상속·충돌·검증이 명세·계획·작업에 모두 연결되며 모순·미해결 질문은 없다.


## 96단계: 탭 하위 분류·선택 숨김 (US81)

- [x] T263 [US81] src/shortcut.rs에 하위 분류·포함·가시성 판정과 공유/숨김/실행 보존 테스트를 구현한다. FR-231~FR-233.
- [x] T264 [US81] src/ui.rs에 분류별 소제목·공유 행·고유 UI ID·빈 분류 안내를 적용하고 기존 승인 흐름을 유지한다. FR-231~FR-233.
- [x] T265 [US81] src/shortcut.rs·src/controls.rs·src/ui.rs에서 체크아웃 후보·Reset 키 표시를 공통 배지로 변경한다. FR-234.
- [x] T266 [US81] src/ui.rs 단축키 슬롯을 공통 배지로 표시하고 등록·승인 동작을 유지한다. FR-235.
- [x] T267 [US81] 포맷·컴파일·Clippy·관련 테스트와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-098~SC-100.

T263 → T264 → T265 순서로 수행한다. 분석 결과 사용자가 확인한 설정 공유·실제 탭 분류·가시성/실행 분리·기존 승인 보존 요구가 명세·계획·작업에 모두 연결되며 모순·미해결 질문은 없다.

배지 추가 분석: FR-234·SC-099를 공통 배지·첫 열 개 조건·기존 선택 보존 계획과 T265~T266에 연결했다. T263 → T264 → T265 → T266 순서로 진행하며 모순·미해결 질문은 없다.

슬롯 배지 분석: FR-235·SC-100의 공통 표현·등록 및 승인 보존을 T266·T267에 연결했다. T263 → T264 → T265 → T266 → T267 순서로 수행하며 모순·누락은 없다.


## 97단계: 브랜치 삭제 팝업 (US82)

- [x] T268 [US82] src/ui.rs의 삭제 제목·이름 행·본문·원격 체크·취소 버튼과 우측 확인을 정리한다. FR-236~FR-237.
- [x] T269 [US82] src/ui.rs에서 Remote 제한 체크와 클릭·입력전환·포커스 및 기존 삭제 전달을 연결한다. FR-237~FR-238.
- [x] T270 [US82] 포맷·컴파일·Clippy·기존 삭제 테스트·수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-101.

T268 → T269 → T270 순차 수행한다. 분석 결과 배치·Remote 기존 범위·확인 실행·키보드·취소·보호·검증을 명세·계획·작업에서 모두 다루며 모순·미해결 질문은 없다.


## 98단계: 참조 종류별 연결 삭제 (US83)

- [x] T271 [US83] src/branches.rs에 upstream/refspec 기반 삭제 계획·관계 재검증·선검사·lease 서버 삭제와 로컬 안전 삭제 및 임시 저장소 회귀 테스트를 구현한다. FR-240~FR-242.
- [x] T272 [US83] src/ui.rs에 비동기 계획 조회·종류별 제목/관계/체크·활성 조건·공통 키/포커스·단일 확인 전달을 연결한다. FR-239~FR-243.
- [x] T273 [US83] src/ui.rs의 Stash/Tag 전용 제목·체크 없는 확인 전용 footer와 기존 취소·오류 경로를 정리한다. FR-239, FR-241.
- [x] T274 [US83] 포맷·컴파일·Clippy·전체 관련 테스트와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-102.

T271 → T272 → T273 → T274 순서다. 분석 결과 네 팝업·실제 관계·스냅샷/원격 목적지·선검사/부분 실패·키보드·기존 정책 대체·회귀 요구가 계획과 작업에 모두 연결되며 모순·미해결 질문은 없다.


## 99단계: 브랜치 푸시·우선순위 (US84)

- [x] T275 [US84] src/shortcut.rs의 이름·키·Local 범위·브랜치 우선 및 중복 정책과 회귀 테스트를 적용한다. FR-244~FR-245.
- [x] T276 [US84] src/ui.rs에서 Local 범위를 연결하고 Upload 제목·브랜치명/설명/취소 버튼을 정리한다. FR-246.
- [x] T277 [US84] 포맷·컴파일·Clippy·단축키/게시 테스트와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-103.

T275 → T276 → T277 순서다. 분석 결과 이름·기본 키·Local 범위·명시된 브랜치 우선·중복 정책·팝업·기존 게시 보존 요구가 명세·계획·작업에 연결되며 모순·미해결 질문은 없다.


## 100단계: 브랜치 푸시 키 복원 (US85)

- [x] T278 [US85] src/shortcut.rs의 기본 키를 u로 복원하고 기본/공통/우선순위 회귀 테스트를 갱신한다. FR-247.
- [x] T279 [US85] 포맷·컴파일·관련 테스트 및 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-104.

T278 → T279 순서다. 분석 결과 키 복원·기존 동작 유지·검증이 명세·계획·작업에 연결되며 이전 기본 키 요구는 명시적으로 대체되어 모순·미해결 질문은 없다.

## 101단계: 팝업 입력 드래그 보호 (US86)

준비·기반: 기존 공통 팝업과 설치된 GPUI 이벤트 구현을 사용하며 추가 설정은 없다.

- [x] T280 [US86] src/popup.rs 공통 카드에서 왼쪽 누름 전파를 차단하여 내부 시작 드래그가 배경 클릭으로 이어지지 않게 한다. FR-248~FR-249.
- [x] T281 [US86] src/popup.rs·src/commit_editor.rs·src/ui.rs의 네 경계와 자식 이벤트·취소·진행 보호를 검토하고 포맷·컴파일·Clippy·기존 입력/팝업 테스트 결과를 specs/001-git-client/quickstart.md에 기록한다. SC-105.

T280 → T281 순서로 단일 사용자 이야기를 완성한다. 같은 이벤트 경로를 검토하므로 병렬 구현 대상은 없다. 독립 수용 기준은 네 시작/종료 경계, 입력 선택 및 기존 버튼·키·진행 보호의 유지다. 최소 구현 범위는 US86 전체다.

## 102단계: 커밋 Shift+Enter 줄바꿈 (US87)

준비·기반: 기존 캡처 키 처리와 다중 줄 편집기를 사용하며 추가 설정은 없다.

- [x] T282 [US87] src/ui.rs에서 커밋 메시지 포커스의 Shift+Enter를 단축키 해석 전에 편집기로 전달하고 진행 중에는 차단한다. FR-250~FR-251.
- [x] T283 [US87] src/ui.rs·src/commit_editor.rs의 포커스·수정키·선택·조합·진행 경로를 검토하고 포맷·컴파일·Clippy·바이너리 테스트 결과를 specs/001-git-client/quickstart.md에 기록한다. SC-106.

T282 → T283 순서이며 병렬 구현은 없다. 최소 구현은 US87 전체로, 독립 수용 기준은 메시지 입력 Shift+Enter의 줄바꿈과 기존 확인·입력 보호 유지다.

## 103단계: 파일목록 트리 토글 (US88)

준비·기반: 기존 가상 파일목록·단축키 체계를 사용한다.

- [x] T284 [US88] src/model.rs에 원시 경로 트리 행 생성·가시 범위 선택 및 계층/접힘/중복/인덱스 회귀 테스트를 추가한다. FR-253~FR-254.
- [x] T285 [US88] src/shortcut.rs에 백틱 기본 명령·Stage/History 분류와 범위/재지정 테스트를 추가한다. FR-252.
- [x] T286 [US88] src/ui.rs에 모드·접힘·폴더/파일 렌더·갱신·가시 탐색/선택/스크롤을 연결한다. FR-252~FR-254.
- [x] T287 [US88] 포맷·컴파일·Clippy·관련 테스트 및 수렴 점검 후 specs/001-git-client/quickstart.md에 검증 결과를 기록한다. SC-107.

T284 → T285 → T286 → T287 순서로 수행한다. T284와 T285는 서로 다른 파일에서 병렬 수행 가능하나 이번에는 순차 진행한다. 최소 구현은 US88 전체이며 독립 수용 기준은 백틱 왕복·폴더 접힘과 파일 선택/작업의 원본 대상 보존이다.

## 104단계: 펼친 트리·폴더 일괄 Stage (US89)

준비·기반: US88의 접힘 요구를 대체하며 기존 일괄 Stage 실행 경로를 재사용한다.

- [x] T288 [US89] src/model.rs의 접힘 상태를 제거하고 항상 펼친 트리·하위 경계 기반 목표 맵과 계층/혼합/중복/빈 대상 테스트를 구현한다. FR-255~FR-256.
- [x] T289 [US89] src/ui.rs에 폴더 선택·강조·일반/Shift 탐색·대상 해제와 기존 보호 경로의 폴더 Space·재조회 후 선택 유지를 연결한다. FR-255~FR-257.
- [x] T290 [US89] src/setup.rs의 임시 저장소 테스트로 폴더 전체 Stage/Unstage·바깥 경로 보존을 검증하고 포맷·컴파일·Clippy·관련 테스트 결과를 specs/001-git-client/quickstart.md에 기록한다. SC-108.

T288 → T289 → T290 순차 진행하며 병렬 구현 대상은 없다. 최소 구현은 US89 전체다. 독립 수용 기준은 항상 펼친 폴더와 디렉터리 Space 왕복 및 다른 디렉터리 무변경이다.

## 105단계: Local 원격 브랜치 연결 (US90)

준비·기반: 기존 원격 매핑·비동기 대화상자·단축키 체계를 재사용한다.

- [x] T291 [US90] src/branches.rs에 upstream 후보/계획·재검증·연결과 임시 저장소 회귀 테스트를 구현한다. FR-259~FR-261.
- [x] T292 [US90] src/shortcut.rs에서 Local 푸시를 제거하고 새 연결 ID·r·대화상자 탐색과 범위/재지정/과거 키 무효화 테스트를 구현한다. FR-258.
- [x] T293 [US90] src/ui.rs의 Upload를 비동기 연결 목록·현재 선택·클릭/키보드·스크롤·확인/취소·빈 목록·오류·포커스 복원으로 교체한다. FR-258~FR-261.
- [x] T294 [US90] 포맷·컴파일·Clippy·관련 테스트와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-109.

T291 → T292 → T293 → T294 순서다. T291/T292는 서로 다른 파일로 병렬 가능하지만 이번에는 순차 수행한다. 최소 구현은 US90 전체이며 독립 수용 기준은 Local r에서 목록 선택/확인으로 upstream만 변경하고 기존 푸시 기능을 제거하는 것이다.

## 106단계: 원격 연결 라디오 목록 (US91)

준비·기반: 기존 공통 라디오·Connect 선택·스크롤을 재사용한다.

- [x] T295 [US91] src/ui.rs·src/shortcut.rs에 Local 표시 행 제거·라디오/포커스·Space/방향키·8행 고정 높이와 스크롤 제한을 적용한다. FR-262~FR-263.
- [x] T296 [US91] 포맷·컴파일·Clippy·기존 연결/단축키 테스트와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-110.

T295 → T296 순서이며 병렬 대상은 없다. 최소 구현은 US91 전체다. 독립 수용 기준은 라디오 키보드 선택과 8행 초과 스크롤, 기존 연결 대상·확인 동작 보존이다.

## 107단계: Merge 충돌 해결·완료·중단 (US92)

준비·기반: 기존 gix 조회·Stage·BranchDialog·Git 실행과 검증 체계를 사용한다.

- [x] T297 [US92] src/model.rs·src/git.rs에 Merge 상태 조회·동일 작업 판정·단일 충돌 행/작업 결과 미리보기를 구현한다. FR-264~FR-265.
- [x] T298 [US92] src/branches.rs에 완료/중단 재검증과 Git 실행 및 실제 충돌·삭제·Stage·완료·Abort·worktree·실패/오래된 대상 회귀 테스트를 구현한다. FR-266~FR-268.
- [x] T299 [US92] src/ui.rs에 Merge 진행 표시·충돌 강조/안내·Stage 갱신·완료/중단 확인·비동기 오류/재조회와 경합 단축키 제한을 연결한다. FR-264~FR-268.
- [x] T300 [US92] 포맷·컴파일·Clippy·관련 테스트 및 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-111.

T297 → T298 → T299 → T300 순차 진행하며 이번 작업의 병렬 대상은 없다. 최소 구현은 US92 전체이고 독립 수용 기준은 충돌 후 기존 Stage를 거쳐 병합 완료 또는 확인 후 중단까지 앱에서 처리하는 것이다.

## 108단계: Rebase 충돌 해결·계속·중단·건너뛰기 (US93)

준비·기반: 기존 Merge 복구·충돌 Stage·대화상자 실행을 재사용한다.

- [x] T301 [US93] src/model.rs·src/git.rs에 Rebase 상태/세션/단계 판정·merge/apply/worktree 조회와 git am 구분을 구현한다. FR-269, FR-272.
- [x] T302 [US93] src/branches.rs에 Continue/Abort/Skip 검증·새 단계 충돌 대기 판정과 실제 연속 충돌/완료/중단/건너뛰기/실패/오래된 대상/apply/worktree 테스트를 구현한다. FR-270~FR-272.
- [x] T303 [US93] src/ui.rs에 공통 배너/가드·Rebase 진행/커밋·확인창·다음 충돌 시 Stage 이동·실패/재조회를 연결한다. FR-269~FR-273.
- [x] T304 [US93] 포맷·컴파일·Clippy·전체 회귀 테스트와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-112.

T301 → T302 → T303 → T304 순차 진행하며 병렬 대상은 없다. 최소 구현은 US93 전체다. 독립 수용 기준은 연속 충돌을 계속 해결해 완료하거나 확인 후 중단/건너뛰기하는 흐름과 기존 Merge 보존이다.

## 109단계: Revert 충돌 해결·계속·중단·건너뛰기 (US94)

준비·기반: 기존 통합 작업 배너/Stage/확인창·Git 실행을 재사용한다.

- [x] T305 [US94] src/model.rs·src/git.rs에 Revert 상태/옵션/세션/대상 조회·다른 작업 구분과 공통 진행 여부를 구현한다. FR-274, FR-277.
- [x] T306 [US94] src/branches.rs에 옵션 저장 실행·Continue/Abort/Skip·단일 no commit Stage 보존과 실제 일반/옵션/중단/건너뛰기/실패/연속/외부/worktree 회귀 테스트를 구현한다. FR-275~FR-277.
- [x] T307 [US94] src/ui.rs에 Revert 배너·확인창/옵션 미상 체크·포커스·Stage 갱신/전환·기존 공통 가드/재조회를 연결한다. FR-274~FR-278.
- [x] T308 [US94] 포맷·컴파일·Clippy·전체 관련 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-113.

T305 → T306 → T307 → T308 순차 진행하며 이번 작업의 병렬 대상은 없다. 최소 구현은 US94 전체다. 독립 수용 기준은 단일 Revert의 두 커밋 모드를 유지하는 충돌 해결·계속·중단·건너뛰기와 기존 Merge/Rebase 보존이다.

## 110단계: Stash 충돌 해결·완료·취소 (US95)

준비·기반: 기존 apply --index·충돌 Stage·통합 배너/확인창을 재사용한다.

- [x] T309 [US95] src/stash_recovery.rs·src/lib.rs에 백업/원자 메타데이터·안전한 apply·조회/완료/취소·경로/인덱스/무결성/잠금 보호와 실제 저장소 테스트를 구현한다. FR-279~FR-283.
- [x] T310 [US95] src/model.rs·src/git.rs·src/setup.rs에 앱 복원 상태와 일반 미병합 조회·기존 복원 서비스 연결 및 회귀 검증을 추가한다. FR-279, FR-282~FR-284.
- [x] T311 [US95] src/ui.rs에 앱/외부 충돌 배너·Stage 갱신/전환·완료/취소 확인·오류 보존과 공통 경합 가드를 연결한다. FR-279~FR-284.
- [x] T312 [US95] 포맷·컴파일·Clippy·전체 관련 테스트와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-114.

T309 → T310 → T311 → T312 순차 진행하며 이번 작업의 병렬 대상은 없다. 최소 구현은 US95 전체다. 독립 수용 기준은 Stash 충돌 해결 완료와 보관한 파일/인덱스를 통한 안전한 복원 취소다.

## 111단계: DiffViewer 충돌 구간 선택 (US96)

준비·기반: 기존 Diff·가상 목록·Stage 비동기 흐름과 1 MiB 제한을 재사용한다.

- [x] T313 [US96] src/conflict.rs·src/lib.rs에 충돌 파서·구간 선택·작업 파일/미병합/원문 재검증을 구현하고 두 구간·세 선택·CRLF·빈 쪽·마지막 개행·diff3·잘못된 마커·실제 저장소 인덱스 보존을 검증한다. FR-286, FR-288.
- [x] T314 [US96] src/model.rs·src/git.rs에 충돌 미리보기 원문/구간을 연결하고 Stage 미병합에 한정한다. FR-285.
- [x] T315 [US96] src/ui.rs의 충돌 시작 줄에 세 선택 버튼·비동기 적용·중복 방지·재조회·실패 표시를 연결한다. FR-285~FR-288.
- [x] T316 [US96] 포맷·컴파일·Clippy·관련 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-115.

T313 → T314 → T315 → T316 순차 진행하며 병렬 대상은 없다. 최소 구현은 US96 전체다. 독립 수용 기준은 한 파일의 두 구간을 서로 다른 선택으로 해결하고 기존 Space Stage로 이어지며 오래된 원문은 덮어쓰지 않는 것이다.

## 112단계: 마커 보존과 구간·줄 부분 Stage (US97)

준비·기반: 기존 Diff 가상 목록·원문 조회·Git 실행·비동기 Stage 갱신을 재사용한다.

- [x] T317 [US97] src/model.rs·src/git.rs·src/ui.rs에 독립 충돌 동작 행을 추가하고 마커 원문/좌표 보존과 기존 구간 선택 회귀를 검증한다. FR-289.
- [x] T318 [US97] src/partial_stage.rs·src/lib.rs·src/git.rs에 원문 기반 줄 선택/구간 계산·패치 생성·인덱스/작업 파일/경로/필터 재검증·부분 Stage를 구현하고 실제 저장소 보존/경계 테스트를 추가한다. FR-290~FR-292, SC-116.
- [x] T319 [US97] src/ui.rs에 구간 Stage·줄 체크·선택 수/Stage/해제·세대별 초기화·비동기/경합 방지·오류/목록 재조회를 연결한다. FR-290, FR-293.
- [x] T320 [US97] 포맷·컴파일·Clippy·관련/기존 테스트 및 수렴 점검 후 specs/001-git-client/quickstart.md에 결과와 수동 검증 범위를 기록한다. SC-116.

T317 → T318 → T319 → T320 순차 진행하며 병렬 대상은 없다. 최소 구현은 US97 전체다. 독립 수용 기준은 마커가 보이는 상태로 기존 충돌을 해결하고 일반 파일의 한 구간/일부 줄만 Stage하되 원본 작업 파일과 기존 Stage 결과를 보존하는 것이다.

## 113단계: DiffViewer 키보드·고정 충돌 도구 모음 (US98)

준비·기반: 기존 텍스트 커서·viewer 단축키·줄 토글·Stage·충돌 비동기 처리를 재사용한다.

- [x] T321 [US98] src/conflict.rs·src/model.rs·src/git.rs에 원문 시작/종료 줄 기반 커서 구간 판정과 합성 행 제거를 구현하고 두 구간·마커/내용/바깥/좌표 없는 줄·diff3/CRLF 경계를 검증한다. FR-296~FR-297.
- [x] T322 [US98] src/shortcut.rs·src/ui.rs에 viewer Space 줄 토글/Enter 선택 Stage·설정/도움말/재지정·초기 커서 복원을 연결하고 단축키 범위/이전 키 무효화를 검증한다. FR-294, FR-298.
- [x] T323 [US98] src/ui.rs에서 선택 해제 버튼과 본문 충돌 버튼을 제거하고 상단 고정 충돌 대상/버튼·클릭 직전 재판정·비활성화를 연결한다. FR-295~FR-298.
- [x] T324 [US98] 포맷·컴파일·Clippy·관련 및 기존 테스트·수렴 점검 후 specs/001-git-client/quickstart.md에 결과와 실제 화면 검증 범위를 기록한다. SC-117.

T321 → T322 → T323 → T324 순차 진행하며 병렬 대상은 없다. 최소 구현은 US98 전체다. 독립 수용 기준은 Space 체크/재토글→Enter 부분 Stage와 커서가 속한 두 번째 충돌만 상단 버튼으로 해결하며 원문/기존 Stage와 다른 단축키 동작을 유지하는 것이다.

## 114단계: 버튼 옆 단축키 안내 (US99)

준비·기반: 기존 Bindings·공통 버튼·키캡을 사용한다.

- [x] T325 [US99] src/shortcut.rs에 명령 기반 버튼 안내와 대체 키/빈 슬롯 키캡 표시를 구성한다. FR-299~FR-300.
- [x] T326 [US99] src/ui.rs의 기존 단축키 연결 실행/확인/취소/부분 Stage 버튼에 안내를 적용하고 이벤트·비활성 조건을 보존한다. FR-299~FR-301.
- [x] T327 [US99] 포맷·컴파일·Clippy·기존 단축키 회귀를 검증하고 specs/001-git-client/quickstart.md에 결과와 수렴 점검을 기록한다. SC-118.

T325 → T326 → T327 순서이며 동일 UI 변경이므로 병렬 대상은 없다. 최소 구현은 US99 전체다. 독립 수용 기준은 기본/재지정/빈 슬롯에서 표시와 실행 키가 일치하는 것이다.

## 115단계: 오류 표시와 순차 열람 (US100)

준비·기반: 기존 오류 필드와 공통 GPUI 카드를 사용한다.

- [x] T328 [US100] src/error_history.rs·src/main.rs에 출처별 전이·순서·페이지 모델을 구현하고 재발생/중복/경계/새 오류 수용 기준을 검증한다. FR-303~FR-304.
- [x] T329 [US100] src/ui.rs·src/settings.rs에 모든 탭/대화상자/설정 오류 수집과 알림 연결을 구성하고 기존 3초 표시를 대체한다. FR-302, FR-304~FR-305.
- [x] T330 [US100] src/ui.rs에 왼쪽 하단 느낌표·건수·오류 열람 카드/스크롤·이전/다음·닫기와 기존 작업 보존을 구현한다. FR-302~FR-305.
- [x] T331 [US100] 포맷·컴파일·Clippy·관련 회귀와 수렴 점검을 수행하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-119.

T328 → T329 → T330 → T331 순차 수행한다. 같은 상태를 수정하므로 병렬 대상은 없다. 최소 구현은 US100 전체이며 독립 수용 기준은 서로 다른 두 오류를 발생 순서대로 열람하고 재발생/탭 전환/작업 팝업을 보존하는 것이다.

## 116단계: 오류 확인완료와 최신 페이지 (US101)

준비·기반: 기존 오류 기록 모델·카드·단축키를 재사용한다.

- [x] T332 [US101] src/error_history.rs에 최신 페이지 열기·개별 확인완료와 페이지 보정을 구현하고 중복 억제/재발생·제거 경계를 검증한다. FR-307~FR-308.
- [x] T333 [US101] src/ui.rs에 바깥 클릭 닫기·입력 소비·확인완료 버튼/확인 단축키·빈 기록 숨김을 연결한다. FR-306~FR-308.
- [x] T334 [US101] 포맷·컴파일·Clippy·오류 기록 회귀와 수렴 점검을 수행하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-120.

T332 → T333 → T334 순차 진행한다. 동일 모델/UI 변경으로 병렬 대상은 없고 최소 구현은 US101 전체다. 독립 수용 기준은 최신 오류로 열어 현재 한 건만 제거하고 바깥 입력으로 기록/배경을 보존하며 닫는 것이다.

## 117단계: 오른쪽 오류 표시와 기본 키 정리 (US102)

준비·기반: 기존 오류 카드·Bindings를 재사용한다.

- [x] T335 [US102] src/shortcut.rs에 오류 기록 F10 전역 명령과 충돌 범위를 추가하고 확인 기본 Ctrl+S를 제거한다. FR-310~FR-311.
- [x] T336 [US102] src/ui.rs에 도움말 링크 제거·오른쪽 오류/카드·작은 아이콘·하단 공간 및 클릭/키 공통 토글을 적용한다. FR-309~FR-310.
- [x] T337 [US102] 포맷·컴파일·Clippy·단축키 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-121.

T335 → T336 → T337 순차 진행하며 같은 UI/명령을 변경하므로 병렬 대상은 없다. 최소 구현은 US102 전체다. 독립 수용 기준은 오른쪽 작은 오류 표시를 F10으로 토글하고 확인 기본 키가 Enter만인 것이다.

## 118단계: DiffViewer 범위·커서·버튼 (US103)

준비·기반: 기존 텍스트 선택·부분 Stage·가상 행을 재사용한다.

- [x] T338 [US103] src/text_selection.rs에 범위 행과 유효 행 커서 이동을 구현하고 경계/방향/빈 행을 검증한다. FR-312~FR-313.
- [x] T339 [US103] src/ui.rs에 범위 체크 집합·번호 행 초기화/키/전체 선택/마우스/드래그와 Stage 버튼 최소 너비를 연결한다. FR-312~FR-314.
- [x] T340 [US103] 포맷·컴파일·Clippy·부분 Stage/단축키/커서 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-122.

T338 → T339 → T340 순차 수행하며 동일 UI 상태이므로 병렬 대상은 없다. 최소 구현은 US103 전체이고 독립 수용 기준은 번호 행 사이의 범위를 잡아 여러 변경 줄을 체크하고 Enter로 기존 부분 Stage를 실행하는 것이다.

## 119단계: 오류 표시 순서와 자동 너비 (US104)

준비·기반: 기존 오류 표시·카드·두 하단 행을 사용한다.

- [x] T341 [US104] src/ui.rs에서 내용 너비 오류 표시를 분리하고 두 하단 행의 F11 왼쪽에 배치하며 고정/예약 너비를 제거한다. FR-315~FR-316.
- [x] T342 [US104] 포맷·컴파일·Clippy·관련 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-123.

T341 → T342 순차 진행하며 동일 UI 변경이므로 병렬 대상은 없다. 최소 구현은 US104 전체다. 독립 수용 기준은 두 하단에서 오류 표시가 F11 왼쪽에 내용 너비로 배치되는 것이다.

## 120단계: 오류 표시 왼쪽 배치와 배경 (US105)

준비·기반: 기존 오류 표시/카드와 두 하단 바를 사용한다.

- [x] T343 [US105] src/ui.rs의 표시/하단 배경·좌우 8px 패딩·왼쪽 배치와 카드 방향을 수정한다. FR-317~FR-318.
- [x] T344 [US105] 포맷·컴파일·Clippy·오류 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-124.

T343 → T344 순차 진행하며 동일 UI 변경이므로 병렬 대상은 없다. 최소 구현은 US105 전체다. 독립 수용 기준은 두 탭에서 오류가 왼쪽에 바탕색/여백을 맞춰 표시되는 것이다.

## 121단계: 오류 재발생 기록과 복사 (US106)

준비·기반: 기존 오류 기록/수집·입력 편집기를 재사용한다.

- [x] T345 [US106] src/error_history.rs에 명시적 발생 기록·관찰 중복 억제를 구현하고 같은 문구 재발생/확인/순서/페이지를 검증한다. FR-319~FR-320.
- [x] T346 [US106] src/ui.rs·src/settings.rs의 오류 발생 대입을 명시적 기록/설정 발생 버전에 연결한다. FR-319~FR-320.
- [x] T347 [US106] src/commit_editor.rs·src/ui.rs에 읽기 전용 오류 선택·Ctrl+C·전체 복사 버튼과 페이지 캐시/입력 보호를 구현한다. FR-321.
- [x] T348 [US106] 포맷·컴파일·Clippy·발생/선택/기존 오류 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-125.

T345 → T346 → T347 → T348 순차 수행하며 동일 상태를 변경하므로 병렬 대상은 없다. 최소 구현은 US106 전체이며 독립 수용 기준은 같은 오류의 재발생을 새 건으로 기록하고 메시지를 수정 없이 복사하는 것이다.

## 122단계: F11 팝업 위 오류 표시 투명화 (US107)

- [x] T349 [US107] src/ui.rs의 오류 표시와 느낌표 기본 배경을 명시적 투명으로 바꾸고 붉은 hover를 유지한다. FR-322.
- [x] T350 [US107] 포맷·컴파일·Clippy·오류/단축키 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-126.

기존 표시를 재사용하며 T349 → T350 순차 진행한다. 동일 UI 변경이므로 병렬 대상은 없다. 최소 구현은 US107 전체이고 독립 수용 기준은 F11 팝업 위에서 기본 배경이 투명하고 hover는 붉게 유지되는 것이다.

## 123단계: 오류 메시지 자동 줄바꿈 (US108)

- [x] T351 [US108] src/commit_editor.rs의 읽기 전용 오류 메시지에 실측 폭 줄바꿈·원문 좌표·가로 차단/세로 허용을 구현한다. FR-323~FR-324.
- [x] T352 [US108] 포맷·컴파일·Clippy·줄바꿈/선택/오류 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-127.

기존 읽기 전용 편집기를 사용하고 T351 → T352 순차 수행한다. 동일 편집기를 변경하므로 병렬 대상은 없다. 최소 구현은 US108 전체이며 독립 수용 기준은 긴 오류를 가로 이동 없이 읽고 원문 그대로 복사하는 것이다.

## 124단계: 오류 단일 기록과 Revert 충돌 파일 (US109)

- [x] T353 [US109] src/error_history.rs·src/ui.rs에 동일 실패 mirror와 브랜치 확인 키 반복 방지를 구현하고 기록 건수/재시도 회귀를 검증한다. FR-325~FR-326.
- [x] T354 [US109] src/branches.rs·src/ui.rs에 실제 Revert 충돌 경로 진단/배너 목록과 오류 보존을 구현하고 실제 저장소 회귀를 검증한다. FR-327.
- [x] T355 [US109] 포맷·컴파일·Clippy·오류/브랜치 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-128.

기존 발생 기록/복구 모델을 재사용하며 T353 → T354 → T355 순차 수행한다. 같은 UI 결과를 변경하므로 병렬 대상은 없다. 최소 구현은 US109 전체이고 독립 수용 기준은 한 실패 한 기록과 실제 Revert 충돌 파일 식별이다.

## 125단계: 브랜치·파일 대괄호 탭 이동 (US110)

- [x] T356 [US110] src/shortcut.rs에 영역별 대괄호 이전/다음 명령·History 분류를 추가하고 범위/재지정을 검증한다. FR-328~FR-329.
- [x] T357 [US110] src/ui.rs에 순환 계산·공유 탭 전환·키 처리와 목록 포커스/기존 초기화를 연결한다. FR-328, FR-330.
- [x] T358 [US110] 포맷·컴파일·Clippy·단축키/탭 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-129.

기존 탭/Bindings를 재사용하며 T356 → T357 → T358 순차 진행한다. 같은 명령/UI 변경으로 병렬 대상은 없다. 최소 구현은 US110 전체이고 독립 수용 기준은 해당 목록의 내부 탭을 [/]로 순환하는 것이다.

## 126단계: 체크박스 Space 중복 방지 (US111)

- [x] T359 [US111] src/controls.rs·src/ui.rs에 마우스 클릭/수동 키 분리와 포커스된 체크박스 키 반복 차단을 적용하고 이벤트 조합을 검증한다. FR-331~FR-332.
- [x] T360 [US111] 포맷·컴파일·Clippy·단축키/팝업 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-130.

기존 체크박스/Bindings를 재사용하고 T359 → T360 순차 수행한다. 동일 입력 경로 변경이므로 병렬 대상은 없다. 최소 구현은 US111 전체이며 독립 수용 기준은 누름/해제 한 사이클에 체크 상태가 한 번만 바뀌는 것이다.

## 127단계: 설정·프로젝트 탭과 원격 키 (US112)

- [x] T361 [US112] src/shortcut.rs에 내부 탭 공통화·설정/프로젝트 이동 범위·충돌 검사와 원격 새 기본 입력을 구성한다. FR-333~FR-336.
- [x] T362 [US112] src/ui.rs에 설정/프로젝트 공유 탭 전환과 키/캡처 순서·조회/스크롤/세션 보존을 연결한다. FR-333~FR-334.
- [x] T363 [US112] 포맷·컴파일·Clippy·범위/원격/탭 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-131.

기존 Bindings/탭 순환을 재사용하며 T361 → T362 → T363 순차 수행한다. 동일 UI/명령 변경으로 병렬 대상은 없다. 최소 구현은 US112 전체다. 독립 수용 기준은 설정 분류/프로젝트 탭의 키 분리와 원격 기본 조합 교체다.

## 128단계: 공유 단축키 설정 통합 (US113)

- [x] T364 [US113] src/shortcut.rs에 공통 전체 선택/체크아웃·범위·이전 설정 승계·중복 없는 하위 분류를 구현한다. FR-337~FR-339.
- [x] T365 [US113] src/ui.rs에 공유 섹션의 적용 대상 표시를 연결하고 설정/도움말의 중복 행과 기존 캡처/저장 동작을 확인한다. FR-338~FR-339.
- [x] T366 [US113] 포맷·컴파일·Clippy·공유 범위/설정 승계/분류 회귀와 수렴 점검 후 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-132.

기존 Bindings/명령 렌더링을 재사용하며 T364 → T365 → T366 순차 수행한다. 같은 설정 모델을 수정하므로 병렬 대상은 없다. 최소 구현은 US113 전체이고 독립 수용 기준은 한 번 설정한 공유 기능이 관련 영역에 함께 적용되고 중복 편집 행이 사라지는 것이다.
## 129단계: 공통 섹션 명칭 간소화 (US114)

- [x] T367 [US114] src/ui.rs의 공통 제목 접미사를 제거한다. FR-340.
- [x] T368 [US114] 포맷·컴파일·기존 단축키 분류 회귀를 확인하고 quickstart.md에 검증 및 수렴 결과를 기록한다. SC-133.

T367 → T368 순서로 수행한다. 독립 수용 기준은 두 공유 섹션이 공통으로만 표시되는 것이다.

## 130단계: 해제 버튼 높이 정렬 (US115)

- [x] T369 [US115] src/ui.rs의 단축키 지정/해제 버튼 높이를 맞추고 X를 가운데 정렬한다. FR-341.
- [x] T370 [US115] 포맷·컴파일과 이벤트 보존을 확인하고 quickstart.md에 검증 및 수렴 결과를 기록한다. SC-134.

T369 → T370 순차 수행한다. 두 버튼의 높이 일치가 독립 수용 기준이다.

## 131단계: 기본값 복원 단축키 (US116)

- [x] T371 [US116] src/shortcut.rs에 팝업 전용 복원 명령과 기본/범위/재지정/충돌 회귀를 추가한다. FR-342~FR-343.
- [x] T372 [US116] src/ui.rs의 복원 메서드를 버튼/키에 연결하고 키 안내 및 캡처 우선을 보존한다. FR-342~FR-343.
- [x] T373 [US116] 포맷·컴파일·단축키 회귀를 확인하고 quickstart.md에 검증 및 수렴 결과를 기록한다. SC-135.

T371 → T372 → T373 순차 수행한다. 팝업에서만 동일한 초안 복원이 실행되는 것이 독립 수용 기준이다.

## 132단계: 기본값 복원 버튼 배경 (US117)

- [x] T374 [US117] src/ui.rs의 복원 버튼에 회색 기본/호버 배경을 적용한다. FR-344.
- [x] T375 [US117] 포맷·컴파일·동작 보존을 확인하고 quickstart.md에 검증 및 수렴 결과를 기록한다. SC-136.

T374 → T375 순차 수행한다. 복원 버튼의 회색 배경이 독립 수용 기준이다.

## 133단계: 복원 버튼 중복 호버 오류 수정 (US118)

- [x] T376 [US118] src/ui.rs의 복원 버튼 추가 hover를 제거하고 회색 기본 배경을 유지한다. FR-345.
- [x] T377 [US118] 공용 버튼과 GPUI 호버 등록 경로·포맷·컴파일을 검증하고 quickstart.md에 검증 및 수렴 결과를 기록한다. SC-137.

T376 → T377 순차 수행한다. 호버의 단일 등록이 독립 수용 기준이다.

## 134단계: 단축키 설정 탭 고정 (US119)

- [x] T378 [US119] src/popup.rs에 선택적 본문 고정 헤더를 제공하고 기존 스크롤/높이/하단 영역을 유지한다. FR-346.
- [x] T379 [US119] src/ui.rs의 탭 생성을 분리해 설정 전용 고정 헤더로 연결하고 목록·메시지만 스크롤한다. FR-346~FR-347.
- [x] T380 [US119] 포맷·컴파일·Clippy·관련 회귀를 확인하고 quickstart.md에 검증 및 수렴 결과를 기록한다. SC-138.

T378 → T379 → T380 순차 수행한다. 긴 목록을 스크롤해도 분류 탭이 남는 것이 독립 수용 기준이다.

## 135단계: 세팅 단일 화면과 저장 버튼·탭 방향키 (US120)

- [x] T381 [US120] src/settings_git.rs의 전역 저장을 개별 보존 정책으로 수정하고 읽기/범위 격리·잠금·없는 저장소 회귀를 갱신한다. FR-349.
- [x] T382 [US120] src/settings.rs의 탭/체크 상태를 제거하고 단일 섹션·세 명시적 버튼·Tab 포커스/Enter/Space·불러오기/저장 처리를 구현한다. FR-348~FR-350.
- [x] T383 [US120] src/ui.rs에서 생성자/세션 선호 연결을 정리하고 포커스 버튼 실행을 기존 세팅 키 경로에 연결한다. FR-350.
- [x] T384 [US120] src/shortcut.rs의 내부 탭 기본값을 좌우로 변경하고 탐색 scope와 범위/충돌/재지정 회귀를 갱신한다. FR-351.
- [x] T385 [US120] 포맷·컴파일·Clippy·앱 회귀를 실행하고 quickstart.md에 검증 및 수렴 결과를 기록한다. SC-139.

T381 → T382 → T383 → T384 → T385 순차 수행한다. 저장 범위 격리와 포커스 실행·내부 탭 키 교체가 독립 수용 기준이다.

## 136단계: 공통 방향과 내부 탭 통합 (US121)

- [x] T386 [US121] src/shortcut.rs에서 별도 내부 탭 명령을 제거하고 공통 좌우 scope/충돌·프로젝트 두 슬롯 기본값과 회귀를 갱신한다. FR-352~FR-353.
- [x] T387 [US121] src/ui.rs의 설정 분류/패널 탭을 해석된 좌우 target에 연결하고 캡처/순환/프로젝트 경로를 유지한다. FR-352~FR-353.
- [x] T388 [US121] 포맷·컴파일·Clippy·앱 회귀와 코드/문서 수렴을 확인하고 quickstart.md에 기록한다. SC-140.

T386 → T387 → T388 순차 수행한다. 공통 방향 재지정의 내부 탭 공유 및 독립 프로젝트 두 슬롯이 수용 기준이다.

## 137단계: 전역 작성자 체크와 읽기 전용 (US122)

- [x] T389 [US122] src/settings_git.rs에 전역 사용 적용/local rollback·상태 조회와 격리 회귀를 추가한다. FR-355.
- [x] T390 [US122] src/commit_editor.rs의 읽기 전용 전환과 단일행 표시 보존을 제공한다. FR-354·FR-356.
- [x] T391 [US122] src/settings.rs에서 세 버튼을 체크/저장으로 교체하고 표시/초안 전환·포커스·범위 저장을 연결한다. FR-354~FR-356.
- [x] T392 [US122] 포맷·컴파일·Clippy·앱 회귀 및 수렴을 확인하고 quickstart.md에 기록한다. SC-141.

T389 → T390 → T391 → T392 순차 수행한다. 체크 시 전역값 읽기 전용 및 저장 시 local override만 해제하는 것이 수용 기준이다.

## 138단계: 세팅 저장 버튼 위치 통일 (US123)

- [x] T393 [US123] src/settings.rs에서 작성자 저장을 하단용 버튼 생성으로 분리하고 안내/포커스 순서를 유지한다. FR-357~FR-358.
- [x] T394 [US123] src/ui.rs의 세팅 상단 취소를 제거하고 저장을 공용 팝업 footer에 배치한다. FR-357.
- [x] T395 [US123] 포맷·컴파일·Clippy·관련 회귀 및 수렴을 확인하고 quickstart.md에 기록한다. SC-142.

T393 → T394 → T395 순차 수행한다. 세팅 우측 하단의 단일 저장과 기존 동작 유지가 수용 기준이다.

## 139단계: 저장 버튼 강조 색상 (US124)

- [x] T396 [US124] src/settings.rs의 저장 버튼에 주요 확인 버튼 테마와 포커스 구분을 적용한다. FR-359.
- [x] T397 [US124] 포맷·컴파일·중복 hover 없음과 수렴을 확인하고 quickstart.md에 기록한다. SC-143.

T396 → T397 순차 수행한다. 주요 확인 버튼과 동일한 기본 강조색이 수용 기준이다.

## 140단계: 전역 작성자 안내 제거 (US125)

- [x] T398 [US125] src/settings.rs의 작성자 안내 child를 제거한다. FR-360.
- [x] T399 [US125] 문구 제거·포맷·컴파일과 수렴을 확인하고 quickstart.md에 기록한다. SC-144.

T398 → T399 순차 수행한다. 해당 문구가 없는 것이 수용 기준이다.

## 141단계: 세팅 섹션 제목 글꼴 통일 (US126)

- [x] T400 [US126] src/settings.rs의 세 섹션 제목에 단축키 제목의 text_sm/ACCENT/SEMIBOLD 스타일을 적용한다. FR-361.
- [x] T401 [US126] 포맷·컴파일·스타일 일치와 수렴을 확인하고 quickstart.md에 기록한다. SC-145.

T400 → T401 순차 수행한다. 세 제목의 동일한 스타일이 수용 기준이다.

## 142단계: git2 실행 백엔드 전환 (US127)

- [x] T402 [US127] git2 0.21.0 의존성과 공유 인프로세스 실행 기반을 추가한다. FR-362.
- [x] T403 [US127] gix 설정/참조 조회·변경과 인증 helper 직접 호출·원격 인증 콜백을 전환한다. FR-363·FR-365.
- [x] T404 [US127] 생성/복제·fetch/pull/push와 lease/추적 브랜치 처리를 전환한다. FR-363.
- [x] T405 [US127] gix 전체/부분 index·커밋·브랜치/태그와 git2 폐기·체크아웃·안전 삭제를 전환한다. FR-363.
- [x] T406 [US127] Merge/Rebase/Revert와 계속/중단/건너뛰기 상태·데이터 보존을 전환한다. FR-364.
- [x] T407 [US127] Stash 저장/삭제/충돌 복구와 보호 상태를 전환한다. FR-364.
- [x] T408 [US127] 기존 픽스처 Git을 제품 경로와 분리하고 신규 회귀·전체 검사·정적 실행 감사 후 quickstart.md에 기록한다. SC-146.

T402 → T403 → T404 → T405 → T406 → T407 → T408 순차 수행한다. 제품 Git 실행 제거와 기존 작업의 데이터 보존이 수용 기준이다.

## 143단계: 전체 스테이징 상태 일치 복구 (US128)

- [x] T409 [US128] src/setup.rs에 격리된 시스템 줄바꿈 설정 회귀 테스트를 추가하고 실패를 재현한다. FR-366·SC-147.
- [x] T410 [US128] src/native_local.rs와 src/git.rs의 설정 병합 경로를 통합하고 조회 필터 차단을 유지한다. FR-367.
- [x] T411 [US128] 신규/수정 파일과 로컬 재정의·기존 회귀·포맷·컴파일·Clippy를 검증하고 quickstart.md에 결과를 기록한다. SC-147.

T409 → T410 → T411 순차 수행한다. 작업 파일 바이트 보존과 정확한 상태 표시가 수용 기준이다.

## 144단계: 스태시 추가 팝업 정리 (US129)

- [x] T412 [US129] src/ui.rs에서 추가 팝업 제목·입력 라벨을 변경하고 취소 버튼·일반 설명을 제거한다. 삭제 팝업과 기존 동작을 유지한다. FR-368·FR-369.
- [x] T413 [US129] 포맷·컴파일 및 표시 분기를 검토하고 quickstart.md에 검증 결과를 기록한 뒤 수렴을 확인한다. SC-148.

T412 → T413 순차 수행한다. 네 가지 표시 변경과 삭제 팝업 보존이 수용 기준이다.

## 145단계: 일반 그래프 스태시 제외 (US130)

- [x] T414 [US130] tests/repository.rs의 스태시 회귀를 일반 그래프 제외·신규 파일 보관·직접 상세 조회·일반 참조 도달성 기준으로 갱신한다. FR-370·FR-371.
- [x] T415 [US130] src/git.rs의 그래프 시작점에서 Stash 참조를 제외하고 src/ui.rs의 스태시 선택 시 추가 페이지 탐색을 막는다. FR-370·FR-371.
- [x] T416 [US130] 스태시·페이징 회귀 및 포맷·컴파일 검사를 수행하고 quickstart.md에 기록한 뒤 수렴을 확인한다. SC-149.

T414 → T415 → T416 순차 수행한다. 일반 그래프의 스태시 전용 이력 제외와 목록·내용 조회 보존이 수용 기준이다.

## 146단계: 읽기 전용 입력 배경 (US131)

- [x] T417 [US131] src/commit_editor.rs의 입력 및 스크롤바 배경을 읽기 전용이면 연한 회색, 그 외에는 기존 입력색으로 지정한다. FR-372.
- [x] T418 [US131] 포맷·컴파일과 배경 경로를 확인하고 quickstart.md에 기록한 뒤 수렴을 점검한다. SC-150.

T417 → T418 순차 수행한다. 읽기 전용 배경 구분과 기존 입력 동작 보존이 수용 기준이다.

## 147단계: 읽기 전용 입력 패딩 배경 (US132)

- [x] T419 [US132] src/commit_editor.rs의 상태별 배경을 공유하고 src/settings.rs의 입력 래퍼 패딩에도 동일 색상을 적용한다. FR-373.
- [x] T420 [US132] 포맷·컴파일과 배경 전환 경로를 검증하고 quickstart.md에 기록한 뒤 수렴을 확인한다. SC-151.

T419 → T420 순차 수행한다. 테두리 내부 배경 일치와 입력 동작 보존이 수용 기준이다.

## 148단계: 세팅 저장 버튼 포커스 제외 (US133)

- [x] T421 [US133] src/settings.rs의 저장 포커스·Tab 대상을 제거하고 체크박스 전용 핸들·활성화로 정리한다. FR-374~FR-375.
- [x] T422 [US133] src/ui.rs의 저장 버튼 호출을 갱신하고 포맷·컴파일·입력 경로를 검증하여 specs/001-git-client/quickstart.md에 기록한다. SC-152.

기존 설정 컴포넌트를 사용한다. T421 → T422 순서이며 병렬 작업은 없다. 최소 구현은 US133 전체다. 독립 수용 기준은 저장 버튼의 포커스 제외와 클릭·단축키 저장 유지다.

## 149단계: 원격 저장소 추가와 목록 (US134)

- [x] T423 [US134] src/native_local.rs의 원자 설정 편집·원격 추가와 src/settings_git.rs의 입력 검증·격리 회귀를 구현한다. FR-379·SC-153.
- [x] T424 [US134] src/settings.rs에 원격 목록·추가 버튼 포커스·입력 팝업·비동기 저장과 초안 보존을 구현한다. FR-376~FR-380.
- [x] T425 [US134] src/ui.rs의 제목·단축키·하위 팝업 닫기를 연결하고 포맷·컴파일·회귀 후 specs/001-git-client/quickstart.md에 기록한다. SC-153.

기존 기반을 재사용하며 T423 → T424 → T425 순서다. 의존 구현이므로 병렬 작업은 없다. 최소 구현은 US134 전체이며 독립 검증은 여러 원격 추가와 오류 시 설정·입력 보존이다.

## 150단계: 세팅 유지와 추가 버튼 위치 (US135)

- [x] T426 [US135] src/settings.rs에서 추가 버튼을 목록 아래로 옮기고 추가 팝업 본문·오류 표시를 세팅 본문과 분리한다. FR-381~FR-383.
- [x] T427 [US135] src/ui.rs에서 세팅 프레임을 유지하며 별도 추가 overlay·입력 차단·닫기를 연결하고 specs/001-git-client/quickstart.md에 포맷·컴파일·수렴 결과를 기록한다. SC-154.

T426 → T427 순서로 진행하며 같은 UI 경계를 변경하므로 병렬 작업은 없다. 기존 기반을 재사용하고 최소 구현은 US135 전체다. 독립 수용 기준은 목록 아래 버튼과 세팅/추가 팝업 동시 표시 및 최상위 팝업만 닫기다.

## 151단계: 원격 URL 입력 테두리 (US136)

- [x] T428 [US136] src/settings.rs의 원격 추가 본문 하단 여백과 두 입력 래퍼의 축소 방지를 적용한다. FR-384.
- [x] T429 [US136] 포맷·컴파일·스크롤 보존 경로를 확인하고 specs/001-git-client/quickstart.md에 검증 결과를 기록한다. SC-155.

T428 → T429 순서로 수행한다. 표시 변경이므로 병렬 작업은 없고 최소 구현은 US136 전체다. 독립 수용 기준은 마지막 입력의 전체 테두리 표시와 낮은 창의 스크롤 유지다.

## 152단계: 원격 삭제 아이콘 (US137)

- [x] T430 [US137] src/native_refs.rs와 src/settings_git.rs에 원격 삭제 진입·격리 회귀를 추가한다. FR-386~FR-387·SC-156.
- [x] T431 [US137] src/settings.rs에 원격별 삭제 아이콘·비동기 삭제·목록 갱신·작업 중 차단을 구현한다. FR-385~FR-387.
- [x] T432 [US137] 포맷·컴파일·설정 회귀와 수렴을 확인하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-156.

T430 → T431 → T432 순서로 실행한다. 같은 경계를 변경하므로 병렬 작업은 없다. 최소 구현은 US137 전체이며 독립 수용 기준은 선택 원격만 제거하고 다른 원격·로컬 작업을 보존하는 것이다.

## 153단계: 삭제 버튼 키보드 실행 (US138)

- [x] T433 [US138] src/settings.rs에 이름별 삭제 포커스 동기화·Tab 순서·포커스 표시·클릭 포커스·삭제 후 복귀를 구현한다. FR-388·FR-390.
- [x] T434 [US138] src/ui.rs와 src/settings.rs의 Space 전용 삭제·키 반복 보호를 연결하고 포맷·컴파일·관련 회귀 및 수렴 결과를 specs/001-git-client/quickstart.md에 기록한다. FR-389·SC-157.

T433 → T434 순차 실행하며 병렬 작업은 없다. 기존 기반을 재사용하고 최소 구현은 US138 전체다. 독립 수용 기준은 키보드로 선택한 원격만 한 번 삭제하고 안정적인 포커스로 복귀하는 것이다.

## 154단계: Fetch 오류 진단 (US139)

- [x] T435 [US139] src/native_remote.rs에서 원격별 Fetch 오류 원문·분류·코드 전달과 URL 비밀정보 가림을 구현하고 합성/로컬 실패 회귀를 추가한다. FR-391~FR-392.
- [x] T436 [US139] 기존 Fetch 회귀·포맷·컴파일과 코드 수렴을 검증하고 specs/001-git-client/quickstart.md에 결과와 실제 원격 재검증 한계를 기록한다. SC-158.

T435 → T436 순서로 수행한다. 같은 전송 경계를 변경하므로 병렬 작업은 없다. 최소 구현은 US139 전체이며 독립 수용 기준은 실제 Fetch 오류 구분과 비밀정보 가림이다.


- [x] T437 [US139] Cargo.toml·Cargo.lock에 git2 HTTPS를 활성화하고 src/native_remote.rs에 링크된 HTTPS 기능 회귀를 추가한다. FR-393·SC-159. T435 이후 실행하고 최종 검증 T436과 함께 완료한다.


## 155단계: 인증 목록과 방식별 추가 (US140)

- [x] T438 [US140] src/auth_profiles.rs·src/lib.rs·Cargo.toml에 메타데이터/보안 저장소·범위 선택·원자 저장/삭제·실패 복구와 격리 회귀를 구현한다. FR-394·FR-397·FR-400.
- [x] T439 [US140] src/credentials.rs·src/oauth.rs·src/main.rs에 GCM 계정 선택·독립 GitHub Device Flow·취소/오류 회귀를 구현하고 src/native_remote.rs에 등록 인증 조회를 연결한다. FR-396~FR-397.
- [x] T440 [US140] src/commit_editor.rs에 비밀 마스킹·복사 차단·UTF-8 경계 보호·비밀 지우기와 회귀를 추가한다. FR-398.
- [x] T441 [US140] src/auth_settings.rs·src/settings.rs·src/ui.rs에 인증 목록·삭제/추가 포커스·겹친 팝업·방식별 동적 입력·비동기 저장/취소를 연결한다. FR-394~FR-400.
- [x] T442 [US140] 포맷·컴파일·Clippy·인증/저장소/입력/전송 회귀와 수렴을 확인하고 specs/001-git-client/quickstart.md에 검증과 실제 OAuth 한계를 기록한다. SC-160.

기존 기반을 재사용하고 T438 → T439 → T440 → T441 → T442로 실행한다. 한 에이전트가 순차 구현하며 별도 병렬 작업은 없다. 최소 구현은 US140 전체이며 독립 수용 기준은 방식별 등록·재시작 목록·비밀 분리·전송 사용·선택 삭제와 취소다.

## 156단계: 인증 버튼과 기존 목록 보완 (US141)

- [x] T443 [US141] src/auth_profiles.rs에 비밀 없는 기존 인증 열거·병합·중복/숨김·과거 파일 호환과 격리 회귀를 구현한다. FR-402~FR-404·SC-161.
- [x] T444 [US141] src/auth_settings.rs에 기존 인증 로드·실패 안내·기존 인증 표시와 빈 버튼 라벨 생략을 연결한다. FR-401~FR-404.
- [x] T445 [US141] 포맷·컴파일·Clippy·인증 회귀 및 수렴을 확인하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-161.

T443 → T444 → T445로 순차 실행한다. 같은 인증 경계이므로 병렬 작업은 없다. 최소 구현은 US141 전체이며 독립 수용 기준은 기존 인증 표시·비밀 보호·재등장 방지와 정상 버튼 크기다.

## 157단계: 프로젝트 원격별 인증 표시 (US142)

- [x] T446 [US142] src/auth_profiles.rs에 원격별 정확/부모 범위 선택·합집합·경계 보호와 회귀를 구현한다. FR-405~FR-406·SC-162.
- [x] T447 [US142] src/auth_settings.rs와 src/settings.rs에서 전체/표시 상태·원격 URL 전달·로드/등록/삭제 후 필터와 핸들 갱신·빈 목록 안내를 연결한다. FR-407~FR-408.
- [x] T448 [US142] 포맷·컴파일·Clippy·관련 회귀 및 수렴을 확인하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-162.

T446 → T447 → T448로 순차 수행한다. 같은 인증 경계라 병렬 작업은 없다. 최소 구현은 US142 전체이며 독립 수용 기준은 현재 원격별 첫 일치 범위만 표시하고 변경 즉시 상위 대체를 재계산하는 것이다.

## 158단계: 인증 삭제 로그아웃 (US143)

- [x] T449 [US143] src/auth_profiles.rs에 잠금 선행·GCM 제거 콜백·실패 보존·실제 로그아웃 후 재조회와 부분 성공의 회귀를 추가한다. FR-410~FR-411·SC-163.
- [x] T450 [US143] src/credentials.rs와 src/auth_settings.rs에 계정 지정 GCM erase·방식별 실제 삭제·로그아웃 안내를 연결하고 원격 필터 재계산을 유지한다. FR-409~FR-412.
- [x] T451 [US143] 포맷·컴파일·Clippy·인증/원격 회귀와 수렴을 확인하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-163.

T449 → T450 → T451로 실행한다. 같은 경계이므로 병렬 작업은 없다. 최소 구현은 US143 전체이며 독립 수용 기준은 GCM erase가 목록 변경보다 먼저 성공하고 실패 시 기존 등록을 유지하는 것이다.

## 159단계: 오류 아이콘 키 배지 (US144)

- [x] T452 [US144] src/ui.rs의 공용 오류 표시에서 개수 다음에 현재 errors 단축키 배지를 표시한다. FR-413~FR-414.
- [x] T453 [US144] 포맷·컴파일·기존 오류 단축키 회귀와 수렴을 확인하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-164.

T452 → T453으로 순차 수행한다. 단일 표시 경계라 병렬 작업은 없다. 최소 구현은 US144 전체이며 독립 수용 기준은 느낌표·개수·실제 단축키 순서의 배지다.

## 160단계: 오류 표시 배경과 F1 (US145)

- [x] T454 [US145] src/ui.rs의 두 오류 표시 우선 렌더를 제거하고 src/shortcut.rs·src/ui.rs에서 기본 errors 단축키와 디스패치를 F1로 바꾼다. FR-415~FR-416.
- [x] T455 [US145] src/shortcut.rs의 오류 전역/재지정 회귀에 F1·F10 기본 해제를 반영하고 포맷·컴파일·Clippy·수렴 결과를 specs/001-git-client/quickstart.md에 기록한다. SC-165.

T454 → T455로 순차 수행한다. 같은 UI/명령 변경이라 병렬 작업은 없다. 최소 구현은 US145 전체이며 독립 수용 기준은 팝업 뒤의 오류 표시와 F1 전역 동작이다.

## 161단계: GCM 삭제 잔존 수정 (US146)

- [x] T456 [US146] src/auth_profiles.rs에 기존 대상 열거/삭제 추상화·정확 범위 제거·잔존 확인·다른 계정 보호와 가짜 저장소 회귀를 구현한다. FR-417~FR-419.
- [x] T457 [US146] src/credentials.rs와 src/auth_settings.rs에서 GCM 종료 코드와 Windows 제거 증거를 결합하고 실패 시 등록을 보존하는 회귀를 추가한다. SC-166.
- [x] T458 [US146] 포맷·컴파일·Clippy·관련 회귀와 수렴을 확인하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-166.

T456 → T457 → T458로 순차 실행한다. 같은 인증 경계라 병렬 작업은 없다. 최소 구현은 US146 전체이며 독립 수용 기준은 무효 성공을 거부하고 선택 계정만 실제로 제거하는 것이다.

## 162단계: GCM 경로 보존과 프로젝트 문맥 (US147)

- [x] T459 [US147] src/auth_profiles.rs에서 .git 제거를 없애고 표준 URL 비교·등록 URL 전달·GCM 요청 구성 및 회귀를 구현한다. FR-420~FR-421.
- [x] T460 [US147] src/native_remote.rs에서 프로젝트별 helper CWD와 등록 URL 조회·Clone 부모 문맥·Push 원인 및 서버 거부 이유를 연결하고 회귀를 추가한다. FR-422~FR-424.
- [x] T461 [US147] 포맷·컴파일·Clippy·인증/전송 회귀와 수렴을 확인하고 specs/001-git-client/quickstart.md에 진단 근거·실제 사용자 환경 한계를 기록한다. SC-167.

T459 → T460 → T461로 순차 수행한다. 같은 인증 경계라 병렬 작업은 없다. 최소 구현은 US147 전체이며 독립 수용 기준은 정확한 .git 주소와 프로젝트 문맥의 일치 및 실제 Push 오류 전달이다.

## 163단계: 인증 추가 방식 정리 (US148)

- [x] T462 [US148] src/auth_settings.rs의 최상위 GCM 선택·안내를 제거하고 두 방식/포커스·브라우저 기본·GCM 대화식 실행·브라우저 계정 ID 제거로 정리한다. FR-425~FR-427.
- [x] T463 [US148] 포맷·컴파일·Clippy·기존 인증 회귀와 수렴을 확인하고 specs/001-git-client/quickstart.md에 결과를 기록한다. SC-168.

T462 → T463으로 순차 수행한다. 단일 UI 경계라 병렬 작업은 없다. 최소 구현은 US148 전체이며 독립 수용 기준은 두 방식 표시와 기존 GCM 기능/데이터 보존이다.



## 164단계: 독립 OAuth 등록 제거 (US149)

- [x] T464 [US149] src/auth_settings.rs의 OAuth UI·상태·포커스·등록 경로를 제거하고 GCM 로그인과 안내를 정리한다. FR-428.
- [x] T465 [US149] src/main.rs·src/oauth.rs·Cargo.toml·Cargo.lock에서 직접 OAuth 모듈과 전용 의존성을 제거하고 기존 프로필은 유지한다. FR-429.
- [x] T466 [US149] 포맷·컴파일·Clippy·인증 회귀를 확인하고 quickstart.md에 결과를 기록한 뒤 수렴을 점검한다. SC-169.
- [x] T467 [US149] src/auth_settings.rs의 GitHub Personal Access Token 안내를 제거하고 포맷·컴파일로 확인한다. FR-428 추가 요구.

T464 → T465 → T467 → T466 순서이며 단일 인증 경계로 병렬 작업은 없다. 최소 구현은 US149 전체이다.


## 165단계: 프로젝트 탐색기 열기 (US150)

- [x] T468 [US150] src/file_manager.rs·src/main.rs에 세 플랫폼 파일 관리자 실행과 경로 인수 검증을 구현한다. FR-431.
- [x] T469 [US150] src/ui.rs·src/shortcut.rs에 footer 버튼·F10·재설정 반영·오류 기록을 연결한다. FR-430~432.
- [x] T470 [US150] 포맷·컴파일·Clippy·관련 회귀를 수행하고 quickstart.md 기록과 수렴 점검을 완료한다. SC-170.

T468 → T469 → T470 순차 수행한다. 최소 구현은 US150 전체이다.


## 166단계: 프로젝트 Bash 터미널 열기 (US151)

- [x] T471 [US151] src/terminal.rs·src/main.rs에 플랫폼별 Bash/터미널 탐색·안전한 프로젝트 경로 전달·실행 오류 처리와 회귀를 구현한다. FR-434~435.
- [x] T472 [US151] src/ui.rs·src/shortcut.rs에 탐색기 왼쪽 터미널 버튼·F9·재설정 반영과 오류 기록을 연결하고 단축키 회귀를 확인한다. FR-433~435.
- [x] T473 [US151] 포맷·컴파일·Clippy·실행 계획/단축키 회귀와 수렴을 확인하고 quickstart.md에 결과와 플랫폼 검증 범위를 기록한다. SC-171.

T471 → T472 → T473 순차 수행한다. 최소 구현은 US151 전체이며 동일 UI/실행 경계로 병렬 작업은 없다.


## 167단계: 브랜치 Search (US152)

- [x] T474 [US152] src/git_search.rs에 모든 브랜치·태그의 커밋 메시지·파일 경로·파일 내용 검색과 파일별 커밋 이력을 구현한다.
- [x] T475 [US152] src/ui.rs 및 모델에 Tag 오른쪽 Search 탭과 입력·결과·키보드/더블클릭 동작을 연결한다.
- [x] T476 [US152] 별도 창에서 파일 커밋 이력과 독립 DiffViewer UI를 구현한다.
- [x] T477 [US152] 포맷·컴파일·회귀 검증 후 코드와 명세 수렴을 확인한다.

T474 → T475 → T476 → T477 순서로 수행한다.


## 168단계: Search 표시와 검색 보완 (US153)

- [x] T478 [US153] src/ui.rs에서 탭 활성 상태·입력 크기/정렬/패딩·결과 글자 크기를 조정한다. FR-436~437.
- [x] T479 [US153] src/git_search.rs에서 커밋 메시지 제목/본문 일치 검색과 결과 문맥 회귀를 구현한다. FR-438.
- [x] T480 [US153] src/search_window.rs에서 창 글자 크기·Diff 라인 번호·패딩을 조정한다. FR-439.
- [x] T481 [US153] 포맷·컴파일·Clippy·관련 회귀 및 수렴을 확인한다. SC-172.

T478 → T479 → T480 → T481 순서로 수행한다.


## 169단계: Search 결과·파일 창 조작 보완 (US154)

- [x] T482 [US154] src/ui.rs에서 결과 크기·유형 배지·포커스 스타일과 Search 유지 Space 동작을 구현한다. FR-440~441.
- [x] T483 [US154] src/git_search.rs의 파일명 검색 회귀를 확인하고 누락 시 보완한다. FR-441.
- [x] T484 [US154] src/search_window.rs의 높이·선택 가능한 Diff·1/2/Esc를 구현한다. FR-442~443.
- [x] T485 [US154] 부모창 차단과 자식 종료 복구를 연결하고 포맷·컴파일·회귀·수렴을 확인한다. FR-443, SC-173.

T482 → T483 → T484 → T485 순서로 수행한다.


## 170단계: Search 반복 검색과 파일 Diff 스타일 (US155)

- [x] T486 [US155] src/ui.rs의 Search 좌우 탭 이동·입력 재진입·반복 검색을 구현한다. FR-444~445.
- [x] T487 [US155] src/git_search.rs의 제목/본문 검색과 재검색 회귀를 확인하고 보완한다. FR-445.
- [x] T488 [US155] src/search_window.rs의 독립 Diff 색/행/흰 배경·포커스 스타일을 구현한다. FR-446.
- [x] T489 [US155] 포맷·컴파일·관련 회귀와 수렴을 확인한다. SC-174.

T486 → T487 → T488 → T489 순차 수행한다.

## 171단계: Search 호버와 파일 이력 창 개선 (US156)

- [x] T490 [US156] Search 결과 행과 파일 이력 행의 호버 규칙을 기존 목록과 일치시킨다. FR-447~448.
- [x] T491 [US156] 파일 이력 목록 위·아래 방향키, 선택 Diff 갱신, 보이는 위치 스크롤을 구현한다. FR-448.
- [x] T492 [US156] 파일 Diff의 줄 번호·기호·본문 열을 분리하고 색과 선택·복사를 확인한다. FR-449.
- [x] T493 [US156] 파일 창의 폭 드래그 및 중립 구분선을 구현한다. FR-450.
- [x] T494 [US156] 포맷·컴파일·관련 회귀와 수렴을 확인한다. SC-175.

T490 → T491 → T492 → T493 → T494 순서로 수행한다.

## 172단계: 파일 이력 가상 스크롤·Diff 열 (US157)

- [x] T495 [US157] src/search_window.rs 커밋 목록을 `uniform_list` 가상 스크롤로 전환하고 선택·호버·키보드를 유지한다. FR-451.
- [x] T496 [US157] src/commit_editor.rs 파일 Diff를 가시 줄만 형상화·그리도록 바꾸고 커서·스크롤 좌표를 보존한다. FR-452.
- [x] T497 [US157] Diff 본문 선택 제한, 기호 중앙 정렬, 회색 번호 배경과 두 경계선을 구현한다. FR-454~455.
- [x] T498 [US157] src/search_window.rs 크기 조절 손잡이 양쪽 공백을 제거한다. FR-453.
- [x] T499 [US157] 포맷·컴파일·관련 회귀와 수렴을 확인한다. SC-176.

T495 → T496 → T497 → T498 → T499 순서로 수행한다.

## 173단계: 파일 이력 커밋 행 전체 폭 (US158)

- [x] T500 [US158] src/search_window.rs 가상 목록 커밋 행의 너비를 목록 폭 100%로 지정하고 긴 제목을 행 안에서 잘라 표시한다. FR-456.
- [x] T501 [US158] 포맷·컴파일·관련 회귀와 명세 수렴을 확인한다. SC-177.

T500 → T501 순서로 수행한다.

## 174단계: 파일 이력 그리드와 세로 분할 (US159)

- [x] T502 [US159] src/git_search.rs 파일 이력에 작성자·시각을 포함하고 관련 회귀를 갱신한다. FR-457~458.
- [x] T503 [US159] src/search_window.rs 네 열 헤더·그리드 행을 구현하고 가상 목록·선택 기능을 유지한다. FR-457~458, FR-460.
- [x] T504 [US159] 파일 이력 창의 목록 위·Diff 아래 배치와 세로 크기 조절을 구현한다. FR-459.
- [x] T505 [US159] 포맷·컴파일·관련 회귀와 수렴을 확인한다. SC-178.

T502 → T503 → T504 → T505 순서로 수행한다.

## 175단계: 파일 이력 창 크기와 방향키 (US160)

- [x] T506 [US160] src/ui.rs 파일 창 기본 크기를 줄이고 현재 공통 단축키 설정을 자식 창에 전달한다. FR-461~462.
- [x] T507 [US160] src/search_window.rs 방향 이동을 기존 바인딩으로 해석하고 i/j/k/l 및 사용자 변경값을 적용한다. FR-462~463.
- [x] T508 [US160] 기본·변경 바인딩 회귀, 포맷·컴파일과 명세 수렴을 확인한다. SC-179.

T506 → T507 → T508 순서로 수행한다.

## 176단계: 파일 이력 DiffViewer 단축키 (US161)

- [x] T509 [US161] src/commit_editor.rs 읽기 전용 키 처리를 매핑 키로 호출할 수 있게 분리한다. FR-464~466.
- [x] T510 [US161] src/search_window.rs Diff 포커스의 viewer 단축키 해석과 해제된 원시 키 차단을 연결한다. FR-464~465.
- [x] T511 [US161] 기본·변경 바인딩 회귀, 포맷·컴파일과 명세 수렴을 확인한다. SC-180.

T509 → T510 → T511 순서로 수행한다.

## 177단계: 파일 이력 빈 그리드 선 (US162)

- [x] T512 [US162] src/search_window.rs 목록 패널의 빈 영역에 기존 네 열 경계선의 연장선을 그린다. FR-467~468.
- [x] T513 [US162] 포맷·컴파일·관련 회귀와 명세 수렴을 확인한다. SC-181.

T512 → T513 순서로 수행한다.

## 178단계: 파일 Diff 빈 열 연장 (US163)

- [x] T514 [US163] src/commit_editor.rs 파일 Diff 캔버스의 회색 번호 배경과 번호·기호 경계선을 빈 영역 아래까지 연장한다. FR-469~470.
- [x] T515 [US163] 포맷·컴파일·관련 회귀와 명세 수렴을 확인한다. SC-182.

T514 → T515 순서로 수행한다.

## 179단계: 기존 DiffViewer 열 배경과 연장선 (US164)

- [x] T516 [US164] src/ui.rs 기존 Diff 행의 번호 회색 배경과 번호·기호 열 오른쪽 경계선을 구현한다. FR-471~472.
- [x] T517 [US164] 짧은 Diff 빈 영역의 열 배경·경계 가이드를 부분 Stage 및 가로 스크롤 위치에 맞춰 구현한다. FR-473.
- [x] T518 [US164] 포맷·컴파일·관련 회귀와 명세 수렴을 확인한다. SC-183.

T516 → T517 → T518 순서로 수행한다.

## 180단계: 기존 DiffViewer 선택·드래그 (US165)

- [x] T519 [US165] src/ui.rs Diff 파일명 헤더 아래쪽 경계선을 적용한다. FR-474.
- [x] T520 [US165] 줄 번호 없는 Diff 행의 커서·선택 제한을 제거하고 부분 Stage 행 필터를 유지한다. FR-475.
- [x] T521 [US165] 뷰어 섹션 밖에서도 드래그 선택 끝점을 갱신하고 해제 시 종료한다. FR-476.
- [x] T522 [US165] 포맷·컴파일·관련 회귀와 명세 수렴을 확인한다. SC-184.

T519 → T520 → T521 → T522 순서로 수행한다.

## 181단계: 기존 DiffViewer 행 정렬과 빈 영역 포커스 (US166)

- [x] T523 [US166] src/ui.rs Diff 줄 번호 셀을 행 중앙에 맞추고 본문 글리프의 줄 높이 정렬을 확인한다. FR-477.
- [x] T524 [US166] src/ui.rs DiffViewer 빈 영역 클릭 시 키보드 포커스를 설정하고 기존 선택을 보존한다. FR-478.
- [x] T525 [US166] 포맷·컴파일·정적 검사 및 명세 수렴을 확인한다. SC-185.

T523 → T524 → T525 순서로 수행한다.

## 182단계: 기존 DiffViewer 변경 기호 세로 정렬 (US167)

- [x] T526 [US167] src/ui.rs 변경 기호 셀을 본문과 같은 22px 텍스트 줄 높이로 정렬한다. FR-479~480.
- [x] T527 [US167] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-186.

T526 → T527 순서로 수행한다.

## 183단계: 기존 DiffViewer 줄 번호 오른쪽 정렬 (US168)

- [x] T528 [US168] src/ui.rs 이전·현재 줄 번호 셀을 각각 오른쪽 정렬한다. FR-481~482.
- [x] T529 [US168] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-187.

T528 → T529 순서로 수행한다.

## 184단계: Search 탭 방향키 포커스 (US169)

- [x] T530 [US169] src/ui.rs Search 진입 시 가상 선택 위치와 Workspace 포커스를 설정한다. FR-483.
- [x] T531 [US169] src/ui.rs 가상 위치·입력창·결과 목록의 위아래 방향키 이동을 연결한다. FR-484~486.
- [x] T532 [US169] 포맷·컴파일·정적 검사와 이동 회귀 및 명세 수렴을 확인한다. SC-188.

T530 → T531 → T532 순서로 수행한다.

## 185단계: 브랜치 Search 탭 좌우 순환 (US170)

- [x] T533 [US170] src/ui.rs Search를 포함한 다섯 참조 탭의 공통 순환을 구현한다. FR-487~488.
- [x] T534 [US170] src/ui.rs Search 활성 상태에서도 파일 탭의 좌우 이동을 유지한다. FR-489.
- [x] T535 [US170] 정방향·역방향 단위 테스트, 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-189.

T533 → T534 → T535 순서로 수행한다.

## 186단계: 목록 빈 공간 포커스 (US171)

- [x] T536 [US171] src/ui.rs 브랜치·파일·커밋 히스토리 스크롤 프레임의 빈 공간 클릭 시 해당 목록으로 포커스를 이동한다. FR-490~491.
- [x] T537 [US171] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-190.

T536 → T537 순서로 수행한다.

## 187단계: 파일명 세로 중앙 정렬 (US172)

- [x] T538 [US172] src/ui.rs 파일명 셀을 24px 행의 세로 중앙에 정렬한다. FR-492~493.
- [x] T539 [US172] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-191.

T538 → T539 순서로 수행한다.

## 188단계: Stage 파일 상태 문자 크기 (US173)

- [x] T540 [US173] src/ui.rs Stage 상태 문자에만 큰 글자 크기를 적용한다. FR-494~495.
- [x] T541 [US173] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-192.

T540 → T541 순서로 수행한다.

## 189단계: 부분 Stage 체크 열 스타일 (US174)

- [x] T542 [US174] src/ui.rs 실제 Diff 행의 체크 열에 회색 배경·오른쪽 경계선과 아이콘 중앙 정렬을 적용한다. FR-496·498.
- [x] T543 [US174] src/ui.rs 체크 열 가이드의 배경·경계선을 빈 영역 아래까지 연장한다. FR-497.
- [x] T544 [US174] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-193.

T542 → T543 → T544 순서로 수행한다.

## 190단계: 체크 열 경계 제거와 Stage 글꼴 통일 (US175)

- [x] T545 [US175] src/ui.rs 실제 Diff 행과 가이드에서 체크 열 오른쪽 경계선을 제거한다. FR-499.
- [x] T546 [US175] src/ui.rs Stage 상태 문자의 글꼴 지정을 제거해 History와 통일한다. FR-500.
- [x] T547 [US175] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-194.

T545 → T546 → T547 순서로 수행한다.

## 191단계: Stage 상태 두 칸 정렬 (US176)

- [x] T548 [US176] src/ui.rs Stage 상태 문자열을 staged·unstaged의 같은 폭 두 칸으로 표시한다. FR-501~502.
- [x] T549 [US176] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-195.

T548 → T549 순서로 수행한다.

## 192단계: Stage 상태 간격 축소 (US177)

- [x] T550 [US177] src/ui.rs Stage 상태 두 칸을 13px씩으로 줄이고 전체 폭을 26px로 맞춘다. FR-503~504.
- [x] T551 [US177] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-196.

T550 → T551 순서로 수행한다.

## 193단계: Stage 상태 칸 11px 조정 (US178)

- [x] T552 [US178] src/ui.rs Stage 상태 두 칸을 11px씩으로 줄이고 전체 폭을 22px로 맞춘다. FR-505~506.
- [x] T553 [US178] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-197.

T552 → T553 순서로 수행한다.

## 194단계: 파일 상태 글자 크기 통일 (US179)

- [x] T554 [US179] src/ui.rs Stage·History 상태 셀에 같은 `text_xs` 크기를 적용한다. FR-507~508.
- [x] T555 [US179] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-198.

T554 → T555 순서로 수행한다.

## 195단계: Stage 상태 칸 9px 조정 (US180)

- [x] T556 [US180] src/ui.rs Stage 상태 두 칸을 9px씩으로 줄이고 전체 폭을 18px로 맞춘다. FR-509~510.
- [x] T557 [US180] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-199.

T556 → T557 순서로 수행한다.

## 196단계: Stage 상태 세로 정렬 (US181)

- [x] T558 [US181] src/ui.rs Stage 상태 셀과 문자 칸에 전체 행 높이·24px 줄 높이를 적용한다. FR-511~512.
- [x] T559 [US181] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-200.

T558 → T559 순서로 수행한다.

## 197단계: 브랜치 빈 목록 안내 여백 (US182)

- [x] T560 [US182] src/ui.rs 브랜치의 빈 목록 안내에 좌우 12px·상하 8px 패딩을 적용한다. FR-513~514.
- [x] T561 [US182] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-201.

T560 → T561 순서로 수행한다.

## 198단계: 파일명 글자 크기 통일 (US183)

- [x] T562 [US183] src/ui.rs Stage·History 파일명 셀에 `text_xs`를 적용한다. FR-515~516.
- [x] T563 [US183] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-202.

T562 → T563 순서로 수행한다.

## 199단계: 파일 히스토리 그리드 열 너비 조절 (US184)

- [x] T564 [US184] src/search_window.rs 창 로컬 열 너비와 세 경계의 제한된 크기 조절 계산을 구현한다. FR-517·519.
- [x] T565 [US184] src/search_window.rs 헤더 드래그 핸들을 추가하고 행·가이드에 동일한 열 너비를 적용한다. FR-518.
- [x] T566 [US184] 경계·최소 폭 단위 테스트, 포맷·컴파일·정적 검사 및 명세 수렴을 확인한다. SC-203.

T564 → T565 → T566 순서로 수행한다.

## 200단계: 기존 디렉터리 신규 저장소 초기화 (US185)

- [x] T567 [US185] src/ui.rs 신규 저장소에 기존 디렉터리 선택을 사용하고 Clone의 새 경로 선택을 유지한다. FR-520·522.
- [x] T568 [US185] src/setup.rs 기존 디렉터리의 `.git` 초기화·중복 방지·Windows 숨김 속성을 구현한다. FR-520~522.
- [x] T569 [US185] 파일 보존·중복·숨김 속성 회귀 및 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-204.

T567 → T568 → T569 순서로 수행한다.

## 201단계: 최근 저장소 관리 취소 버튼 제거 (US186)

- [x] T570 [US186] src/ui.rs 최근 저장소 관리 팝업 footer에서 취소 버튼만 제거한다. FR-523~524.
- [x] T571 [US186] 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-205.

T570 → T571 순서로 수행한다.

## 202단계: 기존 디렉터리에 Clone (US187)

- [x] T572 [US187] src/ui.rs Clone에서 기존 디렉터리 선택을 사용하고 `repository` 새 폴더 경로 제안을 제거한다. FR-525.
- [x] T573 [US187] src/setup.rs 빈 디렉터리 검증·복제·Windows `.git` 숨김 속성을 구현한다. FR-526~527.
- [x] T574 [US187] 복제·기존 내용 보호·숨김 속성 회귀 및 포맷·컴파일·정적 검사와 명세 수렴을 확인한다. SC-206.

T572 → T573 → T574 순서로 수행한다.

## 203단계: 저장소 폴더 직접 삭제 (US188)

- [x] T575 [US188] src/ui.rs 삭제 확인 팝업의 취소 버튼·문구와 실행 경로를 수정한다. FR-528~529.
- [x] T576 [US188] src/setup.rs에서 검증된 저장소 폴더를 직접 삭제하고 휴지통 구현을 제거한다. FR-529~530.
- [x] T577 [US188] 임시 저장소 삭제·보호 회귀와 포맷·컴파일을 확인한다. SC-207.

## 204단계: Clone 실행 버튼 문구 (US189)

- [x] T578 [US189] src/ui.rs 버튼 문구를 `Clone 실행`으로 수정한다. FR-531.
- [x] T579 [US189] 문구와 포맷·컴파일을 확인한다. SC-208.

## 205단계: 새 탭 추가 단축키 (US190)

- [x] T580 [US190] src/shortcut.rs에 공통 새 탭 명령과 기본 `ctrl-n`을 등록한다. FR-532~533.
- [x] T581 [US190] src/ui.rs에서 새 탭 명령을 기존 추가 흐름에 연결한다. FR-532.
- [x] T582 [US190] 기본·변경 키 회귀와 포맷·컴파일을 확인한다. SC-209.

## 206단계: 새 탭 버튼 키보드 조작 (US191)

- [x] T583 [US191] src/ui.rs에 새 탭 버튼·URL 입력·최근 저장소 행의 Tab 포커스 흐름과 강조 스타일을 구현한다. FR-534·536.
- [x] T584 [US191] 포커스된 버튼의 Space 실행과 처리 중 보호를 구현한다. FR-535.
- [x] T585 [US191] 순서·역순·Space 경로 회귀 및 포맷·컴파일을 확인하고 명세와 수렴한다. SC-210.

## 207단계: 저장소 탭 닫기 단축키 (US192)

- [x] T586 [US192] src/shortcut.rs에 기본 `ctrl-x` 탭 닫기 명령을 등록하고 변경 키·범위를 검증한다. FR-537~538.
- [x] T587 [US192] src/ui.rs에서 활성 저장소 탭에 기존 닫기 흐름을 연결한다. FR-537~539.
- [x] T588 [US192] 단축키·탭 흐름 회귀, 포맷·컴파일·정적 검사 및 명세 수렴을 확인한다. SC-211.
