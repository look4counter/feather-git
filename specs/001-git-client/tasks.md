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
