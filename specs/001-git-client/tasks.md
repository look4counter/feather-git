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
