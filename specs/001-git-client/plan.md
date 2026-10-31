# 구현 계획: 단일 화면 Git 클라이언트

## 팝업 입력 드래그 보호 설계 (US86)

기존 Rust·GPUI 0.2.2와 공통 카드 구조를 유지한다. src/popup.rs의 카드에서 왼쪽 MouseDown 버블 전파를 차단하여 상위 배경의 클릭 대기 상태가 만들어지지 않게 한다. 자식 입력과 버튼은 먼저 이벤트를 처리하며 기존 카드 on_click 차단은 유지한다. 별도 전역 상태나 좌표 저장을 추가하지 않는다. src/ui.rs의 배경 닫기·진행 보호·키보드 취소는 변경하지 않는다. 네 경계 시나리오를 이벤트 등록 및 전파 순서로 검토하고 fmt/check/clippy와 기존 팝업·입력 테스트를 실행한다. 실제 화면 검증이 없으면 이를 기록한다. 헌법은 템플릿이며 한글 문서·전체 절차 준수에 위반이 없다.

**일자**: 2026-10-26 | **명세**: [기능 명세](spec.md)

## 요약

Rust 2024와 GPUI 0.2.2로 Windows 앱을 구성하고 gix 0.87.1로 실제 저장소를 읽는다. 상단 프로젝트 탭 아래 초기 왼쪽 30%, 오른쪽 70%, 상하 50%로 네 영역을 배치한다. 본문과 영역 사이 여백 및 카드 모서리 둥글림 없이 맞붙이고 공통 세로·가로 경계선으로 크기를 조절한다. 어두운 테마, 청록색 선택 강조, 고정 헤더와 독립 스크롤을 제공한다.

## 기술 문맥

- 설치된 Rust 1.98, MSVC 및 Windows SDK를 사용한다.
- GPUI, gix, anyhow, 텍스트 비교용 similar를 사용한다.
- 조회는 gix로 수행하고 신규 생성·Clone·fetch·선택 파일 스테이징에 Git CLI를 사용하며 확인 후 저장소 휴지통 이동도 허용한다.
- 조회는 백그라운드에서 수행한다. 프로젝트 ID와 요청 세대로 오래된 결과를 버린다.
- 커밋은 전체 도달 가능한 이력을 읽으며 파일 미리보기는 1 MiB, 차이는 5,000줄로 제한한다.
- 상태는 gix status, 커밋 파일은 첫 부모와 트리 비교, stash는 reflog로 조회한다.
- 이름 변경은 삭제/추가로 나타날 수 있다. 외부 필터는 실행하지 않는다.

## 원칙 점검

헌법은 미작성 템플릿이므로 추가 확정 규칙은 없다. 한글 Markdown, 전체 speckit 절차, 사용자 확정 읽기 전용 범위를 준수한다. 설계 전후 모두 통과한다.

## 프로젝트 구조

```text
src/main.rs             네이티브 창 및 실행 인자
src/lib.rs              조회 모듈 공개
src/model.rs            프로젝트 및 선택 상태
src/git.rs              gix 조회 및 파일 비교
src/ui.rs               네 영역과 비동기 조회
src/layout.rs           분할 비율과 최소 크기 계산
 tests/repository.rs    임시 실제 저장소 통합 검증
```

## 단계와 검증

공통 구조 → 프로젝트 화면 → 참조 및 커밋 → 상태와 차이 → 통합 검증 순으로 구현한다. cargo fmt, check, clippy, test를 수행한다. 초기화·Clone 및 통합 검증 준비에 Git CLI를 사용한다. 두 프로젝트 전환, 참조·파일 탭, 초기 저장소, 바이너리, 오류와 로딩을 검증하고 수렴 점검한다.

## 추가 설계: 맞붙은 영역 및 크기 조절

- src/ui.rs의 본문 gap과 padding, 패널 테두리·둥글림을 제거하고 경계선은 오버레이로 한 번만 그린다.
- 좌·우 열과 위·아래 행은 각각 같은 비율을 공유한다. Workspace에 분할 상태와 본문 실제 Bounds를 둔다. 프로젝트 전환 시 유지한다.
- GPUI canvas의 prepaint에서 본문 좌표를 측정한다. 경계선은 1px, 투명한 포인터 영역은 7px로 콘텐츠 위에 겹친다.
- 경계선 왼쪽 버튼 누름으로 드래그를 시작하고 canvas paint에서 등록한 창 전역 캡처 이벤트로 본문 상대 좌표를 비율로 변환한다. 전역 마우스 해제로 종료하며, 왼쪽 버튼이 눌리지 않은 이동도 종료 처리한다. GPUI의 occlude가 상위 hitbox를 가리므로 루트 hitbox 대신 전역 이벤트를 사용한다.
- src/layout.rs에서 좌측 260px, 우측 360px, 위·아래 180px을 보장한다. 가용 공간이 부족하면 최소 크기를 비례 축소한다. 창 축소 후 확대 시 저장한 비율을 재사용한다.
- 수치 검증으로 경계·최소 크기·창 변경을 확인하고 fmt/check/clippy 및 기존 통합 테스트를 실행한다. 이전 Computer Use 앱 실행 거절로 실제 드래그 시각 검증은 미확인임을 유지한다.

## 추가 설계: 전체 높이 탭 바

- src/ui.rs에서 저장소 탭과 빈 탭을 하나의 WorkspaceTab 열거형으로 관리한다. 빈 탭에는 Project와 경로가 없다. ID는 기존 next_id로 발급한다.
- 상단은 높이 44px, 여백·둥글림 없는 사각 탭으로 구성한다. 로고·앱 이름·새로고침·저장소 열기 버튼 및 폴더 선택 처리를 제거한다.
- 각 탭은 전체 높이를 채우고 우측 닫기 버튼은 중앙 24px 정사각형을 사용한다. 닫기 클릭은 이벤트 전파를 막아 삭제한 탭이 다시 활성화되지 않게 한다.
- +는 가로 스크롤 목록 바깥 맨 오른쪽의 고정 44px 영역이다. 클릭하면 새 Empty 탭을 추가하고 활성화한다.
- GPUI ScrollHandle로 새로 활성화되거나 닫기 후 선택되는 탭을 가로 스크롤 영역 안에 보이게 한다.
- 빈 탭은 세 저장소 시작 선택지와 필요한 입력·진행·오류를 표시한다. 저장소 패널·상태 표시줄은 렌더링하지 않는다. 잘못된 실행 경로의 오류는 별도 오류 문구로 유지한다.
- 저장소 탭만 기존 gix 조회와 비동기 결과 적용의 대상이다. 마지막 탭을 닫으면 빈 탭을 만든다. 실행 인자 없이 시작해도 빈 탭을 만든다.
- 검증은 fmt/check/clippy와 기존 전체 테스트로 수행하며, 코드 검토로 빈 탭의 비조회·독립 ID·닫기 전파·고정 + 및 패널 렌더링 분기를 확인한다. 단순 스타일·목록 연산을 복제하는 테스트는 추가하지 않는다.

## 새 탭 시작 설계

src/setup.rs에서 셸 없이 Git 초기화와 Clone을 실행한다. create_dir로 새 대상을 확보하여 기존 경로를 보호한다. Windows 콘솔 창을 숨기고 터미널 인증 입력은 비활성화한다. 기존 조회는 gix로 유지한다. src/ui.rs에서 탭별 URL·오류·진행 상태와 포커스를 관리한다. 네이티브 폴더 선택 및 새 경로 선택 후 백그라운드에서 실행하고 동일 ID 탭에 결과를 반영한다. 닫기 버튼은 중앙 24px로 제한한다. 실제 로컬 초기화·Clone 및 기존 대상 거절을 테스트한다. 헌법은 미작성 템플릿이므로 한글 문서와 전체 speckit 절차를 따른다.

## 커밋 그리드·상세 설계

src/graph.rs는 zit의 build_commit_graph 레인 알고리즘을 현재 Commit 모델에 맞춰 이식한다. 커밋 부모 ID와 입력/출력 레인·색상·노드를 모델에 추가한다. src/git.rs는 선택된 커밋의 상세를 Snapshot에 포함하여 동일 조회 세대 보호를 사용한다. src/history_view.rs는 Graph·Description·Author·Date·Hash의 공통 열 너비와 24px 행을 정의하고 GPUI canvas에 12px 레인 간격·2px 선·베지어 곡선·9px 노드를 그린다. 조회 이후 행별 상대 연결점을 캐시한다. 사용 중인 GPUI 0.2.2는 완성 경로 좌표를 외부에 공개하지 않으므로 canvas의 실제 원점이 바뀔 때만 절대 경로를 생성해 캐시하고 같은 원점의 재렌더링에는 재사용한다. 모든 참조의 전체 이력을 자식 우선 위상 순서와 가능한 최신 시각 순서로 조회한다. src/ui.rs는 패널 전체 폭에서 공통 열 배열로 헤더와 가상 목록을 정렬하고 History 상세 항목은 세로 파일 스크롤 바깥에 둔다. Project의 상세 선택 불리언과 selected_file은 상호 배타적이며 상세 선택 시 diff_generation을 증가시켜 대기 결과를 무효화한다. 커밋 클릭은 History의 상세를 기본 선택하고 Snapshot의 상세로 렌더링한다. 단위 레인 계약과 실제 저장소 상세 통합 검증을 수행한다. 기존 확정 헌법이 없으며 한글 문서·speckit 절차를 준수한다.

## 전체 이력·가상 스크롤·열 조절 설계

기존 계획의 선택 참조별 200개·고정 합계 폭 설계를 대체한다. src/git.rs는 모든 커밋 참조와 HEAD를 시작점으로 단일 rev_walk하여 중복 없는 전체 이력을 읽고 브랜치 끝 ID를 기록한다. src/model.rs는 Reference.target·Commit.branches·시간대 및 선택 조회 결과를 정의한다. git::load_commit은 전체 조회와 분리하여 파일과 상세만 읽는다.

src/ui.rs는 GPUI uniform_list와 프로젝트별 UniformListScrollHandle을 사용한다. 고정 24px 행을 가시 range에서만 생성하고 RowGraph 캐시도 가시 범위만 유지한다. 전체 조회 완료 시 UI에서 모든 그래프를 생성하던 코드를 제거한다. 브랜치 클릭은 해당 tip으로 스크롤하고 선택 조회만 실행한다. 전체 그래프 계산은 백그라운드에 남기고 화면 렌더에서 그래프 폭 전체 순회를 제거한다.

src/history_view.rs는 canvas로 패널 실제 폭을 측정하고 공통 열 비율을 헤더·가상 행에 적용한다. 열 경계 마우스 다운과 기존 전역 캡처 이벤트로 이웃 두 열 폭을 조절하며 총폭을 보존한다. 최소 폭 합계보다 작은 창에서는 최소 폭을 비례 축소한다. 비율은 창 공통 상태이며 재실행 복원은 제외한다. 시간은 커밋 오프셋을 반영하여 날짜와 HH:MM을 표시한다. 순수 열 계산·날짜 테스트, 실제 다중 참조·200개 초과 통합 테스트, 이력/선택 분리 성능 측정을 수행한다.

전체 이력에서는 같은 시각이나 잘못된 시각의 부모가 자식 위에 나타나는 것을 방지하기 위해 src/graph.rs에서 자식 우선 위상 정렬을 수행한다. 처리 가능한 노드 중 최신 시각을 우선하며 모든 작업은 백그라운드 전체 조회에서만 수행한다.

## 공통 스크롤과 점진 조회 설계

이전 전체 모델 일괄 로드를 UI에서 제거한다. src/paging.rs는 프로젝트 세션별 작업 스레드에서 gix 위상/날짜 iterator를 유지하고 요청 채널에 따라 200개씩 디코딩한다. 처음부터 skip하거나 전체 Commit 모델을 미리 만들지 않는다. 위상 탐색은 관계 판단에 필요한 객체 헤더를 추가로 탐색할 수 있다. src/graph.rs는 페이지 간 레인 상태를 유지하는 GraphBuilder를 제공한다. 초기 Snapshot은 세션·has_more·참조별 인덱스를 가지며 페이지 조회는 선택 조회와 별도 세대/진행 상태를 사용한다. 탭 제거로 채널을 해제하여 작업자를 종료한다. 기존 git::load 전체 조회는 테스트/프로파일 호환 함수로 유지하고 UI만 load_initial을 사용한다.

src/scrolling.rs에 공통 손잡이 수치 계산을 두고 src/ui.rs의 공통 프레임은 UniformListScrollHandle의 base_handle과 실제 트랙 Bounds로 세로·가로 스크롤을 연결한다. 전역 캡처 이벤트로 트랙 밖 드래그와 해제를 처리한다. 바 공간을 확보하여 콘텐츠를 침범하지 않는다. 리스트 하단 판정은 가시 범위 콜백에서 수행하고 비동기 시작을 다음 업데이트로 예약한다. 페이지별 pending/끝/오류를 구분하고 실패는 재시도 버튼으로 처리한다.

브랜치는 조회 때 만든 종류별 참조 인덱스를, 파일은 Stage 그룹 행 매핑을 사용하여 가시 범위만 생성한다. Diff 행과 커밋상세 캐시도 uniform_list에 연결하고 긴 줄의 가로 폭을 측정할 대표 행을 선택한다. Graph 열은 공통 px_2를 적용하고 현재 브랜치는 Snapshot의 실제 head 전체 참조와 비교한다. 한글 Markdown 및 전체 speckit 절차를 따르며 헌법은 미작성 템플릿이다.

## 조건부 스크롤과 헤더 제거 설계

src/scrolling.rs에 콘텐츠 크기와 스크롤 프레임의 실제 Bounds로 필요한 축을 구하는 순수 계산을 둔다. 바가 없는 크기에서 시작해 두 축 영향을 계산하여 이전 프레임 상태에 따른 진동을 방지한다. src/ui.rs의 프레임에 count×행 높이와 고유 콘텐츠 폭을 전달하고 canvas로 외곽 뷰포트를 측정한다. 필요한 바만 레이아웃/히트박스에 포함하고 사라진 축의 드래그를 종료한다. 히스토리 컬럼 폭과 우측 헤더 여백도 실제 세로 바 유무에 맞춘다. Diff 폭은 최초 표시 시 글꼴로 측정해 캐시하여 과대 추정에 의한 불필요한 가로 바를 방지한다.

헤더 함수와 네 영역의 호출을 삭제한다. 페이지 로딩은 히스토리 그리드 안에 애니메이션 진행 오버레이로 표시한다. 브랜치·파일·고정 상세는 ROW_HEIGHT와 w_full을 공유하고 버튼의 세로 padding을 제거한다. detail_rows는 상위 해시를 라벨과 같은 행에 배치하며 시간은 오프셋 반영 후 초까지 표시하고 시간대 접미사만 제거한다. 상세 포맷의 회귀 검증과 스크롤 수치 검증을 수행한다. 한글 문서와 전체 speckit 절차를 따르며 헌법은 미작성 템플릿이다.

## 날짜·해시 기본 고정폭 설계

src/history_view.rs의 기본 가중치를 앞 세 열에만 배분한다. Date 최소 폭은 136px, Hash는 76px로 지정한다. src/ui.rs의 두 데이터 셀은 기존 Diff와 같은 Cascadia Mono를 사용한다. 12px 글꼴에서 16자/8자와 좌우 여백·경계선을 포함한다. 수동 경계 드래그는 기존 가중치 조절을 유지하여 명시적 사용자 조절만 고정 기본값을 변경한다. 기존 최소 폭 축소와 헤더·행 공통 너비를 유지한다. 헌법은 미작성 템플릿이며 한글 문서와 전체 절차를 준수한다. 기존 레이아웃 테스트에 창 확대 시 기본 고정폭 검증을 추가하고 fmt/check/clippy/test로 확인한다.

## 세션·최근 저장소·도움말·진행 표시 설계

src/session.rs에서 serde JSON으로 탭 경로(빈 탭은 null)·활성 인덱스·최근 경로를 저장한다. LOCALAPPDATA/FeatherGit/workspace.json을 사용하고 작은 상태만 동기적으로 안전한 임시 파일 교체로 저장한다. 읽기 오류는 복구 UI에 알리고 기존 파일은 보존한다. 복구 검증은 백그라운드에서 한 번 수행해 순서를 유지한다. 동일 정규 경로는 중복 제거한다. src/ui.rs에서 탭 추가·닫기·선택·열기 및 최근 제거 시 저장한다.

src/setup.rs에 숨긴 Git fetch --all 명령과 저장소 삭제 대상 검증을 추가한다. 삭제는 저장소 루트 일치·일반 .git 폴더·루트 재분석 지점 여부를 검사하고 Windows 휴지통 API로 처리한다. 실패 시 영구 삭제로 전환하지 않는다. 명령과 경로는 셸 문자열에 삽입하지 않는다. UI는 삭제 확인·진행·실패 및 성공 시 탭/최근 제거를 담당한다.

삭제 직전 src/paging.rs의 종료 요청/응답으로 해당 저장소의 페이지 작업자가 파일 핸들을 해제한 것을 확인한다. 실패하면 열린 탭은 유지하며 재조회로 세션을 복구한다. 휴지통 COM 작업은 전용 스레드에서 실행해 작업 풀의 COM 초기화 상태와 분리한다.

src/ui.rs의 공통 최상위 렌더 래퍼는 빈 탭과 저장소 화면 모두 동일한 도움말/삭제 확인/Progress 오버레이를 사용한다. 전역 키 이벤트는 캡처 단계에서 Progress일 때 차단하고 입력 포커스에서는 문자 단축키를 무시한다. 일반 작업 중복 방지용 busy와 모달 Progress를 분리한다. 모든 프로젝트의 history_loading/page_loading만 모달 진행 기준으로 사용한다. 오버레이는 배경 클릭·휠·드래그를 가리고 단축키도 막는다. 작업 결과는 프로젝트 ID와 기존 세대 검증을 유지하며 탭이 사라진 결과를 버린다. fetch 후 요청한 프로젝트를 재조회하고 탭 선택은 보존한다.

src/main.rs 기본 높이는 790px로 줄인다. session 표시 함수는 드라이브와 UNC 접두사만 화면에서 변환한다. 테스트는 임시 상태 파일 왕복/오류, 경로 표시, 삭제 대상 거절, 로컬 원격 fetch 및 작업 트리 보존을 검증한다. cargo fmt/check/clippy/test와 수렴 점검을 수행한다. 기존 사용자 변경을 보존하며 헌법은 미작성 템플릿이다.

키보드 목록 탐색: src/ui.rs에 포커스한 목록 종류와 프로젝트 ID를 보관한다. 공통 루트 FocusHandle을 행 클릭 시 포커스하며 URL 포커스는 별도로 유지한다. ↑/↓는 현재 선택에서 범위 내 다음 인덱스를 계산하고 기존 선택 메서드를 호출한다. 파일은 실제 파일 인덱스로 이동해 그룹을 건너뛰며 스크롤 위치만 표시 행으로 매핑한다. 모달과 진행 상태가 키 처리를 우선 차단한다. 공통 인덱스 경계를 단위 검증한다.

읽기 전용 선택: src/text_selection.rs에 행·UTF-8 바이트 위치의 앵커/커서와 범위·복사·이동을 정의한다. src/ui.rs의 가상 텍스트 행은 실제 shape_line의 위치 계산으로 클릭과 하이라이트·커서를 그린다. 실제 원문을 복사하며 탭은 표시 좌표만 펼친다. 마우스 누름/이동으로 선택하고 공통 키 처리에서 Ctrl+A/C 및 Shift 방향키를 지원한다. 선택 시작 후 파일·커밋 변경 시 초기화한다. 텍스트 입력 편집 기능은 제공하지 않는다.

## 히스토리 진행 표시와 브랜치 탐색 설계

src/model.rs의 Project에 history_loading과 history_target을 추가한다. 기존 loading은 커밋상세 조회에도 사용하므로 Progress 기준으로 재사용하지 않는다. 전체 이력을 다시 읽는 reload에서만 history_loading을 켜고 해당 세대의 성공·실패에서 해제한다. 키·마우스·드래그·텍스트 선택 차단은 히스토리 Progress 기준을 공유하고 일반 busy는 중복 fetch/삭제 방지에 사용한다.

브랜치 클릭과 키보드 선택을 src/ui.rs의 공통 select_reference로 모은다. 상세 선택 후 대상 ID를 저장하고 현재 모델에서 탐색한다. 없으면 기존 load_more로 200개씩 순차 조회한다. 각 성공 페이지를 즉시 누적하여 오류 전까지 진행을 보존한다. Project의 대상 해석 메서드는 발견 인덱스를 반환하거나 끝에서 명시적 오류를 만든다. 미발견 대상은 다음 페이지로 이어진다. 발견 시 요청한 프로젝트가 활성 상태일 때만 가운데로 이동한다. page_generation 보호와 중복 요청 차단을 유지하며 재조회/새 선택 때 대상을 초기화한다.

도움말 제목은 `[단축키]`, 본문은 `f - 패치`와 `F5 - 새로고침`을 둔다. Esc와 바깥 클릭 처리는 유지한다. tests/repository.rs에서 실제 651개 이력과 오래된 브랜치를 구성해 대상 해석과 기존 페이지 조회를 검증한다. fmt/check/clippy 및 단위·통합 검증을 수행한다. 헌법은 미작성 템플릿이며 한글 문서와 전체 speckit 절차를 준수한다.


## 원격 별칭·Stage 스테이징·F5 설계

src/git.rs에서 원본 참조를 먼저 수집해 iterator의 packed 참조 버퍼를 해제하고, 각 참조의 이름과 전체 경로를 저장한 뒤 커밋 대상으로 peel한다. 심볼릭 참조의 이름 변경이 UI 데이터로 전파되지 않게 한다. packed refs·annotated tag 회귀를 검증한다.

src/model.rs의 file_rows는 staged 그룹 헤더만 만들고 FileChange의 Stage 전용 상태 표시 메서드로 두 칸 문자열을 반환한다. History의 단일 상태는 유지한다. src/ui.rs에서 두 칸 고정폭 상태와 staged 파일명에 색상을 적용한다. 같은 파일의 staged/unstaged 행은 유지한다.

src/setup.rs에 경로를 검증하고 셸 없이 `git --literal-pathspecs add --all -- <선택 경로>`를 실행하는 함수를 추가한다. 절대 경로·상위 탈출·빈 경로는 거절한다. src/git.rs에 히스토리를 읽지 않는 작업 상태 조회를 공개한다. UI는 저장소별 staging 작업 집합으로 중복을 막고 백그라운드에서 스테이징 후 상태를 읽는다. 완료 시 요청한 프로젝트의 Stage 행을 갱신하고 현재 선택 경로를 최신 목록에서 다시 찾아 가능하면 같은 staged 행을 선택한다. 현재 화면이 Stage일 때만 선택 Diff를 갱신하고 페이지/그래프/히스토리 스크롤은 유지한다. 닫힌 탭은 갱신하지 않는다. 실패 시 오류를 표시하고 진행 상태를 해제한다.

F5는 공통 키 처리에서 활성 저장소의 기존 reload를 호출한다. 원격 통신을 하지 않으며 히스토리 Progress 기준을 재사용한다. 도움말은 `[단축키]`, `f - 패치`, `F5 - 새로고침`만 표시한다. 스테이징은 입력/텍스트/모달/다른 목록에서 실행하지 않는다. 실제 검증은 임시 저장소에서만 수행한다. 헌법은 미작성 템플릿이며 한글 문서와 전체 speckit 절차를 준수한다.


## Stage 헤더 제거와 양방향 Space 설계

src/ui.rs의 Stage 초기 조회와 인덱스 작업 후 갱신 모두 file_rows의 그룹 없는 매핑을 사용한다. History 매핑과 공통 행 모델은 유지한다. src/setup.rs는 공통 경로 검증과 literal pathspec을 유지하면서 stage는 add, unstage는 HEAD가 있으면 경로 한정 reset HEAD, unborn이면 rm --cached --force를 사용한다. 작업 트리는 변경하지 않는다. CLI 인자를 셸로 합치지 않는다.

UI는 시작 행의 staged 여부를 캡처해 반대 작업을 실행하고, 완료 시 해당 경로의 반대 staged 여부를 우선해 선택을 복원한다. 비동기 중 사용자가 다른 파일을 선택했다면 그 선택을 유지한다. 실제 변경이 없어져 행이 사라지면 기존 선택/Diff 해제 처리를 사용한다. 임시 저장소에서 여섯 상태 왕복·부분 staged·unborn·literal 경로를 검증하고 fmt/check/clippy/단위·통합 검사 및 수렴 점검을 수행한다. 헌법은 미작성 템플릿이며 한글 문서와 전체 speckit 절차를 준수한다.


## 안정 정렬·지연 탭·커밋 메시지 설계

src/git.rs의 상태 정렬을 경로 우선, 같은 경로에서는 staged 우선으로 바꾼다. src/session.rs는 저장된 비활성 경로를 저장소 탐색 없이 보존하고 활성 경로만 검증한다. 실행 인자도 경로를 합친 뒤 마지막 활성 대상만 검증한다. src/model.rs의 Project에 최초 조회 여부를 추가하고 src/ui.rs의 탭 활성화/닫기와 복구 경로에서 미조회 활성 탭만 reload한다. History 고정 커밋상세 행에 파일 행과 같은 text_sm을 명시한다.

src/commit_editor.rs에 GPUI EntityInputHandler 기반 여러 줄 입력기를 추가하여 실제 IME 입력과 UTF-16 범위 변환을 지원한다. UTF-8 원문·선택/조합 범위·행별 shape_line 위치를 유지하고 커서·선택·스크롤·클립보드 편집을 제공한다. src/ui.rs는 커밋 대상 프로젝트 ID·입력 Entity·실행/오류 상태를 가진 모달을 관리한다. 공통 키 캡처에서 c로 열고 편집 중에는 입력기에 전달하며 Ctrl+Enter/Ctrl+S만 제출한다. 실행 중에는 입력·취소를 막고 완료 후 실패 메시지를 보존하거나 성공 시 해당 탭을 reload한다. 도움말에 c를 추가하고 모달 하단에 실행 키를 표시한다.

src/setup.rs의 commit은 메시지를 stdin으로 Git commit --file=-에 전달하고 --only/-a/--allow-empty를 사용하지 않아 현재 인덱스만 커밋한다. 공백 메시지와 NUL을 거절하고 Git 오류를 반환한다. Git의 사용자 설정·훅·서명을 따르되 터미널 입력/에디터 창은 비활성화한다. 검증은 임시 저장소에서만 실행한다. fmt/check/clippy/단위·통합 및 수렴 검사를 수행한다. 헌법은 미작성 템플릿이며 한글 문서와 전체 절차를 준수한다.


## 커밋 팝업 조건·버튼·메시지 스크롤 설계

src/ui.rs의 open_commit/submit_commit에 staged 변경 존재 조건을 넣고 src/setup.rs에서도 diff --cached --quiet로 비어 있는 인덱스의 실제 commit 호출을 막는다. 커밋/도움말 제목은 text_sm으로 줄인다. 제출 키는 Ctrl+S만 받고 하단은 소문자 안내와 우측 커밋·닫기 버튼을 배치한다. 버튼은 기존 submit_commit과 공통 취소 조건을 사용하며 실행 중 비활성 상태를 표시한다.

src/commit_editor.rs의 기존 입력 스크롤을 명시적 조건부 두 축 스크롤바로 보완한다. 실제 shape_line 폭과 행 수/22px 높이를 콘텐츠 크기로 사용하고 공통 scrolling::Visibility/Thumb으로 바 공간과 손잡이를 계산한다. 가시 영역의 ScrollHandle과 트랙 클릭/드래그를 연결한다. 입력 높이는 220px로 고정하고 제목/하단은 스크롤 바깥에 둔다. 기존 커서 자동 스크롤과 IME를 유지한다. 빈 메시지에는 입력 폭보다 작은 최소 캔버스로 불필요한 가로 스크롤을 없앤다. 기존 단위/실제 저장소 검사와 fmt/check/clippy 및 수렴 점검을 수행한다. 헌법은 미작성 템플릿이다.


## 작성자 메일과 Stage 다중 선택 설계

src/model.rs의 Commit에 author_email과 표시 메서드를 추가하고 src/paging.rs에서 작성자 시그니처의 이름/메일을 각각 읽는다. src/ui.rs Author 셀은 표시 메서드를 사용한다. 초기/추가 페이지가 동일 경로를 사용한다.

StageSelection은 파일 경로+staged 상태의 키 집합과 범위 기준 키를 가진다. UI 클릭/Shift 방향키는 이 모델로 선택을 변경하며 Diff용 selected_file은 대표 행으로 유지한다. 렌더는 집합에 포함된 모든 Stage 행을 강조한다. 새 조회/파일 탭 변경에서는 초기화하고 상태 전환 후 실제 목록과 목표 상태에 따라 키와 기준을 복원한다.

src/setup.rs에 경로 사전 검증·중복 통합·상태별 일괄 인덱스 변경을 추가한다. 같은 경로의 상반 요청은 stage 우선이다. 기존 단일 경로 함수도 공통 다중 경로 실행을 사용한다. UI는 요청 대상 키를 캡처하고 실행 후 성공/오류 모두 최신 상태를 읽으며 선택을 갱신한다. 적용 중 다른 선택으로 바꾸면 현재 선택을 기준으로 복원하고 작업 오류는 선택 Diff 조회가 끝난 후에도 보이는 상태 위치에 알린다. 신규 입력/History의 Space는 기존 포커스 제한을 유지한다. 임시 저장소와 선택 모델 검증 및 fmt/check/clippy·수렴 점검을 수행한다. 헌법은 미작성 템플릿이다.

## 선택 파일 폐기 설계

src/setup.rs에서 전체 경로를 사전 검증하고 HEAD의 항목 존재 여부로 tracked/new를 구분한다. HEAD 항목은 git restore --source=HEAD --staged --worktree로 해당 경로만 복원한다. 새 경로는 literal pathspec의 rm --cached 후 정확한 파일만 remove_file하며 재귀 삭제는 사용하지 않는다. 중간 경로의 symlink/reparse point와 디렉터리/서브모듈은 거절한다. src/ui.rs는 프로젝트 ID·고유 선택 경로·실행 상태를 가진 확인 모달로 범위를 알리고 확인 후에만 실행한다. 실패/성공 모두 작업 상태를 다시 읽고 공통 선택 복원으로 갱신한다. 도움말에 d를 추가하며 입력/History/다른 모달에서는 실행하지 않는다. 실제 검증은 임시 저장소에만 수행한다.

## 풀·푸시 설계

src/setup.rs의 원격 작업 열거형에 Fetch/Pull/Push를 두고 기존 숨긴 Git 명령으로 fetch --all, pull --ff-only, push를 실행한다. src/ui.rs는 공통 원격 작업 메서드로 중복 방지·작업 라벨·오류·성공 재조회를 처리한다. p/Shift+p는 일반 키 처리에서만 인식하고 입력/모달에서 차단한다. 상태 표시줄에 진행 라벨을 두며 히스토리 재조회 구간만 Progress를 사용한다. 실제 검증은 임시 로컬 원격에서만 수행한다.

커밋 제출 메서드는 push_after 플래그를 받는다. Ctrl+S는 false, Ctrl+P는 true이며 버튼은 제거한다. 백그라운드에서 commit 성공 후에만 push를 실행하고 push 오류는 commit 오류와 분리해 반환한다. 로컬 커밋이 만들어졌으면 오버레이를 닫고 재조회하며 push 실패는 하단 오류로 표시한다. 실행 중에는 입력/중복/취소를 계속 차단한다.


## 간결한 폐기 확인과 Ctrl+A 설계

src/ui.rs에서 폐기 모달의 파일 목록과 버튼을 제거하고 개수/범위 문구 아래 소문자 키 안내를 표시한다. 공통 키 처리의 discard_dialog 분기에서 실행 중이 아닐 때 Ctrl+S를 기존 discard_selected에 연결하고 Esc 취소를 유지한다. 모달 처리 우선순위를 유지해 커밋 창/일반 화면의 Ctrl+S와 격리한다.

src/model.rs의 StageSelection에 전체 선택 메서드를 추가하고 src/ui.rs의 Stage 파일 포커스에서 Ctrl+A로 호출한다. 현재 대표 selected_file이 있으면 Diff를 재조회하지 않고 유지하며 선택이 없고 파일이 있으면 첫 파일을 대표로 조회한다. 기존 text/commit/URL 처리 뒤에 배치하여 편집 동작을 유지한다. 선택 모델의 빈 목록/전체/개별 해제/복원 검증과 fmt/check/clippy/단위·통합 및 수렴 검사를 수행한다. 헌법은 미작성 템플릿이다.

## 하단 오류 수명 설계

src/ui.rs에서 현재 원본 오류와 마지막 관찰 오류, 일시 표시 문자열, 표시 세대를 분리한다. 새 하단 오류를 관찰하면 GPUI 백그라운드 타이머로 3초 후 해당 세대의 표시만 지운다. 원본 self.error/Project.error는 유지해 패널 실패 처리를 보존한다. 일반 상태와 오류 색상, 빈 탭 하단 표시 모두 일시 문자열만 사용한다. 모달 오류는 이 타이머와 독립이다.

## 섹션 포커스와 단축키 설계

기존 Rust/GPUI와 ListFocus를 재사용한다. src/ui.rs의 행 생성 함수에 Window를 전달하고 공통 선택색 함수로 활성 저장소·FocusHandle·모달/진행 상태·ListFocus 일치를 검사한다. 선택 행 호버는 색을 덮지 않는다. UI 상태만 계산하며 영속 데이터와 의존성을 추가하지 않는다. 브랜치와 이력과 파일에 순차 적용한다.
도움말은 적용 범위별 묶음으로 표시한다. 새로고침 조건은 입력/모달 보호 뒤에서 f5와 ctrl+r를 공통 처리한다. fmt/check/clippy 및 기존 테스트로 검증한다. 헌법은 미작성 템플릿이므로 추가 강제 조항은 없다. 기존 변경은 보존한다.

## 새 탭 좌우 배치와 최근 목록 설계

src/ui.rs의 StartPage에 UniformListScrollHandle을 추가해 탭별 최근 목록 위치를 유지한다. 왼쪽은 너비 300px 내외의 기존 작업/복제 폼, 오른쪽은 flex_1/min_w_0/min_h_0를 사용하는 최근 저장소 패널로 구성한다. 고정 높이 40px인 uniform_list 프로세서에서 요청된 범위만 recent_row로 생성한다. 기존 scroll_frame의 40번 키와 수직 스크롤바를 재사용한다. 경로 텍스트는 한 줄 말줄임하고 삭제 아이콘은 줄어들지 않게 한다. 별도 의존성 없이 canvas로 휴지통 아이콘을 그린다.

삭제 선택 상태는 기존 delete_target과 신규 delete_confirm 불리언으로 나타낸다. 아이콘은 대상과 false를 설정해 선택창을 열고, 목록 제거는 최근 기록만 필터링/저장한다. 삭제 선택은 true로 전환해 기존 확인창을 표시하며 실제 delete_repository는 true일 때만 실행 가능하다. 기존 모달 차단·취소·삭제 완료·오류 복구를 재사용한다. 실제 사용자 저장소의 삭제는 검증 중 실행하지 않는다.

검증은 fmt/check/clippy, 단위와 기존 저장소 통합 테스트 및 코드상 렌더 범위·이벤트 전파·최종 확인 연결 점검이다. 현재 실행 앱의 exe 잠금이 있으면 통합 테스트 실행파일을 별도 생성 경로로 빌드하거나 최신 생성된 테스트 파일을 실행한다. 헌법은 미작성 템플릿이며 새 의존성·외부 인터페이스·영속 형식 변경은 없다.

## History 커밋상세 행 색 보정 설계

src/ui.rs의 files_panel에 Window 참조를 전달한다. 커밋상세 행은 공통 button 대신 일반 파일 행과 같은 div 구조를 사용하고 detail 선택 시 selection_color(ListFocus::Files(id), window)를 적용한다. 선택된 행의 호버를 제거하고 미선택 행만 일반 호버를 적용한다. 클릭의 포커스·show_detail·notify는 유지한다. 새 의존성·데이터 변경은 없다. 헌법은 미작성 템플릿이다. fmt/check/clippy와 기존 단위 테스트 및 수렴 점검을 수행한다.

## 빈 탭 저장소 전환 설계

src/ui.rs의 WorkspaceTab에 순수한 빈 탭 전환 함수를 두고 시작 ID가 빈 탭일 때만 처리한다. 미개방 경로는 같은 ID/인덱스에서 Project로 대체하며 이미 열린 경로는 시작 빈 탭만 제거하고 기존 프로젝트 ID를 반환한다. 두 분기와 다른 빈 탭 유지·이미 닫힘을 단위 테스트한다.

Workspace의 공통 완료 메서드는 시작 페이지와 최근 목록 스크롤 캐시를 정리하고 최근 기록을 갱신한다. 완료 시 시작 탭이 활성인 경우에만 결과 탭으로 활성 선택을 옮긴다. 조회를 시작할 때 잠시 결과 탭을 활성화하고 현재 선택을 복원하는 기존 방식을 재사용한다. 최근 저장소 open은 시작 ID를 요청 시 캡처하고 StartPage.busy로 중복 요청을 막으며 실패는 해당 시작 페이지에 표시한다. 폴더 생성·열기·복제 성공도 공통 완료를 호출한다. 시작 페이지가 사라지면 완료를 무시하고 취소·실패는 빈 탭을 유지한다.

Rust/GPUI 기존 구조와 세션 저장을 재사용하고 의존성·영속 형식 변경은 없다. 미작성 헌법은 템플릿으로 처리한다. 회귀 테스트를 먼저 추가한 뒤 구현하고 fmt/check/clippy 및 단위 테스트로 확인한다.

## 도움말 다섯 섹션 설계

src/ui.rs에서 도움말 전용 렌더 함수를 만들고 섹션 이름과 키별 문자열을 정적 목록으로 구성한다. 다섯 섹션을 순서대로 렌더하며 공통 키와 Stage 파일목록 키, 텍스트뷰어 키를 한 줄씩 표시한다. 브랜치/커밋히스토리는 제외 요청 후 전용 키가 없어 빈 안내를 쓴다. 확인창 전용과 기존 diff/커밋상세·행 이동 문구는 제거한다. 헤더와 섹션 간 간격은 유지하고 도움말 목록은 뷰포트 높이에서 여백을 뺀 최대 높이와 세로 스크롤을 사용한다. 실제 key_down 처리와 다른 모달은 유지한다. Rust/GPUI 기존 구성과 상태를 재사용하며 새 데이터·의존성은 없다. 헌법은 미작성 템플릿이다. fmt/check/clippy와 표시 문자열·키 분기 대조로 검증하며 단순 표시 변경의 중복 테스트는 추가하지 않는다.

## 설정 오버레이 구현 계획

Rust/GPUI를 유지하며 src/settings.rs에 설정 전용 엔티티를 추가하고 Workspace는 Option<Entity<SettingsPanel>>로 오버레이를 소유한다. 사용자/원격 탭, 고정 높이 입력과 스크롤 본문, 저장·닫기와 진행/오류 메시지를 제공한다. 기존 CommitEditor에 단일 행 생성·값 설정을 추가해 IME·선택·클립보드를 재사용하며 기존 커밋 메시지 모드는 유지한다. 설정 엔티티는 열 때 저장소 경로를 캡처하고 비동기 결과는 세대/작업 상태로 관리한다. 닫기는 취소 플래그를 설정한다.

src/settings_git.rs는 Git 설정·원격 조회 모델과 GitContext를 제공한다. git config의 --global/--local 명시 범위로 이름·이메일을 조회·쓰기하고 전역 사용 시 현재 저장소의 직접 user.name/user.email을 제거한다. 저장 전 관련 키를 스냅샷으로 읽고 실패 시 복구하며 반영 후 유효값을 확인한다. 새 탭에서는 로컬 쓰기를 거절한다. 임시 global 파일과 시스템 설정 격리를 지원하는 테스트 컨텍스트를 두어 실제 사용자 설정을 바꾸지 않는다. git remote get-url --all과 --push로 원격별 URL을, 브랜치와 추적 설정을 읽는다. URL은 url 2.5.8(기존 잠금/캐시에 존재)을 직접 의존성으로 사용해 사용자정보·쿼리·프래그먼트를 숨기며 로그인 입력은 비밀 없는 HTTPS 주소만 허용한다.

src/credentials.rs는 설치된 git credential-manager 또는 credential-manager-core를 탐지하고 표준 get/store/erase 프로토콜로 로그인/상태/로그아웃을 수행한다. Git 연결은 명시적 configure 버튼에서만 실행한다. 브라우저 요청은 GitHub/GitLab browser, Bitbucket oauth, Azure system 모드를 실행 환경에만 설정한다. 상태 조회는 비대화식, 로그인은 GUI 대화식이다. stdout 자격 증명은 UI에 반환하지 않고 계정명만 추출해 GCM store로 넘긴 후 버퍼를 지운다. 인증 오류는 원문 출력 대신 비밀 없는 작업별 안내를 사용한다. 자식 프로세스 stdout/stderr를 별도 스레드에서 읽어 교착을 피하고 취소·시간 제한으로 종료한다. 테스트에서는 실행기 대역으로 요청·응답·취소와 비밀 비노출을 검증한다. 실제 로그인·전역 사용자 설정 변경은 개발 검증에서 실행하지 않는다.

src/ui.rs는 두 하단 영역의 도움말 옆에 f12 세팅 진입을 추가하고 일반 f12 키를 연결한다. 설정 열림 상태를 모달·포커스 색·리사이즈 차단에 포함한다. 설정 중 키는 입력으로 전달하되 esc/ctrl+s를 설정 전용으로 처리하며 기존 저장소 키는 실행하지 않는다. 기존 커밋/폐기 모달을 덮지 않는다. 도움말 공통에도 f12를 한 줄 추가한다.

검증: 격리 Git 설정/원격 테스트, 가짜 GCM 프로토콜 테스트, 입력 정규화 회귀, fmt/check/clippy 및 기존 단위·통합 테스트. UI 시각 검증 가능 여부는 결과에 명시한다. 헌법은 미작성 템플릿이며 외부 API 서버나 자체 OAuth 앱은 추가하지 않는다.

설정 구현 보완: GCM의 get/store/erase는 Git credential fill/approve/reject에 선택한 helper만 명시해 위임한다. Git이 credential.useHttpPath와 URL별 설정을 적용하므로 기존 fetch/push와 같은 자격 증명 범위를 사용한다. Git 연결(configure) 중에는 사용자 설정 저장과 같이 닫기를 잠시 막고 브라우저 인증 중 닫기는 프로세스 트리 취소로 처리한다.

## 브랜치 생성·전환 및 태그 설계

src/branches.rs에 숨긴 Git 명령 기반의 이름 검증·커밋 검증·생성·체크아웃 준비/실행을 둔다. check-ref-format과 refs/heads/<이름>의 정확한 조회를 사용하고 커밋은 객체 ID 형식과 rev-parse --verify <id>^{commit}로 확인한다. 생성은 git branch --no-track -- <이름> <커밋>으로 현재 HEAD를 유지한다. 체크아웃은 git switch --no-guess -- <이름>, 원격 추적 신규 생성은 git switch --track -c <이름> -- <원격참조>를 사용하며 force/reset 옵션은 사용하지 않는다. 원격 HEAD는 symbolic-ref로 해석하고 등록 remote 중 가장 긴 접두어를 찾아 로컬 후보 이름을 결정한다. 기존 로컬은 이름으로 정확히 확인하고 원격 생성은 UI의 확인 이후에만 실행한다. 사용자 저장소에는 개발 중 브랜치를 만들거나 체크아웃하지 않는다.

src/ui.rs에 생성/원격추적확인 BranchDialog와 비동기 작업 상태를 추가한다. 생성 이름은 기존 단일 행 CommitEditor를 사용한다. b는 History 포커스와 선택 커밋을 확인하고 다른 모달·입력에서는 실행하지 않는다. 확인 버튼/enter/ctrl+s와 esc 취소, IME 조합 보호·중복 차단·오류 유지·진행을 제공한다. Local/Remote의 click_count==2에 전환을 연결하고 Stash/Tag의 기존 선택은 유지한다. 첫 클릭 조회에서 생긴 단순 메타데이터 로딩은 더블클릭을 차단하지 않는다. 변경 명령 이후 성공/실패 모두 세대 갱신으로 최신 상태를 재조회하며 성공 체크아웃은 현재 브랜치/Stage 화면으로 전환한다. b 생성은 기존 선택/HEAD를 유지한다. 도움말의 커밋히스토리 섹션에 b 한 줄을 추가한다.

src/model.rs의 Commit에 tags 목록을 추가하고 src/paging.rs에서 기존 references의 Tag 종류를 커밋 ID별로 모아 모든 페이지의 Commit에 전달한다. 기존 gix 태그 peel 로직이 annotated/lightweight 모두 실제 커밋을 제공한다. src/ui.rs는 브랜치 배지 뒤에 보라색 태그 배지를 렌더한다. 기존 제목·브랜치 표식·그래프와 페이지 순서는 유지한다.

검증은 임시 저장소 생성/전환/원격추적/충돌 회귀 테스트와 두 페이지의 annotated/lightweight 태그 테스트를 먼저 작성한다. fmt/check/clippy와 단위·통합을 실행하고 수렴 점검한다. Rust/GPUI와 Git 기존 기반을 사용하며 새 의존성은 없다. 미작성 헌법은 템플릿으로 처리한다.

브랜치 구현 보완: 첫 클릭에서 대상 커밋이 추가 페이지에 있을 때는 시스템 더블클릭 간격만큼 추가 페이지 모달 시작을 미룬다. 즉시 메타데이터 조회/선택은 유지하고 두 번째 클릭이 오면 대기 중 탐색을 취소해 체크아웃을 진행한다. Windows GetDoubleClickTime을 읽기 위해 기존 windows 의존성의 KeyboardAndMouse 기능만 활성화한다. 또한 재조회에서 선택 커밋의 상세 데이터가 있으면 첫 페이지에 없더라도 선택을 유지한다.

## 전체 변경 스태시 계획

기존 Rust·GPUI·Git CLI를 유지한다. src/setup.rs에서 루트 검증·빈 변경 확인 후 git stash push --include-untracked를 실행한다. 인덱스 부모를 보존하여 --index 복원이 가능하다. 의존성 추가는 없다.
src/ui.rs에 project_id·busy·error 상태와 확인창·s 도움말을 추가한다. 기존 폐기 작업의 비동기 실행·파일/Diff 갱신을 재사용하고 모달 가드에 포함한다.
임시 저장소에서 전체 저장·ignored 보존·빈 상태·--index 왕복을 검증한다. fmt/check/clippy·단위 테스트·수렴 점검을 수행한다. 헌법은 템플릿이므로 규범 점검은 생략한다.

## 스태시 목록 갱신·복원 계획

저장 성공 후 src/ui.rs의 저장소 ID 기반 refresh_after_branch 재조회를 호출하여 참조와 목록을 갱신한다. 선택 탭은 유지한다. Stash 더블클릭은 전용 복원 메서드로 라우팅한다. 첫 클릭의 지연 페이지 탐색은 복원 시 취소한다.
src/setup.rs에서 선택 커밋 해시를 검증하고 stash 목록에 존재하는지 확인 후 git stash apply --index를 실행한다. apply로 성공·실패 모두 원본을 남긴다. 기존 branch_busy와 pending을 사용해 작업을 격리한다. 완료 후 원래 저장소를 재조회하고 성공 시 Stage 탭을 표시한다.
임시 저장소에서 복수 항목·선택 해시·인덱스 복원·목록 보존·충돌·잘못된 대상 거절을 검증한다. fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## 이름 입력과 스태시 삭제 계획

src/ui.rs의 StashDialog를 저장/삭제 공용 상태로 확장한다. 저장은 기존 CommitEditor 단일 행 입력과 IME 보호를 재사용하고 삭제는 selector·해시·이름을 캡처한다. 기존 모달 가드를 그대로 활용한다. d는 브랜치 Stash 포커스일 때 삭제 확인을 우선하고 파일 목록에서는 기존 폐기를 유지한다.
src/setup.rs의 stash_all에 이름 인수를 추가하고 --message 인수로 전달한다. 삭제 함수는 selector 형식과 커밋 해시를 검증하고 현재 selector 해시 일치를 확인한 뒤 git stash drop을 수행한다. 상태 실패는 확인창에 표시하고 성공 후 기존 참조 재조회를 사용한다. 삭제는 작업 트리와 인덱스를 수정하지 않는다.
기존 임시 저장소 테스트를 확장해 이름·빈 입력·선택 삭제·다른 항목과 파일 보존·대상 불일치 거절을 검증한다. fmt/check/clippy·단위 테스트와 수렴 점검을 수행한다.

## 참조 작업과 설정 계획

src/branches.rs에 태그 생성과 참조 삭제 함수를 추가한다. full ref와 예상 커밋을 검증하고 Local은 branch -d, Tag는 tag -d로 처리한다. 태그 이름은 check-ref-format refs/tags/로 검증하고 강제 옵션은 사용하지 않는다. Remote의 의미는 명확화 후 확정한다.
src/ui.rs BranchAction에 Tag·Delete를 추가해 기존 이름 입력/확인/IME/진행/오류 처리와 참조 재조회를 재사용한다. t는 History 포커스, d는 모든 참조 목록 포커스를 요구한다. 삭제 확인은 종류와 이름을 표시한다.
src/settings.rs에서 체크 라벨을 고정하고 설명·저장 버튼·GCM 정보 표시를 제거한다. ctrl+s와 인증은 유지한다. 불필요해진 표시용 상태는 정리한다. 기존 Rust·GPUI·Git CLI와 테스트 도구를 사용하며 새 의존성은 없다.
임시 저장소 테스트로 태그 선택/중복·삭제·현재 브랜치 거절·파일 보존을 확인하고 fmt/check/clippy·단위 테스트 후 수렴 점검한다. 헌법은 템플릿이므로 규범 점검은 생략한다.

Remote d는 branch -d -r, Shift+d는 선택 원격과 브랜치를 분리하고 예상 해시의 --force-with-lease로 서버 삭제를 실행한다. 임시 로컬 bare 원격으로 추적 참조 삭제/서버 삭제/변경 대상 거절을 검증한다.

추가 그래프 계획: src/model.rs Snapshot에 head_commit을 추가하고 src/git.rs에서 실제 HEAD 해시를 전달한다. src/ui.rs는 선택 행과 무관하게 해시를 비교하고 src/history_view.rs는 HEAD 노드 중앙을 행 배경으로 가리고 두꺼운 테두리로 그린다. 일반 노드와 경로는 유지한다.

## 입력·원격 생성·커밋 체크아웃 계획

src/commit_editor.rs의 고정 220px 루트 높이를 단일 행일 때 36px 래퍼 안쪽의 26px로 바꾸고 min-height와 클리핑을 명시한다. src/settings.rs와 src/ui.rs의 입력 래퍼에도 클리핑을 추가한다.
src/branches.rs에 선택 Local ref/예상 해시/원격 이름 검증과 신규 생성 lease를 사용하는 push -u를 추가한다. switch --detach는 해시 검증 후 수행한다. src/ui.rs BranchAction::Upload와 원격명 편집 확인창·u 포커스·진행·오류·재조회를 기존 구조에 연결한다. 태그와 커밋 더블클릭은 공통 비동기 detached 체크아웃 메서드를 사용한다.
src/history_view.rs의 HEAD 채움은 흰색, 외곽선은 2px로 바꾸고 불필요한 배경 인수를 제거한다. 임시 원격·체크아웃 충돌 테스트 및 fmt/check/clippy·전체 단위/저장소 통합 테스트 후 수렴 점검한다.

## 브랜치 우선 체크아웃 계획

src/branches.rs에 실제 refs/heads·refs/remotes를 조회하는 후보 구조와 조회/선택 체크아웃 검증 함수를 추가한다. 커밋 해시를 검증하고 심볼릭 ref는 제외한다. 선택 대상과 대응 로컬의 tip을 검사해 다른 커밋으로 전환되지 않도록 한다.
src/ui.rs에서 히스토리 더블클릭은 전용 비동기 준비 경로로 보낸다. 후보 0은 기존 커밋 체크아웃, 1은 검증 후 브랜치 전환 또는 기존 Remote 생성 확인, 복수는 BranchAction::ChooseCheckout으로 선택·확인·취소를 제공한다. 기존 BranchDialog·busy·오류·IME/모달 격리·재조회를 재사용한다. Remote 단일 생성 확인은 선택 커밋 검증을 포함한다. 태그 더블클릭은 기존 경로를 유지한다.
임시 저장소에서 후보 분기·정렬·Remote/심볼릭·선택 전후 tip 변경·같은 이름 Local 불일치·수정 파일 보호를 검증하고 fmt/check/clippy·단위 테스트·수렴 점검을 수행한다.

## 브랜치 선택 간소화 계획

src/branches.rs 후보 조회에서 모든 Local 이름 집합을 만들고 등록 원격의 가장 긴 이름을 접두사로 제거해 동일한 Local 이름이 있는 원격 후보를 필터한다. 선택 커밋과 무관하게 존재 여부로 판단한다.
src/ui.rs의 단일 후보는 allow_create=true로 즉시 체크아웃한다. 선택창 버튼 클릭은 selected를 지정하고 즉시 submit_branch를 호출한다. 선택창만 공통 확인/취소 버튼과 Enter 기본 제출을 제외한다. Esc·바깥 클릭과 기존 다른 모달 동작은 유지한다.
기존 후보 테스트를 먼저 확장하고 fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## 공통 팝업 계획

src/popup.rs에 title·width·body·선택적 footer/header_action·viewport를 받는 공통 프레임을 추가한다. 배경/테두리/여백/13px 제목/본문 스크롤/선택적 하단을 한 곳에서 정의한다. src/main.rs에 모듈을 등록한다.
src/ui.rs의 모든 모달 분기는 본문만 만들고 제목·폭·하단·헤더 동작을 공통 프레임에 전달한다. 설정 닫기는 header_action, 커밋/폐기의 단축키 안내는 footer, 확인 버튼 묶음도 footer로 이동한다. 도움말은 공통 제목을 추가하고 본문 자체의 중복 높이 제한은 제거한다. 기존 backdrop 이벤트와 작업 상태 로직은 유지한다.
가벼운 레이아웃 리팩터링으로 구현을 복제하는 테스트는 추가하지 않는다. fmt/check/clippy·기존 단위 테스트와 전체 팝업 경로의 구조/이벤트 코드 검토 후 수렴 점검한다. 새 의존성은 없다.

## 현재 브랜치 무시 계획

src/branches.rs에 실제 symbolic HEAD 비교 함수를 추가하고 이미 현재 Local이면 switch를 생략한다. src/ui.rs의 Local 더블클릭과 선택 후보 클릭은 snapshot.current_branch 비교로 작업 시작 전에 반환한다. 히스토리 단일 후보의 비동기 경로는 실제 HEAD 비교 결과를 전달해 무변경이면 refresh_after_branch도 호출하지 않는다. 선택창 설명을 제거한다.
기존 체크아웃 테스트에 현재 브랜치/detached 판별을 추가하고 fmt/check/clippy·단위 테스트 및 수렴 점검을 수행한다.

## 라이트 팔레트 구현 계획

src/theme.rs에 배경·패널·경계·글자·보조·강조·선택/hover·상태·Diff·태그·텍스트 선택·그래프 색상을 정의하고 src/main.rs에 등록한다. src/ui.rs의 지역 색상 상수와 하드코딩된 상태 색을 공통 팔레트로 전환한다. src/popup.rs·src/settings.rs·src/commit_editor.rs·src/history_view.rs도 같은 토큰을 사용한다. backdrop 투명도를 라이트 화면에 맞게 낮춘다.
색상 변경은 의미별 매핑으로 한정한다. 실제 실행 플래그 등 비시각적 숫자는 변경하지 않는다. 대비 계산과 기존 어두운 색상 잔존 검색·fmt/check/clippy·단위 테스트 후 수렴 점검한다. 구현을 복제하는 새 테스트는 추가하지 않는다.

## 탭·배지·그래프 계획

src/ui.rs의 탭바에 border를 추가하고 벡터 아이콘 helper로 배지를 통일한다. Remote 판별은 Snapshot references의 종류와 target/name을 사용한다. src/theme.rs에 Remote 배지 색을 정의한다.
src/history_view.rs에 레인 좌표를 열 폭에 맞추는 공통 함수를 추가한다. src/ui.rs는 전체 로드된 커밋/edge에서 최대 레인 수를 한 번 계산해 모든 행에 전달한다. 경로 캐시는 원점뿐 아니라 폭·레인 수도 키로 사용한다. 좌표 경계 테스트를 먼저 추가하고 fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## 히스토리·상세 보완 계획

src/theme.rs의 ACCENT/SELECT를 파랑 계열로 변경하고 대비를 계산한다. src/ui.rs는 태그 루프를 브랜치 앞에 배치하고 브랜치/커밋상세 목록 크기를 12px로 바꾸며 벡터 커밋 아이콘을 추가한다.
src/history_view.rs는 원을 4개 대칭 cubic 경로로 그려 rounded quad 경로를 제거한다. HEAD는 외곽 색 원과 2px 작은 흰 원을 조합하고 일반 노드는 채움만 그린다. cell의 컬럼 선을 제거하고 헤더 호출에서만 선을 추가한다.
src/ui.rs의 상세 텍스트 shaping에 라벨과 값의 TextRun을 분리하고 메타데이터 구간만 굵게 한다. 측정도 같은 run을 사용하며 시간 문자열/기존 테스트를 갱신한다. fmt/check/clippy·단위 테스트·대비 계산 후 수렴 점검한다.

## 강조와 크기 조정 계획

src/theme.rs의 ACCENT는 녹색으로 복구하고 SELECT는 파랑으로 유지하며 LOCAL_BG를 추가한다. src/ui.rs 배지에 테두리와 Local 배경을 적용하고 목록 아이콘을 독립 18px 요소로 분리한다. src/history_view.rs HEAD 반지름을 7.5px로 확대하고 기존 좌표 경계 테스트도 확대 크기로 검증한다. fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## 확대 대상 정정 계획

src/history_view.rs의 HEAD 반지름과 경계 테스트를 5.5px로 복구한다. src/ui.rs 브랜치 배지에서 현재 표시 원을 이름 문자열과 분리해 16px 요소로 그린다. 이름은 11px를 유지한다. 기존 단위 테스트·fmt/check/clippy 및 수렴 점검을 수행한다.

## 원 정렬 계획

src/ui.rs에 글자 줄높이와 독립적인 current_branch_dot canvas helper를 추가한다. 배지는 18px 고정·원 슬롯은 12px, 목록 슬롯은 18px로 설정하고 실제 원은 각각 슬롯 중앙에 10px로 그린다. 이름/그래프/기존 이벤트를 유지한다. fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## 원격 HEAD 표시 필터 계획

src/model.rs Reference에 원격 HEAD 표시 판별을 추가하고 src/git.rs의 reference_rows와 src/paging.rs의 배지 구성에서만 제외한다. 원본 references와 Git 참조는 수정하지 않는다. tests/repository.rs의 기존 원격 HEAD 테스트를 목록/배지 숨김·내부/실제 참조 보존 검증으로 갱신한다. fmt/check/clippy·단위/통합 테스트 후 수렴 점검한다.

## 픽셀 정렬 계획

src/history_view.rs에 physical stroke 두께·중심 스냅 helper를 추가하고 테스트를 먼저 작성한다. 시작/끝 좌표와 노드 중심에 같은 스냅을 사용한다. 수직선은 정렬된 사각형 채움으로, 곡선은 픽셀에 맞춘 stroke로 그린다. DPI를 캐시 키에 추가한다. fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## 렌더 굵기·여백 계획

src/ui.rs의 선택 가능한 행과 폭 측정에 shape_text를 이용한 공통 단일 행 helper를 사용한다. WrappedLine의 unwrapped_layout으로 기존 좌표 조회를 유지하고 paint는 줄바꿈 없는 설정을 사용한다. NORMAL/BOLD를 모두 명시하고 run 검증 테스트를 추가한다. diff_panel의 빈 제목 헤더를 생략하고 빈 안내에만 px4/py2를 적용한다. fmt/check/clippy·단위 테스트 후 수렴 점검한다.

추가 요청: Diff 표식은 h/line_height 22px로 맞춰 기본 18px 줄높이로 인한 위쪽 치우침을 줄인다.

## 드래그 캡처 계획

src/ui.rs selectable_line의 이동은 행의 전체 bounds 검사 대신 뷰어 세로 범위로 clamp한 y가 해당 행인지 판단한다. x는 제한하지 않고 shaped hit test의 시작/끝 clamp를 사용한다. 기존 전역 resize_move 캡처에서 활성 텍스트 선택의 뷰어만 바깥 방향으로 스크롤하고 resize_end에서도 해제한다. 모달/활성 탭/선택 key 검사와 시작 범위는 유지한다. clamp helper 테스트·fmt/check/clippy·단위 테스트 후 수렴 점검한다.

추가 요청: history_row는 head_commit과 commit.id를 비교해 행의 폰트 굵기를 바꾸고 배지에는 NORMAL을 명시한다.

## 키캡 공통화 계획

src/shortcut.rs에 조합키/안내/버튼 라벨 렌더 helper와 키 판별을 추가하고 src/main.rs에 등록한다. src/ui.rs의 도움말·상태줄·버튼 라벨·커밋/폐기 안내를 공통 키캡으로 전환한다. 키 입력 이벤트는 변경하지 않는다. 파서 검증·fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## 생성 팝업 계획

src/branches.rs에 원격 선택 후 Local 생성·선택적 publish 조합을 추가한다. src/ui.rs Create action에 remote bool을 추가해 체크박스와 연결하고 폰트/버튼 배치를 Create에만 적용한다. src/commit_editor.rs는 개별 글자 크기를 지원해 생성 입력만 13px로 한다. 파일명은 공통 14px를 명시한다. 임시 원격 테스트·fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## 체크박스·입력 계획

src/controls.rs에 고정 체크박스 helper를 추가하고 설정/브랜치 생성에 적용한다. Create action에 checkout bool을 추가하고 생성 함수는 선택 성공 시 switch를 실행한다. src/commit_editor.rs의 단일 행에 26px 슬롯-22px 줄의 중앙 오프셋을 공통 적용해 paint/IME/hit test를 맞춘다. window 캡처로 포인터가 입력 밖에서도 index_at·선택을 갱신하고 해제한다. fmt/check/clippy·단위 테스트 후 수렴 점검한다.

추가 점검: Clone URL은 별도 문자열 입력이라 공통 CommitEditor로 전환해 동일한 중앙 정렬/범위 밖 선택/IME를 적용한다. Enter 실행과 작업 중 비활성화는 유지한다.

## 체크 여백 계획

src/ui.rs button과 src/settings.rs action은 빈 라벨을 생략한다. 체크박스 래퍼만 py1·flex 중앙 정렬로 바꾸고 일반 버튼은 유지한다. fmt/check/clippy·기존 단위 테스트 후 수렴 점검한다.

## 공통 팝업 포커스 계획

src/popup.rs에 FocusHandle 목록 순환 helper와 테스트를 추가한다. src/ui.rs의 모달 Tab을 공통으로 처리하며 브랜치 체크 두 항목에 재사용 포커스를 추가한다. src/settings.rs도 같은 helper로 전역 체크를 포함하고 Space 전환을 제공한다. 체크 패딩은 controls의 공통 상수 6px를 사용한다. fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## 중복 Space 수정 계획

src/ui.rs 브랜치 체크박스의 Space KeyDown 전환과 src/settings.rs 전역 체크의 on_key_down 전환을 제거한다. 기존 FocusHandle과 on_click은 유지하여 GPUI 기본 KeyUp 클릭으로 한 번만 실행한다. fmt/check/clippy·기존 단위 테스트 후 수렴 점검한다.

## 숫자 포커스 계획

src/ui.rs에 섹션 매핑 helper를 추가하고 기존 modal/입력 가드 뒤에서 1~4를 처리한다. focus/list_focus만 바꾸며 4의 텍스트 커서가 없을 때만 초기화한다. 도움말을 추가하고 매핑 테스트·fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## 옵션·선택 유지 계획

src/session.rs에 기본값 호환 PopupOptions를 추가하고 Workspace 생성 옵션과 설정 체크 선호를 복원/저장한다. 설정은 전달된 선호를 load 후 반영하고 종료 시 되돌려준다. src/ui.rs의 o는 현재 참조/커밋으로 기존 체크아웃을 호출한다. Tag도 간소화된 footer를 사용한다. src/git.rs에 선택 커밋의 유효성 검사와 선택 페이지까지의 재조회 helper를 추가하고 reload에 사용한다. 생성 후 체크아웃의 선택 초기화는 생략한다. 세션/페이지 테스트·fmt/check/clippy·단위/통합 테스트 후 수렴 점검한다.

## 탐색 동작 계획

src/ui.rs에서 입력 가드 이후 탐색 키를 매핑한다. 브랜치 선택은 revision만 바꾸고 Space에서 기존 select_reference를 호출한다. 선택창의 숫자/클릭은 공통 메서드로 후보를 선택한다. 생성 성공은 다음 렌더에서 Workspace 포커스를 복원하고 기존 list_focus를 유지한다. 불필요한 더블클릭 지연 함수는 제거한다. 매핑 테스트·fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## 체크아웃 표시 계획

src/ui.rs refresh_after_branch에서 Local 탭/Branches 포커스 강제 지정을 제거하고 성공 후 실제 키보드 포커스만 복원한다. src/branches.rs 후보 label은 참조 이름만 사용하며 테스트 기대값을 갱신한다. fmt/check/clippy·단위 테스트 후 수렴 점검한다.

## Reset·선택 유지 계획

src/branches.rs에 ResetMode와 검증된 commit reset 함수를 추가한다. src/ui.rs BranchAction::Reset에 commit/mode/expanded를 보관하고 공통 BranchDialog로 처리한다. selectbox는 포커스/키보드/클릭을 지원하며 확인 전에는 변경하지 않는다. refresh_after_branch에서 선택/모드 초기화를 제거한다. src/popup.rs 하단을 우측 정렬하고 확인/취소 순서를 확인 우측으로 맞춘다. 임시 저장소 reset 상태 테스트·fmt/check/clippy·단위 테스트 후 수렴 점검한다.


## 히스토리 작업 팝업 확장 계획

src/branches.rs에서 커밋 해시 검증과 revert/merge/rebase 실행을 담당한다. 편집기를 띄우지 않도록 명시적 비대화 옵션을 사용하고 진행 중 Git 작업은 거절한다. src/ui.rs의 BranchAction에 Revert와 Integrate 상태를 추가한다. Merge/Rebase 목록은 별도 비동기 전체 히스토리 조회와 가상 목록으로 제공하며 원래 선택과 분리한다. Reset 라디오와 Revert 체크 입력은 기존 포커스/공통 footer를 재사용한다. src/session.rs에 no commit 기억을 추가한다. 임시 저장소 테스트와 fmt/check/Clippy로 검증한다.


## Reset 대상 표시 정리 계획

src/ui.rs의 Reset 팝업 렌더 분기에서 대상 표시만 제거한다. 실행 상태의 commit은 유지한다. 포맷과 컴파일 검사로 검증한다.


## Merge·Rebase 확인 간소화 계획

src/ui.rs의 Integrate 상태를 종류와 고정 커밋으로 줄인다. 팝업 전용 히스토리 조회·스크롤·세대·행 렌더링·방향키 선택을 제거하고 확인 문구로 대체한다. 기존 branches.rs 실행과 공통 확인 footer를 재사용한다. fmt/check/Clippy 및 기존 Git 작업 테스트로 검증한다.


## 단축키 통합 설정 계획

Rust/GPUI와 기존 Session 저장을 재사용한다. src/shortcut.rs에 안정 기능 ID, 영역, 기본 두 키와 사용자 재정의를 관리한다. 실제 입력을 등록 기능의 기존 명령 키로 정규화하여 src/ui.rs 실행 경로와 보호 조건을 유지한다. 입력창에서는 일반 앱 키를 처리하지 않으며 확인창 제어는 별도 범위를 사용한다. 설정 표와 도움말은 같은 정의에서 생성한다. 세션에 재정의를 저장하고 없는 필드는 기본값으로 읽는다. 키 충돌·변환·왕복 테스트로 검증한다. 헌법은 미작성 템플릿이므로 별도 제약을 적용하지 않는다. 한글 문서와 전 단계 절차를 준수한다.


## 방향 선택 확장 목록 정리 계획

src/ui.rs의 공용 shortcuts_table에서 select- 접두 기능 네 개를 표시 대상에서 제외한다. src/shortcut.rs의 실행·충돌 검사·기본값과 세션 데이터는 그대로 유지하여 기존 동작을 보존한다. 포맷·컴파일·기존 단축키 테스트로 검증한다.


## 확인창 공통 키 통합 계획

src/shortcut.rs의 창별 네 제어 정의를 dialog 범위 공통 네 정의로 교체한다. 해석 시 활성 창에 따라 확인을 기존 Enter 또는 Control+S 실행 경로로 변환하고 충돌 검사에 공통 범위를 포함한다. src/ui.rs의 표시와 단축키 설정창 취소·후보창 확인을 공통 정의에 연결한다. 기존 창별 재정의는 실행에서 무시한다. 회귀 테스트·포맷·컴파일·Clippy로 검증한다.


## 확인 표시·커밋 팝업 계획

src/ui.rs의 확인 버튼과 확인 안내를 단축키 없는 확인 버튼으로 정리한다. CommitDialog.push_after를 실제 체크 상태로 사용하며 submit_commit은 인자 대신 상태를 읽는다. 전용 체크 포커스를 추가하고 Tab 순환·클릭·Space 전환을 연결한다. 커밋 본문에 라벨/입력창/체크 순서를 적용하고 공통 footer에 확인 하나를 둔다. src/shortcut.rs의 커밋 후 푸시 키는 체크 전환 명령으로 변경한다. 기존 commit_then_push와 오류 복구를 재사용한다.


## 단축키 설정 탭 계획

src/ui.rs Workspace에 선택 탭 인덱스를 추가한다. shortcuts_table은 편집 모드에서 다섯 탭을 렌더링하고 선택 그룹만 표시한다. 상단 정적 안내 문구를 제거하며 복원·상태 메시지는 유지한다. 두 열기 경로에서 공통 탭으로 초기화하고 전환 시 등록·메시지를 해제한다. src/shortcut.rs에서 세 기능의 표시명만 변경한다. 기존 설정 ID와 실행은 유지한다.


## 히스토리 대상 행 제거 계획

src/ui.rs의 Revert 및 Integrate 렌더 분기에서 대상 커밋 행만 삭제하고 사용하지 않는 commit 패턴은 생략한다. BranchAction 상태와 submit_branch의 실행 대상 전달은 변경하지 않는다. 포맷·컴파일·Clippy와 소스 경로 검토로 검증한다.


## 단축키 일괄 승인·헤더 계획

src/ui.rs에서 활성 Bindings와 Option<Bindings> 임시 상태를 분리한다. 열기 시 복제하고 표·등록·해제·복원은 임시 상태만 사용한다. 탭 전환은 임시 데이터를 유지한다. 확인 시 임시 데이터를 넣은 Session을 먼저 저장하고 성공 후 활성 설정을 교체한다. 실패하면 원본과 편집 상태를 유지한다. 취소·바깥 클릭은 편집 상태를 해제한다. 복원·확인은 공통 footer 우측에 배치하고 헤더에 중성 회색 배경과 TEXT 글자를 적용한다. 기존 persist는 공통 세션 생성 메서드를 재사용한다.

저장 승인 검증을 위해 src/session.rs에 save_shortcuts를 두어 전체 세션 저장 성공 후 활성 설정을 갱신하고 다중 그룹·기본값 승인·실패 보존 테스트를 추가한다.


## 단축키 설정 헤더 제거 계획

src/ui.rs shortcuts_table의 열 제목 행은 비편집 도움말에만 렌더링하도록 조건을 제한한다. 설정에는 제목 행 없이 본문을 표시한다. px_3 등 패딩은 추가하지 않고 기존 셀 너비와 편집·승인 로직을 유지한다. 포맷·컴파일·Clippy로 검증한다.


## 단축키 설정 고정 높이 계획

src/popup.rs 공통 프레임에 선택적 고정 높이를 추가하고 해당 모드에서는 본문을 flex_1로 배치한다. src/ui.rs에서 단축키 설정일 때만 640픽셀을 전달하며 기존 viewport-64 상한으로 제한한다. 탭별 데이터나 항목 수는 높이 계산에 사용하지 않는다.


## 공통 방향·전환 통합 계획

src/shortcut.rs에서 nav 네 정의를 navigation 범위로 확장해 기존 주 화면과 reset에 적용하고 명칭을 방향키 방향으로 바꾼다. Reset 전용 방향 정의를 제거하고 창별 Space 전환을 input-toggle 하나로 합쳐 branch/reset/commit-modal에 적용한다. 범위 충돌은 실제 적용 범위의 교집합으로 검사한다. src/ui.rs에서 매핑된 Space의 포커스된 체크 상태 전환을 명시적으로 처리해 변경 키도 작동하게 한다. 브랜치 위치 이동·파일 Stage 정의 및 일반 입력은 유지한다.


## Stage 파일목록 범위 계획

src/shortcut.rs에서 stash/commit ID를 유지하며 파일목록 그룹·stage-files 범위로 옮긴다. stage-files는 기존 files·주 화면 공통·방향키 기능도 적용하고 충돌 교집합에 포함한다. src/ui.rs에서 활성 저장소 Files 포커스와 FileTab::Stage를 동시에 검사해 stage-files를 선택하고 두 실행 분기도 같은 범위로 제한한다. 실제 팝업 함수는 유지한다.


## 하위 분류·선택 숨김 계획

src/shortcut.rs에 그룹별 하위 분류·포함 판정·목록 표시 판정을 추가한다. 후보/Reset 숫자 정의는 보존하고 표시에서만 제외한다. src/ui.rs는 각 하위 분류별 같은 Command ID의 행을 만들고 UI 요소 ID에는 분류명을 포함한다. 빈 History 분류를 표시하며 두 슬롯은 동일 임시 Bindings를 읽고 쓴다. 실행·승인·고정 높이는 유지한다.


## 후보·Reset 키 배지 계획

src/shortcut.rs에 공통 배지 렌더러를 추가한다. src/ui.rs 후보 버튼은 빈 기본 라벨에 배지·참조 이름 행을 넣고 기존 첫 열 개 조건을 유지한다. Reset은 라디오 원·배지·모드 이름을 한 행에 구성한다. 키 라벨은 Bindings.label을 유지하고 선택 콜백은 변경하지 않는다.


## 단축키 슬롯 배지 계획

src/ui.rs 설정 슬롯 버튼의 기본 텍스트를 빈 값으로 하고 공통 shortcut::badge를 자식으로 넣는다. 읽기 전용 표도 같은 배지를 사용한다. 버튼 ID·클릭·등록 상태·삭제·임시 Bindings는 그대로 유지한다. 후보/Reset 배지의 라디오 라벨을 위해 src/controls.rs는 IntoElement를 받는다.


## 브랜치 삭제 팝업 계획

src/ui.rs의 Delete 제목·렌더링을 바꾸고 Delete를 minimal footer 분류에 포함한다. BranchAction::Delete.server를 checkbox에 직접 연결하며 refs/remotes 대상에서만 클릭/입력전환·포커스 순환을 허용한다. 기존 open_reference_delete의 server 기본값/Remote 제한과 submit_branch→branches::delete_reference 전달은 유지한다. 기존 삭제 테스트로 검증한다.


## 참조 종류별 연결 삭제 계획

src/branches.rs에 삭제 계획 스냅샷과 종류·참조 해시·연결 대상·원격 목적지 모델을 추가한다. for-each-ref upstream과 remote fetch refspec으로 관계·정확한 서버 목적지를 구하고 실행 전 재조회한다. 원격과 함께 삭제하는 모든 로컬 대상은 worktree/HEAD·upstream 병합 보호를 선검사하고 Local 단독 삭제는 기존 Git branch -d 판정을 유지한다. lease 서버 삭제·추적 정리 후 기존 안전 branch -d를 실행하며 부분 실패를 설명한다. 기존 delete_reference와 stash 삭제는 재사용한다. src/ui.rs의 Delete는 계획과 추가 삭제 bool을 보관하고 비동기 조회 후 종류별 제목·관계·체크를 표시한다. Tag/Stash 체크·취소 버튼은 제거하고 공통 키/입력전환을 유지한다. 임시 저장소 회귀 테스트로 검증한다.


## 브랜치 푸시·우선순위 계획

src/shortcut.rs의 branch-upload ID를 유지하고 이름·기본 키·적용 범위를 브랜치 푸시/shift-p/local-branches로 변경한다. Local 범위는 branches·공통·navigation을 상속한다. 실행 해석은 브랜치 그룹을 먼저 선택하고 충돌 검사에서 브랜치/공통의 중복만 허용한다. src/ui.rs에서 Local 포커스를 해당 범위로 구분하고 Upload 제목·본문·minimal footer를 정리한다. 기존 u 명령은 내부 실행 대상으로 유지하여 비동기 게시 경로를 재사용한다.


## 브랜치 푸시 키 복원 계획

src/shortcut.rs의 branch-upload 기본 슬롯만 u로 변경하고 기존 회귀 테스트의 기본 키 기대값 및 Local 공통 Shift+P 해석을 갱신한다. 나머지 이름·범위·팝업·사용자 재정의·우선 정책은 변경하지 않는다.

## 커밋 줄바꿈 설계 (US87)

src/ui.rs의 캡처 키 처리에서 수정키가 Shift뿐인 Enter와 커밋 메시지 포커스를 확인하고 단축키 해석 전에 반환한다. 진행 중이면 전파를 차단한다. 기존 src/commit_editor.rs의 다중 줄 Enter 처리와 선택 대체·조합 보호를 재사용한다. 새 데이터·의존성은 없으며 기존 Enter/확인 키와 다른 입력은 유지한다. 헌법은 미작성 템플릿이고 한글 문서·전체 절차를 준수한다.

## 파일 트리 설계 (US88)

src/model.rs에 원본 파일 인덱스를 보존하는 폴더/파일 행과 원시 경로 기반 정렬 트리 생성을 추가한다. 폴더 우선·이름 순으로 표시하며 같은 경로의 중복은 보존한다. StageSelection은 가시 인덱스 순서의 범위 선택을 지원한다. src/ui.rs는 실행 중 공통 모드와 목록 키별 접힌 원시 경로를 보관하고 토글·폴더 클릭·재조회 때 행 캐시를 갱신한다. 선택 인덱스와 diff는 그대로 두며 키보드 이동/스크롤을 가시 행에 대응시킨다. src/shortcut.rs에 파일목록 전용 백틱 명령을 등록하고 Stage·History 설정 분류에 표시한다. 기존 가상 목록을 재사용한다. fmt/check/clippy와 트리·선택·단축키 회귀 테스트로 검증한다. 헌법은 미작성 템플릿이며 한글 문서·전체 절차를 준수한다.

## 항상 펼친 트리·폴더 일괄 Stage 설계 (US89)

US88의 접힘 설계를 대체한다. src/model.rs의 폴더 행에서 접힘 필드를 제거하고 트리를 항상 펼친다. 디렉터리 경계(경로 뒤 /)로 하위 파일을 모아 미스테이징이 하나라도 있으면 전체 true, 아니면 전체 false의 중복 없는 변경 맵을 만든다. src/ui.rs는 목록 키와 원시 경로의 디렉터리 선택을 파일 선택과 구분하고 클릭·일반 방향 이동에서 폴더를 허용한다. Shift 이동은 파일만 대상으로 한다. 폴더 Space도 기존 stage_selected의 보호·비동기 toggle_files·재조회를 재사용한다. 재조회 후 폴더 선택을 유지하며 모드/탭 전환·파일/전체 선택은 이를 해제한다. 모델과 실제 임시 저장소 회귀 테스트·fmt/check/clippy를 수행한다. 새 의존성/영속 상태는 없으며 헌법은 템플릿이고 한글 문서·전체 절차를 준수한다.

## Local upstream 연결 설계 (US90)

US84~US85의 Local 푸시 UI를 대체한다. src/branches.rs에 Local 참조/해시·현재 upstream·후보의 참조/해시/원격/fetch source를 보관하는 계획을 추가한다. for-each-ref와 기존 fetch refspec 역매핑으로 symbolic과 불명확 후보를 제외하며 --set-upstream-to로 명시한 Local만 설정한다. 실행 전 계획의 Local/후보/현재 upstream/매핑을 재검증한다. 기존 publish_branch는 브랜치 생성의 원격 옵션에 필요하므로 유지한다. src/ui.rs는 Upload action/입력창을 Connect 계획+선택 인덱스 목록으로 대체한다. 비동기 조회·화살표/클릭 선택·스크롤·확인/취소·빈 목록·오류·진행 보호를 연결하고 성공 후 재조회/포커스를 복원한다. src/shortcut.rs의 branch-upload를 새 안정 ID branch-upstream과 Local 기본 r로 대체하고 전용 대화상자 범위에 공통 탐색/확인/취소를 적용한다. 실제 임시 저장소 및 단축키 테스트와 fmt/check/clippy로 검증한다. 헌법은 템플릿이며 한글 문서·전체 절차를 준수한다.

## 연결 팝업 라디오 설계 (US91)

src/ui.rs에서 Connect의 Local 표시 행만 제거하고 원격 후보를 공통 controls::radio로 표시한다. 전용 upstream_focus를 선택 행에 추적하고 초기/클릭/방향키/입력전환에 연결한다. 기존 selected 인덱스·현재 선택·scroll_to_item을 유지한다. 후보 행은 30px 고정·축소 금지, 목록은 최대 8행/240px 및 창 높이 제한으로 구성해 내부 스크롤을 보장한다. src/shortcut.rs의 upstream 범위에 공통 입력전환을 허용하고 Space는 선택만 유지한다. 새 데이터·Git 변경은 없다. 기존 연결/단축키 테스트와 fmt/check/clippy로 검증한다. 헌법은 템플릿이며 한글 문서·전체 절차를 준수한다.

## Merge 복구 설계 (US92)

src/model.rs의 Snapshot에 MergeState(정규화한 저장소 경로/MERGE_HEAD 수정 시각/HEAD/현재 브랜치/병합 대상/미해결 원시 경로)를 추가한다. src/git.rs는 gix 저장소의 실제 git 디렉터리와 인덱스 stage 1~3으로 상태를 읽고 Rebase는 제외한다. 충돌 파일은 경로별 한 Unstaged U 행으로 만들어 공통 조상과 현재 작업 결과의 미리보기를 제공한다. Stage 이후 미해결 경로를 갱신한다. src/branches.rs의 MergeControl은 현재 상태를 재조회·검증하고 Git merge --continue/--abort를 실행한다. 기본 병합 메시지와 비대화형 편집기 설정을 사용하며 실패를 반환한다. src/ui.rs는 상단 진행 표시·Stage 이동·충돌 배지·외부 수정 안내·완료/중단 확인을 기존 BranchDialog/비동기 실행에 통합한다. 충돌 없는 경우에만 완료를 허용하고 기존 경합 작업 단축키를 제한한다. 실제 저장소/linked worktree·미해결/완료/Abort/실패/오래된 대상 테스트와 fmt/check/clippy로 검증한다. 새 의존성은 없으며 헌법은 템플릿이고 한글 문서·전체 절차를 준수한다.

## Rebase 복구 설계 (US93)

src/model.rs에 RebaseState와 Snapshot.rebase 및 공통 진행 여부를 추가한다. src/git.rs는 실제 git 디렉터리의 rebase-merge/rebase-apply 메타데이터와 인덱스에서 원래 HEAD/브랜치 끝·onto·단계/전체·현재 커밋·충돌을 읽고 git am은 제외한다. 세션은 저장소/방식/orig-head 수정 시각·원래 대상·기반, 단계는 현재 HEAD/커밋/단계로 구분한다. src/branches.rs의 RebaseControl Continue/Abort/Skip은 실행 직전 상태를 검증하고 Git rebase 명령으로 수행한다. Continue/Skip 뒤 같은 세션의 새 단계 충돌이면 정상적인 해결 대기로 처리하고 실제 실패는 보존한다. src/ui.rs는 기존 Merge 배너·Stage 안내·경합 제한을 공통화하고 Rebase 확인창·계속/중단/건너뛰기·단계 이동 후 Stage 전환을 추가한다. Merge 구현은 유지한다. 실제 다중 충돌·Skip/Abort·실패/오래된 상태·apply/worktree 테스트와 전체 회귀·fmt/check/clippy를 수행한다. 새 의존성/영속 포맷은 없고 헌법은 템플릿이며 한글 문서·전체 절차를 준수한다.

## Revert 복구 설계 (US94)

src/model.rs에 RevertState(root/HEAD/브랜치/세션·대상 시각/대기 목록/원래 HEAD/옵션/no_commit/충돌)를 추가하고 공통 진행 여부에 포함한다. src/git.rs는 REVERT_HEAD·sequencer의 revert 목록/opts를 읽고 다른 작업은 제외한다. src/branches.rs의 앱 Revert는 검증한 커밋의 ^! 선택식으로 실행해 단일 대상도 Git의 옵션 저장을 사용한다. RevertControl은 재검증 후 일반 Continue/Abort/Skip을 실행한다. 단일 no commit 해결은 Git --quit로 메타데이터만 정리해 Stage와 HEAD를 보존한다. 외부 단일 옵션 미상은 확인창에서 모드를 고르고, 여러 커밋 no commit은 무변경 거절한다. src/ui.rs는 기존 공통 배너/Stage/가드·확인창에 Revert를 추가하고 일반 연속 충돌을 새 해결 대기로 처리한다. 실제 저장소 실험에서 native --continue가 no commit 옵션을 잃는 것을 확인했으므로 전용 회귀 테스트로 방지한다. 전체 fmt/check/clippy·기존 Merge/Rebase 회귀를 수행한다. 새 의존성/별도 영속 파일은 없고 헌법은 템플릿이며 한글 문서·전체 절차를 준수한다.

## Stash 복구 설계 (US95)

src/stash_recovery.rs를 라이브러리 모듈로 추가해 안전한 stash apply --index와 상태/완료/취소를 제공한다. worktree별 실제 Git 디렉터리 안의 전용 세션에 현재 추적 파일과 Stash 트리의 일반 파일 경로 합집합을 원시 바이트/권한/해시로 보관하고 원본 인덱스도 보관한다. 심볼릭 링크/재분석 지점/특수 파일/디렉터리 경계는 사전 거절한다. apply 후 인덱스 의미/파일 차이로 변경 경로를 기록하고 실패 시 active 메타데이터와 백업을 남긴다. 취소는 HEAD/브랜치·세션·백업 해시·범위 밖 인덱스를 재검증하고 index.lock 하에서 변경 파일만 원복한 후 원본 인덱스를 원자 교체한다. 범위 밖 작업 파일은 만지지 않는다. 완료는 미병합 0개일 때 메타데이터만 정리한다. 잠금 잔존/부분 실패는 자동 삭제하지 않는다. src/model.rs·src/git.rs의 Snapshot에 앱 복구 상태와 일반 미병합 경로를 포함하고 src/setup.rs는 새 서비스를 사용한다. src/ui.rs는 공통 배너/가드에 Stash 복구와 외부 충돌 안내를 연결하고 완료/취소 확인을 추가한다. 실제 데이터 보존/거절/재실행/worktree와 전체 회귀·fmt/check/clippy로 검증한다. 새 의존성은 없으며 헌법은 템플릿이고 한글 문서·전체 절차를 준수한다.

원본 Stage 객체는 복구 전용 Git 참조에 트리로 고정해 정리 작업에도 보존한다. 원본 인덱스와 별도로 분할 인덱스를 해제한 전체 사본을 보관하며, 공유 인덱스가 사라지면 검증한 전체 사본으로 복구한다. Windows Git에 전달하는 인덱스 경로는 확장 경로 접두사를 제거한다.

## DiffViewer 충돌 구간 선택 설계 (US96)

Rust·GPUI·gix와 기존 비동기 Stage 흐름을 재사용한다. src/conflict.rs에서 원문 바이트 범위·시작 줄·현재/들어오는 범위를 파싱하고 diff3 공통 조상은 제외한다. 잘못된 마커가 하나라도 있으면 전체 선택을 거절한다. src/model.rs의 Diff에 충돌 원문과 구간을 보관하고 src/git.rs에서 미병합 작업 파일에만 이를 채운다. src/ui.rs의 가상 목록에서 충돌 시작 줄을 같은 높이의 버튼 행으로 표시한다. 적용은 백그라운드에서 원문·미병합·일반 파일/안전 경로를 재검증한 뒤 해당 범위만 저장하고 선택 파일을 재조회한다. 전역 pending으로 중복/경합 실행을 제한한다. 인덱스는 변경하지 않는다. 기존 1 MiB/5,000줄 제한을 유지한다.

기술 미확정 사항은 없고 외부 API나 의존성 추가는 없다. 헌법은 미작성 템플릿이므로 확정 원칙은 없으며 한글 문서와 전체 절차를 준수한다. 검증은 파서 경계 테스트와 실제 임시 저장소의 적용·오래된 내용 거절·인덱스 보존 및 기존 회귀로 수행한다.

## 마커 보존·부분 Stage 설계 (US97)

기존 Rust·GPUI·gix·similar를 유지한다. src/model.rs의 DiffLine에 충돌 동작 행 구분을 추가하고 src/git.rs에서 시작 마커 직전에 별도 22px 동작 행을 삽입한다. 원문 마커 행과 줄 번호는 건드리지 않아 가상 목록/선택/스크롤 구조를 유지한다. US96의 시작 줄 대체 설계는 이 방식으로 갱신한다.

src/partial_stage.rs는 미리보기의 원문 두 개와 표시 행의 원본 줄 좌표로 선택 결과를 재구성한다. 선택하지 않은 삭제는 이전 내용에 남기고 선택한 추가만 넣어 현재 인덱스 위에 원하는 결과를 만든다. 표시 문자열은 저장에 사용하지 않는다. 일반 파일·모드·현재 인덱스 객체·작업 파일 원문·사용자 filter/working-tree-encoding 속성을 재검증하고 안전한 경로에 한해 Git apply --cached로 인덱스만 원자 갱신한다. 전체 파일 문맥을 포함한 패치를 사용해 기존 Stage와 다른 경로를 보존하며 Git의 인덱스 잠금을 재사용한다. 경로는 Git의 C 인용으로 표현하고 마지막 개행 표식을 보존한다. 외부 diff/사용자 필터는 실행하지 않는다.

src/ui.rs에 미리보기 세대별 줄 선택을 보관한다. 구간 헤더의 Stage 버튼과 추가/삭제 행의 체크박스, 상단 선택한 줄 Stage/선택 해제를 제공한다. 기존 pending·오류·apply_stage_files를 재사용하고 재조회 시 선택을 지운다. History/미병합/Stage된 쪽은 제외한다. 새 의존성·공개 외부 API는 없고 헌법은 미작성 템플릿이다. 설계 전후 한글 문서·전체 절차를 준수한다.

검증은 실제 임시 저장소에서 원본 인덱스/작업 파일과 선택 후 결과를 비교하고 구간/줄/추가/삭제/부분 Stage 누적·한글/특수 경로·CRLF/무개행·오래된 원문/인덱스/미병합/필터 거절을 확인한다. 마커 행 보존과 동작 행 매핑도 회귀 테스트로 고정한다.

## DiffViewer 키보드·고정 충돌 도구 모음 설계 (US98)

기존 Rust·GPUI·가상 목록·텍스트 Selection.cursor와 단축키 매핑을 재사용한다. src/shortcut.rs의 viewer 범위에 Space/Enter 명령을 추가하고 src/ui.rs의 텍스트 키 처리에서 기존 toggle_partial_line/stage_partial로 연결한다. 마우스와 키보드가 동일한 검증·체크 집합·비동기 경로를 사용한다. 선택 해제 UI만 제거하고 개별 토글 및 자동 초기화는 유지한다.

src/conflict.rs의 구간에 종료 원문 줄 번호를 저장하고 원문 줄 포함 여부를 제공한다. src/model.rs의 Diff가 표시 줄의 new 좌표로 현재 구간을 판정하며 좌표 없는 줄은 제외한다. src/git.rs의 합성 충돌 동작 행 삽입과 DiffLine의 해당 필드를 제거한다. src/ui.rs는 충돌 미리보기에서 상단 고정 도구 모음과 대상 구간/안내·세 버튼을 렌더링하며 클릭 직전 현재 커서를 재판정한다. 구간 밖에서는 실행하지 않는다. 원문 마커와 22px 가상 목록 및 5,000줄 제한은 그대로다. US97의 본문 동작 행 설계는 대체한다.

파일 조회 결과 적용 시 DiffViewer가 포커스 중이면 첫 표시 줄의 초기 Selection을 만들어 재조회 이후 방향키를 계속 쓸 수 있게 한다. 오래된 세대 결과는 기존 가드로 버린다. 새 의존성·영속 데이터·외부 API는 없고 헌법은 미작성 템플릿이다. 한글 문서와 전체 워크플로우를 준수한다. 테스트는 단축키 범위/재지정과 실제 충돌의 표시 줄→구간 경계 매핑·부분 Stage/충돌 회귀를 사용한다.

## 버튼 옆 단축키 안내 설계 (US99)

기존 Rust·GPUI·Bindings를 사용한다. src/shortcut.rs의 hint는 대체 키를 구분하고 빈 키캡을 생략한다. 명령 ID 기반 버튼 안내를 src/ui.rs의 기존 실행·확인·취소·부분 Stage 버튼에 연결한다. 이벤트·포커스·비활성 조건은 보존한다. 새 의존성·저장 상태·외부 API는 없다. 헌법은 미작성 템플릿이며 한글 문서와 전체 절차를 준수한다.

## 오류 표시와 열람 설계 (US100)

기존 Rust·GPUI를 유지한다. src/error_history.rs에 출처별 오류 전이 관찰·순서 보관·페이지 경계를 두고 src/main.rs에 등록한다. src/ui.rs에서 변경 알림 시와 렌더링 시 오류를 수집하며 탭과 팝업별 출처를 구분한다. src/settings.rs의 오류는 읽기 접근자와 부모 관찰로 연결한다. 기존 3초 오류 상태를 제거하고 정상 상태 표시를 유지한다. 최상위 왼쪽 하단의 느낌표와 그 위의 스크롤 가능한 열람 카드로 다른 작업 팝업의 상태를 보존한다. 저장 상태와 외부 API는 없다. 헌법은 미작성 템플릿이므로 확정 원칙은 없고 한글 문서·전체 절차를 준수한다.

## 오류 확인완료·최신 열기 설계 (US101)

src/error_history.rs의 open을 마지막 인덱스로 변경하고 현재 항목 제거 후 페이지를 보정한다. observed는 유지해 같은 발생 상태의 재추가를 억제한다. src/ui.rs의 카드에 GPUI on_mouse_down_out을 연결하고 캡처 단계에서 바깥 입력을 소비한다. 확인완료 버튼과 확인 단축키는 같은 삭제 경로를 사용하고 Esc는 닫기만 수행한다. 빈 기록이면 창을 닫고 스크롤은 페이지 전환 때 초기화한다. 새 의존성·저장 상태는 없고 헌법은 미작성 템플릿이다.

## 오른쪽 오류 표시·키 설정 설계 (US102)

src/ui.rs에서 두 하단 도움말 링크를 제거하고 오류 overlay/card를 right 기준으로 배치한다. 하단은 오류가 있을 때만 오른쪽 공간을 확보하고 왼쪽 상태 문자열 여백을 복구한다. 아이콘은 14px/10px를 사용한다. 토글 경로를 클릭/F10이 공유하며 입력 캡처/진행 중에는 기존 보호를 유지한다. src/shortcut.rs에 공통 오류 기록 명령과 전역 충돌 범위를 등록하고 confirm의 두 번째 기본 슬롯을 비운다. 내부 확인 명령 변환은 유지한다. 새 저장 상태·의존성은 없고 헌법은 미작성 템플릿이다.

## DiffViewer 범위·커서·버튼 설계 (US103)

src/text_selection.rs에 범위 행 계산과 유효 행 집합 기반 커서 이동을 둔다. src/ui.rs에서 DiffLine의 old/new 좌표가 있는 행 목록을 사용하여 초기화·키보드·마우스/드래그·전체 선택을 제한한다. 범위 체크는 기존 세대/Stage 보호를 통과한 추가/삭제 행 집합만 기존 partial_selection에 반영하며 클릭 체크는 기존 한 줄 동작을 유지한다. 선택한 줄 Stage 버튼은 최소 220px와 줄바꿈 금지/축소 금지를 적용하고 바깥 도구 모음의 줄바꿈을 유지한다. 새 의존성·저장 상태는 없다. 헌법은 미작성 템플릿이다.

## 하단 오류 순서·자동 너비 설계 (US104)

src/ui.rs에서 오류 표시와 기록 카드 렌더링을 분리한다. 공통 error_indicator는 고정 너비 없이 느낌표/건수만 렌더링하고 두 하단 행에서 shortcuts_link 앞에 둔다. GPUI deferred를 사용해 하단의 실제 레이아웃을 유지하면서 기존 팝업보다 위에 표시한다. 카드만 기존 최상위 overlay를 사용하고 닫힌 상태의 빈 overlay와 160px 예약 여백을 제거한다. 새 상태/의존성은 없고 기존 명령·오류 모델은 유지한다. 헌법은 미작성 템플릿이다.

## 오류 표시 왼쪽·배경 설계 (US105)

src/ui.rs의 error_indicator는 px_2와 BG 배경을 사용한다. 두 하단 바에도 BG를 지정하여 공통 바탕을 유지한다. 저장소 하단은 오류 표시를 상태 문자열 앞에 두고 빈 탭은 오류와 오른쪽 링크 사이에 유연한 공간을 둔다. 카드 overlay는 왼쪽 16px와 left_0 기준으로 전환한다. 기존 deferred·내용 너비·이벤트는 유지한다. 새 상태/의존성은 없고 헌법은 미작성 템플릿이다.

## 오류 발생 기록·복사 설계 (US106)

src/error_history.rs에 오류 발생 시 report를 추가해 메시지를 기록하고 동일 출처의 관찰 기준도 함께 갱신한다. src/ui.rs의 새 오류 대입은 이 경로를 사용하고 렌더/notify 관찰은 중복을 추가하지 않는다. src/settings.rs는 오류 대입마다 발생 버전을 증가시키고 부모는 엔티티/버전으로 관찰한다. src/commit_editor.rs에 읽기 전용 모드를 두어 선택/복사/탐색만 허용하고 키/붙여넣기/IME 변경을 차단한다. 오류 창은 현재 페이지의 읽기 전용 편집기를 캐시하고 전체 복사 버튼/본문 포커스에 따른 Ctrl+C를 제공한다. 새 의존성/영속 데이터는 없으며 기존 정상 입력 편집기를 보존한다. 헌법은 미작성 템플릿이다.

## US107 명시적 투명 배경 설계

src/ui.rs의 error_indicator 기본 BG를 rgba(0x00000000)로 바꾸고 내부 느낌표 배경도 명시적으로 투명하게 한다. deferred는 유지하여 팝업 위에서도 접근하되 아래의 backdrop이 그대로 보이게 한다. DANGER_HOVER는 유지한다. 새 상태/의존성은 없고 헌법은 미작성 템플릿이다.

## 오류 메시지 줄바꿈 설계 (US108)

src/commit_editor.rs의 읽기 전용 모드에만 가용 너비/실측 글자 폭 기반 표시 행을 생성한다. 표시 행은 원문 바이트 시작 좌표를 보관하여 기존 선택/복사/히트 테스트를 유지한다. 세로 바 너비를 예약해 줄바꿈하고 가로 overflow와 가로 바/커서 가로 이동을 차단한다. 일반 편집기의 기존 레이아웃은 유지한다. 새 의존성/상태는 없고 헌법은 미작성 템플릿이다.

## 오류 전달·Revert 파일 설계 (US109)

src/error_history.rs에 기록을 추가하지 않고 출처 관찰 상태만 갱신하는 mirror를 둔다. src/ui.rs의 submit_branch 실패 결과는 refresh_after_branch에서 한 번 report하고 팝업은 mirror로 전달한다. 브랜치 확인의 is_held 이벤트를 차단한다. src/branches.rs의 revert_commit은 실패 후 기존 gix revert_state의 실제 충돌 목록을 진단에 추가하고 원래 오류를 보존한다. src/ui.rs의 Revert 배너 아래에는 현재 conflicts 경로를 최대 높이 목록으로 표시한다. 새 저장 상태/의존성은 없고 헌법은 미작성 템플릿이다.

## 대괄호 영역 탭 이동 설계 (US110)

src/shortcut.rs에 branch-tab-previous/next(scope branches), files-tab-previous/next(scope files)를 등록하고 파일 History 분류에도 노출한다. 기존 applies가 local-branches/stage-files를 포함한다. src/ui.rs는 탭 순환 계산과 참조/파일 전환 메서드를 분리해 키/클릭이 공유한다. 해당 목록 포커스와 Window focus를 유지하고 기존 파일 선택/차이 세대를 초기화한다. 새 상태/의존성/외부 API는 없고 헌법은 미작성 템플릿이다.

## 체크박스 입력 소유권 설계 (US111)

src/controls.rs에 체크박스 클릭의 마우스 여부/키 반복 정책을 둔다. src/ui.rs에서 수동 키 토글이 있는 다섯 클릭 핸들러(생성 두 체크박스는 한 핸들러)를 마우스 클릭으로 제한한다. 키 처리에서는 해당 체크박스가 포커스된 입력전환 반복만 소비해 상태 변경을 막는다. GPUI의 자동 keyup 클릭은 이 핸들러에서 변경을 만들지 않으며 기본/재지정 keydown은 기존 경로를 사용한다. 설정은 자동 keyup 한 경로이므로 변경하지 않는다. 새 상태/의존성은 없고 헌법은 미작성 템플릿이다.

## 설정/프로젝트 탭과 원격 키 설계 (US112)

src/shortcut.rs에서 기존 branch/files 이전/다음 명령을 공통 tab-previous/next(scope tabs)로 통합하여 목록/shortcuts-modal에서 공유하고 project-tabs 범위 Ctrl 대괄호 명령을 추가한다. shortcuts-modal은 공통 대화상자 범위와 충돌 검사에 포함하고 project-tabs는 일반 목록/뷰어/main/clone에 적용한다. 원격 작업은 기본 입력만 바꾸고 내부 target f/p/Shift+P는 유지한다. src/ui.rs에서 설정 탭 클릭/키와 프로젝트 클릭/키를 각각 공유 경로로 통합하고 기존 adjacent_tab을 재사용한다. 프로젝트 전환은 활성 탭 스크롤·조회·포커스/선택 정리·세션 저장을 수행한다. 새 영속 데이터/의존성은 없고 헌법은 미작성 템플릿이다.

## 공유 단축키 통합 설계 (US113)

src/shortcut.rs에서 files-all을 공통 selection 범위(파일/뷰어), branch-checkout을 공통 checkout-action 범위(브랜치/히스토리)로 확장하고 viewer-all/history-checkout 중복 명령을 제거한다. slots는 기준 ID 우선/이전 ID 보완 승계를 제공한다. 공통화된 체크아웃은 브랜치 우선순위 예외 대신 일반 충돌 검사를 따른다. subsections/in_subsection은 브랜치 공통/Local/Remote와 파일목록 공통/Stage로 중복 없이 분류한다. src/ui.rs의 공유 섹션 제목에 적용 대상을 표시한다. 기존 target과 키 실행 분기는 유지하고 새 영속 모델/의존성은 없다. 헌법은 미작성 템플릿이다.
## 공통 섹션 명칭 설계 (US114)

src/ui.rs의 제목 생성에서 적용 대상 접미사 매핑을 제거하고 기존 subsection을 그대로 사용한다. 도움말의 그룹 경로와 명령 분류·설정 저장은 유지한다. US113의 적용 대상 제목 설계를 대체한다. 새 모델이나 의존성은 없다.

## 해제 버튼 높이 설계 (US115)

src/ui.rs의 단축키 지정/해제 버튼 높이를 기존 배지 20px와 상하 패딩 16px에 해당하는 36px로 명시한다. 해제 버튼은 세로 패딩을 제거하고 flex 가운데 정렬해 기본 글줄 높이에 따른 팽창을 방지한다. 공용 button과 다른 화면은 수정하지 않는다.

## 기본값 복원 단축키 설계 (US116)

src/shortcut.rs에 공통 shortcuts-reset 명령을 shortcuts-modal 범위, ctrl-d 기본/target으로 등록한다. 기존 충돌·두 슬롯·저장을 재사용한다. src/ui.rs에 초안 초기화 메서드를 추출해 버튼과 키가 공유하며 기존 캡처 분기를 우선한다. 버튼에 button_hint를 표시한다. 신규 데이터 구조나 의존성은 없다.

## 기본값 복원 배경 설계 (US117)

src/ui.rs의 reset-shortcuts 버튼에 기존 BUTTON 회색을 지정하고 BUTTON_HOVER를 적용한다. 공용 버튼과 다른 화면은 변경하지 않으며 신규 모델·의존성은 없다.

## 중복 호버 오류 수정 설계 (US118)

공용 button은 이미 hover를 등록하므로 src/ui.rs의 reset-shortcuts 호출부에서 추가 hover를 제거한다. BUTTON 배경은 유지한다. US117의 별도 BUTTON_HOVER 적용 설계를 대체한다. 공용 버튼이나 의존성은 변경하지 않는다.

## 단축키 설정 탭 고정 설계 (US119)

src/popup.rs에 Body 구조체로 선택적 고정 헤더와 스크롤 콘텐츠를 전달한다. frame은 제목 아래·popup-body 스크롤 위에 flex_shrink_0 헤더를 배치한다. src/ui.rs에서 탭 생성 메서드를 분리해 설정 팝업에만 헤더를 전달하고 shortcuts_table에는 목록/메시지만 둔다. 기존 고정 높이·본문 flex/min_h·하단 버튼을 유지한다. 다른 팝업은 헤더 None으로 기존 동작을 유지하며 신규 의존성/영속 데이터는 없다.

## 세팅 재구성과 내부 탭 방향키 설계 (US120)

src/settings.rs의 SettingsTab/use_global/preferred_global/체크 포커스를 제거한다. 한 본문에 작성자·remote_info·인증 메서드를 결합한다. 세 버튼용 FocusHandle 배열을 두고 Tab 순환과 수동 Enter/Space 처리·마우스 전용 클릭을 적용한다. 불러오기는 GitContext에서 최신 전역 identity를 읽어 draft만 갱신한다. 저장은 명시적 global bool을 받고 진행/IME/오류 처리를 공유한다. 전역 저장 후 snapshot만 갱신해 편집값을 유지한다. 저장소 저장은 repository 존재 시만 가능하다. src/ui.rs는 새 생성자 및 키 활성화 경로를 사용하고 옛 체크 선호값을 읽거나 갱신하지 않는다. 세션의 기존 직렬화 필드는 호환 목적으로 유지한다.

src/settings_git.rs의 global 저장이 local 값을 지우지 않도록 수정하고 저장 대상 scope의 identity를 검증한다. SettingsSnapshot의 체크박스용 use_global은 제거한다. rollback은 저장 대상 범위만 복구한다. 임시 저장소와 격리한 전역 파일 테스트로 상호 보존·불러오기 읽기 전용·잘못된 입력/잠금 오류를 검증한다.

src/shortcut.rs에서 tab-previous/next 기본값만 left/right로 변경하고 내부 target [/]는 유지한다. nav-left/right는 horizontal-navigation scope로 분리해 내부 탭 영역에서는 적용하지 않고 main/history/viewer/reset/upstream에서 유지한다. 공통 scope 충돌·두 슬롯과 기존 재지정은 유지한다. popup의 고정 탭 구조는 유지한다. 새 의존성은 없다.

## 공통 방향으로 내부 탭 전환 설계 (US121)

src/shortcut.rs에서 tab-previous/next 정의와 tabs scope를 제거하고 horizontal-navigation 범위를 branches/local-branches/files/stage-files/shortcuts-modal로 확장한다. nav-left/right는 내부 탭과 동일한 충돌 정책을 따른다. 기존 별도 내부 탭 override는 참조하지 않는다. src/ui.rs의 분류/패널 탭 전환은 해석된 left/right target을 사용하고 기존 순환·고정 헤더·캡처 우선을 유지한다. 프로젝트 명령은 별도로 유지하고 1번 기본값 Ctrl+left/right·2번 Ctrl+j/l, 기존 내부 target Ctrl+[/]를 사용한다. 기존 프로젝트 override는 보존한다. 새 모델·의존성은 없다.

## 전역 작성자 체크 설계 (US122)

src/settings.rs에 use_global/local_draft와 체크/저장용 두 포커스를 둔다. snapshot에서 override 유무를 읽어 초기 체크를 정하고 체크 전환은 draft를 보존해 global/local을 표시한다. 세 버튼을 체크박스와 저장으로 대체하고 기존 수동 keydown/마우스 전용 클릭 보호를 재사용한다. 저장은 전역 사용이면 새 GitContext::use_global_identity, 아니면 기존 local save_identity를 사용한다. src/settings_git.rs의 새 메서드는 global을 읽고 local 두 키만 제거하며 실패 시 local 값을 복구한다. snapshot에 use_global 상태를 다시 제공하지만 옛 세션 선호는 사용하지 않는다.

src/commit_editor.rs에 읽기 전용 전환 메서드를 제공한다. 기존 read_only 입력 보호를 재사용하되 자동 줄바꿈/가로 스크롤 금지는 여러 줄 읽기 전용에서만 적용해 단일행 이름/이메일의 레이아웃과 복사를 유지한다. 새 의존성은 없다.

## 세팅 하단 저장 설계 (US123)

src/settings.rs의 작성자 저장 렌더링을 공개 save_button 메서드로 분리해 같은 포커스/가드/마우스 전용 클릭을 유지하고 현재 확인 키 안내를 받는다. focus_next 순서에서 저장을 인증 URL 다음에 배치한다. src/ui.rs의 세팅 분기에서 header_action을 제거하고 settings.update로 생성한 버튼을 공용 popup footer에 전달한다. 저장/닫기 및 backend는 변경하지 않고 새 의존성은 없다.

## 저장 강조 색상 설계 (US124)

src/settings.rs의 save_button에 SELECT 배경·ACCENT 글자색을 적용한다. 기존 action의 단일 hover를 유지하고 포커스 때 테두리를 표시해 강조 배경과 구분한다. 신규 의존성/동작 변경은 없다.

## 전역 작성자 안내 제거 설계 (US125)

src/settings.rs의 identity_section에서 해당 안내를 담은 child를 제거한다. 나머지 체크·저장·입력 및 레이아웃은 유지한다. 신규 의존성/상태는 없다.

## 세팅 섹션 제목 스타일 설계 (US126)

단축키 섹션 제목은 상속 text_sm·ACCENT·SEMIBOLD를 사용한다. src/settings.rs의 세 제목에도 같은 스타일을 적용한다. 레이아웃·기능과 기존 글꼴 가족 상속은 유지하며 새 의존성은 없다.

## git2 실행 백엔드 설계 (US127)

Cargo.toml/Cargo.lock에 git2 0.21.0을 추가한다. src의 공유 라이브러리 백엔드에 repository/config/index/ref/remote/통합 API를 제공하고 기존 제품 호출부를 교체한다. 기존 문자열 기반 경계가 필요한 곳은 인프로세스 요청 어댑터를 사용하되 임의 셸이나 Git fallback은 제공하지 않는다. gix 객체 조회와 UI 모델은 유지하고 쓰기 후 기존 refresh 경로로 갱신한다.

src/setup.rs의 생성/원격/커밋/index/폐기, src/branches.rs의 참조·체크아웃·안전 삭제·통합 제어, src/settings_git.rs의 config, src/partial_stage.rs의 속성 검사/index 쓰기, src/stash_recovery.rs의 stash/index/worktree 작업을 전환한다. credentials.rs는 helper 실행을 직접 수행하고 원격 콜백은 저장된 자격 증명/SSH agent를 사용한다. 충돌 상태 파일·작업 중 세션·abort 시 보존 정책은 기존 조회 모델과 맞춘다. unknown/비호환 작업은 변경 전 거부한다.

기존 테스트의 외부 Git은 픽스처/정답 생성용으로 분리하고 제품 함수는 실제 git2를 호출하게 한다. 임시 저장소·로컬 bare remote·격리 global 파일로 동작을 비교하며 실제 계정 인증은 자동 검증하지 않는다. 헌법은 미작성 템플릿이다.

### US127 확정 라이브러리 역할

기본 조회·config 파싱/쓰기·ref transaction·객체/index/커밋은 gix 기반 공유 모듈에서 처리한다. git2는 clone/fetch/push/인증 콜백과 체크아웃/restore/reset·Merge/Rebase/Revert·Stash 복합 흐름에서 사용한다. 기존 문자열 요청 경계를 받는 어댑터는 운영체제 명령이 아닌 인프로세스 dispatcher이며 지원 요청만 처리하고 외부 Git fallback을 제공하지 않는다.

### US127 구현 파일과 추가 보호

src/native_git.rs는 고정된 인프로세스 요청 dispatcher, native_local.rs는 gix config/index/commit(서명 포함), native_refs.rs는 gix ref/설정 기반 upstream, native_remote.rs는 git2 원격/인증 콜백, native_operations.rs는 git2 체크아웃/통합/Stash를 담당한다. 읽기 전 index lock과 부분 Stage 이전 ID 비교를 사용한다. split-index는 gix로 원자적으로 펼치고 복구 백업은 gix 전체 index로 보완한다. Windows 테스트는 프로젝트 내부 임시 디렉터리를 사용한다. 통합 테스트의 잘못된 경로 fixture는 상위 저장소 유무와 무관하도록 존재하지 않는 경로를 명시한다.

Windows의 gix 시스템 설정 위치 탐색도 Git 실행을 유발할 수 있어 config/attributes의 git_binary와 자동 system 탐색을 비활성화했다. 알려진 시스템 설정 및 GIT_CONFIG_SYSTEM 파일은 직접 읽어 gix 설정 앞에 병합하고 user/local/환경 설정의 우선순위를 유지한다. 생성/조회/쓰기 모두 이 옵션을 공유한다.

## US128 전체 스테이징 상태 복구 계획

상태 조회와 쓰기 저장소를 같은 시스템 설정 병합 함수로 열되 조회는 필터 신뢰 함수를 계속 차단한다. src/native_local.rs에 공통 열기 함수를 제공하고 src/git.rs가 사용한다. src/setup.rs에서 별도 자식 테스트 프로세스로 시스템 설정 환경을 격리하여 CRLF 신규/수정 파일 및 로컬 재정의를 검증한다. 설정 파일이나 실제 사용자 인덱스는 테스트로 변경하지 않는다.

## US129 스태시 추가 팝업 계획

src/ui.rs의 stash_dialog 표시 분기만 수정한다. 추가 제목을 바꾸고 MUTED 스타일의 입력 라벨을 editor 앞에 배치한다. 설명은 삭제 팝업에서만 표시하며 추가 취소 버튼을 제거한다. 확인·오류·busy 및 외부 클릭/단축키 닫기 로직을 유지한다. 표시 변경이므로 구현을 반복하는 테스트는 추가하지 않고 포맷·컴파일과 코드 검토로 검증한다.

## US130 일반 그래프 스태시 제외 계획

src/git.rs에서 tips 구성 시 RefTab::Stash만 제외한다. 목록은 유지하고 HEAD와 일반 브랜치·태그 도달성은 보존한다. src/ui.rs의 select_reference는 스태시를 직접 select_commit한 뒤 그래프 탐색 대상을 해제하고 반환한다. tests/repository.rs의 스태시 회귀를 신규 파일·두 스태시 및 공유 도달성 검증으로 갱신한다. 페이징 자체 구현은 변경하지 않는다.

US130 보완: 선택한 스태시를 유지한 새로고침도 src/git.rs에서 일반 그래프의 추가 페이지 탐색 대상에서 제외하고 다중 페이지 회귀를 검증한다.

## US131 읽기 전용 입력 배경 계획

src/commit_editor.rs의 입력 영역과 스크롤바 배경을 read_only 상태에 따라 theme::BG 또는 theme::INPUT으로 지정한다. 기존 연한 회색 테마를 사용하여 색을 새로 추가하지 않는다. 이벤트·선택·복사 처리는 유지한다. 표시 변경이므로 별도 동작 테스트를 추가하지 않고 포맷·컴파일 및 두 배경 경로를 검토한다.

## US132 입력 패딩 배경 계획

src/commit_editor.rs에서 상태별 배경색을 공개 메서드로 공유한다. 기존 입력과 스크롤바가 이를 사용하고 src/settings.rs의 field 래퍼도 editor 상태를 읽어 같은 배경을 테두리 안쪽 전체에 적용한다. field 호출에는 읽기 컨텍스트를 전달한다. 입력 및 포커스 동작은 유지하고 포맷·컴파일로 검증한다.

## US133 저장 버튼 포커스 제외 계획

src/settings.rs의 identity_focus를 체크박스 전용 단일 핸들로 변경한다. 저장 버튼의 track_focus·포커스 테두리·클릭 focus·Tab 대상을 제거한다. activate_focused는 체크박스만 처리한다. src/ui.rs의 save_button 호출에서 window 인자를 제거한다. 클릭·확인 단축키와 저장 조건을 보존하고 포맷·cargo check --offline으로 검증한다. 헌법은 미작성 템플릿이며 신규 의존성은 없다.

## US134 원격 추가 계획

src/native_local.rs의 설정 잠금·원자 교체를 공통 편집 함수로 추출하고 원격 추가에서 URL·fetch를 한 번에 기록한다. 중복은 유효 설정과 잠금 이후 로컬 설정에서 검사한다. src/native_git.rs가 원격 추가 함수를 재노출하고 src/settings_git.rs는 원격 추가 진입과 격리 회귀를 제공한다. src/settings.rs가 추가 핸들·두 입력·팝업 상태를 보유하며 기존 세팅 카드 내부를 추가 팝업 내용으로 전환한다. src/ui.rs는 제목·닫기·키 처리에 하위 팝업을 연결한다. 성공 후 remote_info만 갱신해 작성자 초안을 보존한다. 기존 원격 상세를 이름·URL 목록으로 교체한다. 신규 의존성·외부 Git·네트워크는 없다. 미작성 헌법은 추가 제약을 부여하지 않는다.

## US135 세팅 유지 계획

src/settings.rs는 원격 제목·목록 뒤에 별도 버튼 줄을 추가한다. SettingsPanel 본문은 항상 세팅을 렌더하고 추가 팝업 본문은 별도 메서드로 분리한다. src/ui.rs는 세팅 제목과 너비를 고정하고 기존 세팅 overlay 다음에 추가 팝업 overlay를 렌더한다. popup::frame으로 클릭 전파를 차단하고 최상위 backdrop은 close_remote만 호출한다. 기존 Esc·Tab·Space·저장 입력은 하위 팝업 상태로 제한한다. 오류·진행 표시는 위 팝업에만 표시한다. 새 의존성·데이터 변경은 없으며 미작성 헌법의 제약은 없다. 표시 변경은 포맷·컴파일과 경로 검토로 검증한다.

## US136 입력 테두리 보완 계획

src/settings.rs의 remote_popup 본문에 하단 여백을 추가하여 마지막 입력 테두리가 overflow_y_scroll 경계에 닿지 않게 한다. 두 field 래퍼는 flex_shrink_0으로 자연 높이를 유지한다. 공용 팝업과 다른 입력 스타일은 변경하지 않는다. 기존 GPUI 스타일만 사용하고 포맷·컴파일과 경로 검토로 검증한다. 헌법은 미작성 템플릿이다.

## US137 원격 삭제 계획

src/native_refs.rs의 원격 요청 분기에 remove를 추가하고 git2의 remote_delete로 표준 원격 삭제를 인프로세스로 수행한다. src/settings_git.rs는 삭제 메서드와 격리 회귀를 제공한다. src/settings.rs의 원격별 행 묶음에 삭제 아이콘을 하나씩 배치하고 비동기 삭제 후 remote_info만 갱신한다. 작업 상태와 팝업 표시 상태로 실행을 제한한다. 기존 아이콘 문자와 공용 버튼 스타일을 사용하며 네트워크·외부 Git·신규 의존성은 없다. 미작성 헌법은 제약이 없다.

### US137 빈 원격 섹션 보완

git2 삭제 후 빈 원격 섹션이 남을 수 있으므로 src/native_refs.rs는 값 없는 섹션을 목록에서 제외한다. src/native_local.rs의 중복 검사도 빈 섹션을 무시하여 같은 이름으로 재등록할 수 있게 한다. Windows의 git2 테스트는 프로젝트 내부 임시 디렉터리에서 실행한다.

## US138 삭제 버튼 포커스 계획

src/settings.rs에 원격 이름별 FocusHandle 맵을 두고 스냅샷 갱신 시 기존 핸들을 유지·추가·제거한다. Tab 배열에 목록 순서대로 삭제 핸들을 넣고 버튼 track_focus·포커스 스타일·마우스 포커스를 연결한다. 포커스 실행은 Space에서만 삭제하도록 src/ui.rs에서 실행 종류를 전달한다. 반복 키 판정은 삭제 핸들도 포함한다. 성공 후 기존 추가 버튼 복귀 플래그를 사용하고 실패 시 핸들을 보존한다. 기존 GPUI 기반을 재사용하며 신규 의존성·데이터 변경은 없다. 헌법은 미작성 템플릿이다.

## US139 Fetch 오류 진단 계획

src/native_remote.rs에서 전송 오류를 원격 이름·git2 오류 class/code·원문 진단으로 구성한다. URL은 사용자 정보·쿼리·프래그먼트를 제거하고 해당 비밀값도 진단에서 가린다. Fetch의 원격 조회/전송 실패에 같은 진단 경계를 적용한다. 외부 네트워크 없이 합성 오류와 존재하지 않는 로컬 원격을 이용해 전달 경로를 검증한다. 기존 setup Fetch 로컬 회귀로 성공 경로를 확인한다. 신규 의존성이나 인증/인증서 정책 변경은 없다. 헌법은 미작성 템플릿이다.

### US139 HTTPS 기능 활성화

Cargo.toml의 git2 의존성에 https 기능을 명시하고 Cargo.lock을 갱신한다. native_remote 진단 회귀에 git2::Version::get().https() 검증을 추가한다. 로컬 원격 실패 회귀는 file URL을 사용하고 기존 Fetch 회귀는 프로젝트 내부 임시 디렉터리에서 실행하여 Windows 환경 경계를 격리한다. 이번 변경은 HTTPS만 활성화하며 SSH 의존성을 새로 추가하지 않는다.

## US140 인증 목록 구현 계획

src/auth_profiles.rs를 공용 라이브러리에 공개한다. 앱 데이터의 auth.json에는 id·정규화 HTTPS URL·username·방식만 저장하고 프로세스 파일 잠금/원자 교체를 사용한다. Windows Credential API의 앱 네임스페이스에 직접/OAuth 비밀을 저장하고 메타데이터 쓰기 실패 시 롤백한다. 저장소 경로별 최장 일치와 username 조건으로 전송 인증을 선택한다. native_remote는 직접 비밀 또는 등록된 GCM 계정 힌트를 기존 콜백에서 사용한다.

src/credentials.rs는 기존 GCM을 재사용하되 계정 선택을 지원한다. src/oauth.rs는 reqwest blocking HTTPS로 GitHub Device Flow 요청·poll·slow_down·만료·취소와 계정 조회를 구현한다. GitHub 고정 endpoint와 검증된 브라우저 URL만 연다. GCM은 공유 인증을 변경하지 않고 읽기/브라우저 로그인하며 앱 삭제는 해당 메타데이터만 지운다.

src/settings.rs는 인증 목록·이름별 포커스·추가 팝업 상태·방식/브라우저 선택·동적 입력·비동기 실행·취소를 보유한다. src/ui.rs는 상위 팝업 렌더·닫기·기존 확인 라우팅에 연결한다. CommitEditor에 비밀 모드를 추가하고 byte 길이 보존 마스크와 UTF-8 경계 보정·클립보드/IME 원문 읽기 차단·비밀 지우기를 적용한다. 기존 비밀 아닌 입력은 유지한다.

Cargo.toml은 Windows Credentials API와 reqwest blocking/json/rustls 기능을 추가한다. 가짜 보안 저장소와 HTTP 전송 추상화로 비밀을 쓰거나 실제 OAuth 계정 없이 회귀를 검증한다. 헌법은 미작성 템플릿이다. 실제 브라우저 로그인·사용자 계정·보안 저장소 조작은 자동 검증하지 않는다.

### US140 인증 UI 분리

인증 목록·작업·팝업은 src/auth_settings.rs의 AuthPanel 엔티티로 분리한다. SettingsPanel은 관찰과 키/닫기/최상위 팝업 라우팅을 위임하며 작성자·원격 입력과 수명을 분리한다. 공용 resolve 함수는 가짜 보안 저장소로 실제 전송 인증 선택 경로를 검증한다.

## US141 기존 인증 조회와 버튼 보완 계획

src/auth_settings.rs의 button은 빈 라벨 child를 만들지 않고 인증 목록 로드에서 auth_profiles::load_with_existing을 사용한다. src/auth_profiles.rs는 CredEnumerateW의 git:* 메타데이터에서 TargetName·UserName만 읽고 CredentialBlob에는 접근하지 않는다. URL 자격 증명·쿼리/프래그먼트는 제거하고 HTTPS만 정규화한다. 기존 AuthProfile에 serde 기본값 imported·hidden을 추가해 과거 파일을 유지한다. 대상과 계정의 안정적인 id로 병합하며 직접 등록과 같은 URL/계정이면 직접 등록을 우선 표시한다. 자동 가져온 항목은 전송 계정 강제 선택에서 제외하고 기존 GCM 흐름을 유지한다. 삭제 시 hidden tombstone으로 재등장을 막고 공유 비밀은 유지한다. 조회/저장 실패는 기존 앱 목록과 경고를 반환한다. 가짜 메타데이터 입력으로 회귀를 검증한다. 새 의존성·헌법 제약은 없다.

## US142 인증 표시 필터 계획

src/auth_profiles.rs에 현재 원격 URL 배열에 대해 정확 주소부터 호스트까지 부모 범위를 탐색하고 처음 일치하는 범위의 활성 프로필을 반환하는 순수 함수를 추가한다. 표시 함수는 imported도 포함하며 전송의 select는 변경하지 않는다. 원격 URL의 사용자 정보는 비교용으로 제거하고 .git/끝 슬래시를 정규화한다. URL segment 경계로 부모를 계산한다.

AuthPanel은 전체 활성 인증과 표시용 인증 및 원격 URL 배열을 분리한다. 모든 로드/등록/삭제 결과에서 표시 목록과 핸들을 재계산한다. SettingsPanel은 최초 스냅샷과 원격 갱신 후 가져오기/푸시 URL을 AuthPanel에 전달한다. 빈 목록은 원격 없음/일치 인증 없음으로 구분한다. 인증 저장은 전역 그대로 유지한다. 신규 의존성·영속 데이터·헌법 제약은 없다.

## US143 삭제 로그아웃 계획

credentials::logout을 제품 코드에서 사용하고 LoginTarget::for_account로 protocol/host/path/username을 포함한 GCM erase를 호출한다. auth_profiles에 목록 파일 잠금 안에서 GCM 제거 콜백을 먼저 실행하는 remove_after_logout 경계를 추가한다. GCM 실패는 메타데이터를 쓰기 전에 반환하며 성공 후 메타데이터 실패는 부분 성공으로 명확히 보고한다. 실제 로그아웃한 imported 항목은 hidden 표식을 남기지 않는다. AuthPanel 삭제는 프로필 방식에 따라 직접 비밀 제거 또는 GCM 제거+등록 제거를 선택하고 목록을 다시 필터링한다. 가짜 runner/보안 저장소와 임시 파일로 검증하며 실제 사용자 계정 로그아웃은 테스트하지 않는다. 새 의존성·헌법 제약은 없다.

## US144 오류 키 배지 계획

src/ui.rs의 error_indicator에서 Bindings::label("errors")를 읽고 오류 개수 다음에 shortcut::keys 배지를 추가한다. 여러 바인딩은 구분하여 개별 배지를 생성하고 비어 있는 슬롯은 생략한다. 두 하단 영역이 같은 함수를 사용하므로 기존 조건·클릭·이벤트 처리를 유지한다. 표시 변경은 경로 검토·포맷·컴파일·기존 단축키 회귀로 검증한다. 신규 의존성·데이터·헌법 제약은 없다.

## US145 오류 표시와 키 변경 계획

src/ui.rs의 두 하단에서 error_indicator의 deferred/priority 래퍼를 제거하여 팝업 backdrop 아래에 일반 자식으로 렌더한다. src/shortcut.rs의 errors target/default를 f1로 변경하고 ui.rs의 전역 오류 키 디스패치도 f1을 확인한다. 배지는 기존 label 조회로 자동 갱신한다. 회귀는 F1 전역·기본 F10 미해결·alt-z 재지정·충돌 보호를 검증한다. 기존 사용자 바인딩 파일은 변경하지 않는다. 신규 의존성·데이터·헌법 제약은 없다.

## US146 GCM 실제 제거 검증 계획

auth_profiles에 기존 Windows 메타데이터 열거/정확 target 제거를 추상화한 ExistingCredentialStore를 추가한다. 프로필 URL·계정 또는 imported id로 선택 대상을 찾고 CredDeleteW로 지운 뒤 다시 열거해 잔존하면 실패한다. 기존 메타데이터 normalizer를 재사용하고 CredentialBlob은 접근하지 않는다. credentials에 GCM erase와 검증된 Windows 제거를 결합하는 logout_profile 경계를 추가한다. GCM 없어도 정확한 Windows 대상 제거가 확인되면 성공하며 둘 다 증거가 없으면 오류다. AuthPanel은 잠금 콜백에서 이 경계를 사용하여 실제 제거 후에만 앱 목록을 변경한다. 가짜 저장소/runner로 검증하고 실제 사용자 계정은 테스트에서 제거하지 않는다. 새 의존성·헌법 제약은 없다.

## US147 경로와 GCM 조회 계획

auth_profiles::normalize_url은 검증과 표준 URL 직렬화만 하고 .git/경로 구성요소를 삭제하지 않는다. 등록·중복·표시·전송 선택은 URL 파싱 기준으로 비교하여 root 표준형과 경계만 처리하고 .git 없는 구주소에 대한 예외는 추가하지 않는다. ResolvedAuth::Gcm에 실제 등록 url을 포함한다.

native_remote::callbacks는 프로젝트 경로를 소유해 GCM 명령의 current_dir에 전달한다. 선택된 GCM 프로필의 URL로 요청하고 원격 기본 조회는 실제 원격 URL 그대로 사용한다. credential_request는 HTTPS host/port/path/username을 안전하게 구성한다. Clone은 대상 부모 문맥을 넘긴다. helper 명령 생성과 요청은 순수 검증 가능한 경계로 분리한다.

Fetch 진단 formatter를 전송 공통 formatter로 확장해 Push 원인과 서버 callback의 거부 이유를 보존한다. 비밀 가림과 기존 안전한 Push/lease 정책은 유지한다. 실제 D:/Project/zit·사용자 인증 파일은 수정하지 않는다. 경로/문맥과 서버 오류 회귀는 가짜 요청·임시 저장소로 검증한다. 새 의존성·헌법 제약은 없다.

### US147 로컬 전송 회귀 환경

src/setup.rs의 configured_repository 테스트 fixture는 프로젝트 내부 임시 디렉터리를 사용한다. Windows 진단 환경에서 기본 OS 임시 디렉터리에 대한 libgit2 접근이 거부되어 로컬 clone 회귀가 실패했으므로 테스트 경계를 작업 공간으로 한정했다. 실제 프로젝트 경로나 인증 정책의 예외는 추가하지 않았다.

## US148 추가 방식 정리 계획

src/auth_settings.rs의 최상위 방식/핸들 배열을 BrowserGcm·Password 두 항목으로 바꾸고 기본을 BrowserGcm으로 설정한다. 별도 GCM 버튼/설명을 제거하고 GCM 등록은 대화식 authenticate로만 실행한다. 내부 AuthMethod::Gcm은 자동 조회/기존 프로필/전송/삭제에 필요하므로 유지한다. 표시 변경은 경로 검토·포맷·컴파일·Clippy·기존 인증 회귀로 검증한다. 신규 의존성·영속 데이터·헌법 제약은 없다.

US148 입력 보완: 브라우저 GCM의 username field와 Tab 대상을 제거하고 LoginTarget에 수동 계정 힌트를 주지 않는다. username은 Password 방식에서만 읽는다. 독립 OAuth는 기존 Client ID만 사용한다.



## US149 독립 OAuth 제거 계획

src/auth_settings.rs의 하위 선택·Client ID·기기 코드·OAuth 비동기 등록과 토큰 안내를 제거한다. 브라우저 GCM URL 로그인과 Password 등록을 유지한다. src/main.rs의 모듈 선언·src/oauth.rs·Cargo.toml의 전용 reqwest 의존성을 제거하고 Cargo.lock을 갱신한다. auth_profiles의 기존 BrowserOAuth 저장·전송·삭제는 유지한다. 새 의존성이나 데이터 변환은 없다. 헌법은 미작성 템플릿으로 추가 제약이 없다.


## US150 구현 계획

src/file_manager.rs에서 플랫폼별 프로그램과 단일 경로 인수를 구성하고 실행한다. Windows는 숨겨진 프로세스로 실행해 콘솔 창을 방지한다. 프로세스 대기는 백그라운드에서 처리한다. src/ui.rs는 활성 프로젝트 경로를 캡처하고 버튼과 키가 같은 메서드를 호출하며 실패를 오류 기록으로 전달한다. src/shortcut.rs에 공통 explorer/f10 명령을 등록한다. 프로젝트 footer에서만 단축키 왼쪽에 표시한다. 새 의존성은 없고 헌법 템플릿에 추가 제약은 없다.


## US151 구현 계획

src/terminal.rs에서 실행 파일 탐색과 플랫폼별 Command 구성을 분리한다. Windows는 PATH 및 Git 실행 파일 주변·Program Files·LocalAppData의 Git Bash를 탐색하고 --cd= 인수로 프로젝트 경로를 전달한다. Linux는 x-terminal-emulator·gnome-terminal·konsole·xfce4-terminal·xterm 중 설치된 도구를 사용하고 Bash에 고정 스크립트와 별도 위치 인수로 프로젝트 경로를 전달한다. macOS는 고정 AppleScript에 경로/Bash를 argv로 전달하고 quoted form으로 Terminal 명령을 구성한다. 프로세스 실행과 대기는 백그라운드에서 수행하고 실패는 오류 기록에 전달한다. src/ui.rs의 footer와 key 처리에 연결하고 src/shortcut.rs의 공통 terminal/f9 등록으로 재설정을 지원한다. 신규 의존성·영속 데이터·헌법 제약은 없다.


## US152 Search 구현 계획

src/git_search.rs에서 모든 로컬·원격 브랜치 및 태그의 도달 가능한 커밋을 중복 없이 순회하여 커밋 메시지, 파일 경로 및 텍스트 파일 내용을 검색한다. 취소·결과 제한·바이너리 제외로 비용을 통제한다. src/ui.rs의 브랜치 탭에 Search 전용 입력·결과·키보드 선택 상태를 추가하고 Enter 검색, Space 커밋 위치 이동, O/더블클릭 파일 창 열기를 연결한다. 파일 창은 gpui 별도 Window로 만들며 좌측 파일 커밋 이력과 우측 독립 DiffViewer UI를 그린다. 두 패널의 외형은 기존 화면과 맞추되 기존 UI 컴포넌트는 재사용하지 않는다. 파일 이력은 모든 브랜치·태그의 커밋을 대상으로 한다.


## US153 계획

src/ui.rs에서 Search 표시 시 다른 탭의 활성 스타일을 막고 입력 컨테이너의 높이·패딩·정렬과 결과 text_xs를 맞춘다. src/git_search.rs에서 메시지 바이트를 손실 대체 문자열로 읽고 본문 일치 줄을 결과에 반영한다. src/search_window.rs는 theme 글자 크기와 Diff 줄별 좌우 번호 컬럼을 렌더링하며 Diff 외곽 패딩을 제거한다. 데이터 저장 형식과 의존성 변경은 없다.


## US154 구현 계획

src/ui.rs의 검색 결과를 기존 브랜치 행 높이·글자 크기에 맞추고 유형 배지를 색상별로 그린다. 결과 포커스 여부에 따라 선택 색을 나누고 Space는 Search를 유지한다. src/git_search.rs의 파일명/경로 검색 조건을 검증한다. src/search_window.rs는 제목 높이·이력 행 높이를 줄이고 Diff를 읽기 전용 선택 가능 텍스트 영역으로 바꾸며 기존 줄 번호를 유지한다. 창 내부 1/2/Esc를 처리한다. 부모 Workspace는 자식 WindowHandle과 종료 구독으로 자식 생존 기간을 추적하며 부모 입력 차단과 재활성화를 적용한다. 영속 데이터와 새 의존성은 없다.


## US155 계획

src/ui.rs의 Search 전용 키 처리에서 목록 포커스의 좌우 이동을 기존 탭 전환 함수로 보내고, 입력 포커스에서는 글자 커서 이동을 유지한다. 검색 완료 후에도 입력을 재선택할 수 있게 목록의 위 방향과 전용 진입키를 제공하며 이전 검색 결과를 세대번호로 배제한다. src/git_search.rs의 전체 메시지 검색과 본문 일치 회귀를 확인한다. 파일 창은 독립 Diff 행 UI에 기존 theme 색상과 줄 배경을 적용하고 복사를 위한 선택 상태를 제공한다. 두 패널은 흰 배경을 사용하고 포커스에 따른 강조를 적용한다.

## US156 계획

src/ui.rs Search 결과 행에 기존 브랜치 목록과 같은 미선택 호버를 적용한다. src/search_window.rs는 선택 행 위치와 ScrollHandle을 관리하고 목록 포커스의 위·아래 키를 선택/스크롤/비동기 Diff 로딩에 연결한다. 양 패널 사이에는 중립색 드래그 핸들과 폭 상태를 둔다. src/commit_editor.rs의 파일 Diff 전용 표시에서 줄 번호·변경 기호·본문의 너비와 시작 위치를 실제 글리프 위치로 구분하고 자동 줄바꿈 대신 가로 스크롤을 허용한다. 기존 선택·복사 동작과 별도 창 구조는 유지한다. 관련 회귀와 포맷·정적 검사를 수행한다.

## US157 계획

src/search_window.rs의 커밋 목록은 `uniform_list`와 전용 스크롤 핸들로 바꾸고 행 생성은 가시 범위에서만 수행한다. 크기 조절 핸들은 투명한 히트 영역을 유지하되 레이아웃 폭을 없애고 목록·Diff 위에 절대 배치하여 양쪽 공백을 제거한다. src/commit_editor.rs의 파일 Diff 모드는 텍스트 변경 시 줄 시작 오프셋과 최대 너비를 준비하고 렌더에서는 뷰포트와 스크롤 위치로 보이는 행만 형상화·그린다. 커서 좌표와 키보드 이동은 전체 줄 인덱스를 기준으로 계산한다. 본문 시작 경계로 클릭·커서·선택 표시를 제한하고 기호를 3문자 열 가운데 놓는다. 줄 번호 열에는 회색 배경과 오른쪽 경계선, 기호 열에도 오른쪽 경계선을 그린다. 읽기 전용 복사와 기존 포커스 이동·종료 동작은 보존한다.

## US158 계획

src/search_window.rs의 가상 커밋 목록 행에 전체 너비와 최소 너비 0을 지정한다. 행 내부 텍스트는 남은 공간에서 줄임표로 표시하여 긴 제목이 행의 너비를 늘리지 않게 한다. 선택·호버·클릭은 행 루트에 유지한다. 별도 데이터 변경이나 의존성은 없다.

## US159 계획

src/git_search.rs의 파일 이력 반환형을 ID·제목·작성자 이름/이메일·작성 시각/오프셋을 담는 구조체로 확장한다. src/search_window.rs는 기존 이력과 동일한 네 열 순서와 날짜 함수를 사용하되 독립 그리드 행·헤더를 구현하고 `uniform_list`를 유지한다. 패널은 세로 Flex 배치로 바꾸고 목록 높이를 창 로컬 상태로 관리한다. 기존 절대 배치 드래그 핸들을 수평선으로 회전해 세로 드래그를 받는다. 키보드 선택과 Diff 로딩은 새 구조체 ID를 사용한다. 변경된 백엔드 회귀와 포맷·정적 검사를 수행한다.

## US160 계획

src/ui.rs의 파일 이력 창 기본 크기를 960×640으로 줄이고, 현재 Workspace의 `shortcut::Bindings` 복사본을 창 생성자에 전달한다. src/search_window.rs는 목록 포커스의 키를 `Bindings::resolve("history", …)`로 해석해 공통 방향키의 기본값과 사용자 변경값을 동일하게 처리한다. 위·왼쪽은 이전, 아래·오른쪽은 다음 커밋으로 이동시키며 기존 스크롤·Diff 갱신을 유지한다. 기본/변경 바인딩 단위 테스트와 포맷·컴파일 검사를 수행한다.

## US161 계획

src/search_window.rs는 Diff 편집기 포커스에서 `Bindings::resolve("viewer", …)`로 키를 변환한다. 읽기 전용 편집기가 지원하는 이동·선택·복사 키만 전달하고, 물리 키가 단축키 설정에서 해제됐으면 기존 편집기의 원시 키 처리를 차단한다. src/commit_editor.rs는 키 처리 본문을 재사용 가능한 메서드로 분리하여 자식 창이 매핑된 키를 직접 전달하도록 한다. 파일 창의 1/2/Esc 처리와 Diff 가상 스크롤은 유지한다. 기본·변경 바인딩 회귀와 포맷·컴파일 검사를 수행한다.

## US162 계획

src/search_window.rs의 커밋 목록 패널에 상대 위치 기준을 두고, 헤더 아래부터 패널 끝까지 이어지는 절대 배치 그리드 가이드를 행 뒤에 그린다. 가이드는 실제 헤더/행과 같은 유동 Description 및 고정 Author·Date·Hash 폭을 사용한다. 가상 목록과 실제 행을 가이드보다 위에 두어 선택·호버·클릭은 그대로 동작한다. 커밋 수와 관계없이 별도 빈 행은 만들지 않는다.

## US163 계획

src/commit_editor.rs의 파일 Diff 전용 캔버스에서 16개의 고정폭 글리프를 형상화해 번호 열과 기호 열의 실제 끝 좌표를 얻는다. 줄 렌더링 전에 전체 캔버스 높이에 회색 번호 배경 및 두 세로 경계선을 그린다. 실제 Diff 줄은 기존 행 배경과 선택을 위에 그리며 번호/기호 영역의 회색 배경과 선을 행별로 다시 그려 색상과 경계를 유지한다. 빈 영역은 텍스트 버퍼와 가상 줄 수에 영향을 주지 않는다.

## US164 계획

src/ui.rs의 `diff_row`에서 줄 번호 두 셀을 회색으로 묶고 두 번째 번호 셀 오른쪽에 경계선을 추가한다. 변경 기호 셀은 고정 20px 폭에 오른쪽 경계선을 추가하고 기호를 가운데 정렬한다. `diff_panel`은 가상 목록 뒤에 동일한 열 폭의 배경 가이드를 배치하여 짧은 Diff의 빈 높이를 채운다. 부분 Stage의 24px 선행 셀과 가로 스크롤 오프셋을 가이드 위치에 반영한다. 상세 텍스트 뷰는 기존 렌더링을 유지한다. 포맷·컴파일·관련 회귀를 수행한다.

## US165 계획

src/ui.rs의 Diff 파일명 헤더에 하단 경계선을 적용한다. `cursor_rows`, Diff 로드 후 초기 선택, `selectable_line`의 커서 허용 조건에서 줄 번호 필터를 제거한다. 부분 Stage 행 토글의 추가·삭제 줄 필터는 유지한다. `resize_move`의 전역 마우스 캡처에서 텍스트 드래그 중인 뷰어의 뷰포트와 스크롤 오프셋으로 실제 행·열을 계산하고, 경계 밖 포인터도 뷰포트 끝으로 제한해 선택 끝점을 갱신한다. 기존 줄별 마우스 이동 콜백은 제거하여 중복 갱신을 방지한다. 회귀와 포맷·정적 검사를 수행한다.

## US166 계획

src/ui.rs의 기존 Diff 행에서 줄 번호 두 셀을 Flex 세로 중앙 정렬로 바꾼다. 본문 캔버스의 `WrappedLine::paint`는 GPUI가 22px 줄 높이에 맞춰 글리프를 중앙 정렬하므로 현재 줄 높이를 유지한다. `scroll_frame`의 Diff 뷰어 호출 결과에 클릭 처리를 연결해 실제 행 아래 빈 공간에서도 `ListFocus::Text`와 창 포커스를 설정한다. 기존 선택이 없을 때만 첫 실제 줄로 초기화한다. 빈 공간에는 가짜 행을 추가하지 않는다. 포맷·컴파일·정적 검사를 수행한다.

## US167 계획

src/ui.rs의 기존 Diff 행 변경 기호 셀에서 Flex 세로 정렬을 제거하고 본문 캔버스와 같은 22px 줄 높이로 텍스트를 그린다. 셀의 20px 폭과 가로 중앙 정렬, 경계선은 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US168 계획

src/ui.rs의 기존 Diff 행에서 이전·현재 줄 번호를 그리는 두 48px Flex 셀에 오른쪽 정렬을 적용한다. 기존 세로 중앙 정렬과 셀 패딩·배경·경계선은 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US169 계획

src/ui.rs의 Search 결과 선택 인덱스에 가상 위치 상수를 도입한다. Search 탭 클릭과 Tag 탭의 오른쪽 이동 시 Workspace 포커스와 가상 위치를 설정한다. 키 처리에서는 가상 위치의 아래 방향키가 입력창으로 들어가고, 입력창의 위·아래 방향키가 각각 가상 위치와 첫 결과로 이동하도록 한다. 첫 결과의 위 방향키는 입력창으로 돌아간다. 결과가 없으면 아래 방향키도 가상 위치로 이동한다. 기존 검색 실행과 좌우 탭 이동은 유지한다. 포맷·컴파일·정적 검사와 키 이동 회귀를 수행한다.

## US170 계획

src/ui.rs의 `cycle_panel_tab`에서 Search만 먼저 처리하는 분기를 제거한다. 현재 참조 탭과 Search 활성 상태를 다섯 탭의 인덱스로 환산하고 공통 `adjacent_tab`으로 다음 인덱스를 계산한다. 결과가 Search이면 가상 위치와 Workspace 포커스를, 다른 참조 탭이면 기존 `switch_reference_tab`을 사용한다. 브랜치 목록에 포커스가 있을 때만 브랜치 순환을 적용해 파일 탭 이동을 침범하지 않게 한다. 다섯 탭 순환 단위 테스트와 포맷·컴파일·정적 검사를 수행한다.

## US171 계획

src/ui.rs의 브랜치·파일·커밋 히스토리 `scroll_frame` 호출 결과에 클릭 가능 ID와 포커스 핸들러를 연결한다. 클릭 시 활성 프로젝트인지 확인하고 각각 `ListFocus::Branches`, `Files`, `History`와 Workspace 포커스를 설정한다. 선택 인덱스나 모델 값은 변경하지 않는다. 행 클릭과 중복으로 실행되어도 같은 포커스만 설정하도록 한다. 포맷·컴파일·정적 검사를 수행한다.

## US172 계획

src/ui.rs의 `file_row` 파일명 셀에 전체 행 높이와 24px 줄 높이를 지정한다. 기존 줄임표·색상·선택 표시를 유지하고 포맷·컴파일·정적 검사를 수행한다.

## US173 계획

src/ui.rs의 파일 상태 셀에서 Stage 탭에만 `text_sm`을 적용하고 History는 기존 `text_xs`를 유지한다. 고정폭 글꼴과 22px 상태 칸, 공통 상태 색상 계산은 그대로 둔다. 포맷·컴파일·정적 검사를 수행한다.

## US174 계획

src/ui.rs의 `diff_row` 부분 Stage 체크 셀에 줄 번호 영역과 같은 회색 배경, 오른쪽 경계선, 22px 줄 높이와 가로 중앙 정렬을 적용한다. `diff_column_guides`의 부분 Stage 선행 24px 열에도 같은 배경과 경계선을 그려 실제 행 아래로 이어지게 한다. 기존 가로 스크롤 오프셋과 체크 아이콘 클릭 처리는 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US175 계획

src/ui.rs의 `diff_row` 체크 셀과 `diff_column_guides` 체크 열에서 오른쪽 경계선만 제거한다. 회색 배경·24px 폭·아이콘 클릭과 빈 공간 연장은 유지한다. `file_row`의 Stage 상태 셀에서 `Cascadia Mono` 지정만 제거하여 History와 같은 상속 글꼴을 사용한다. 기존 Stage `text_sm` 크기와 상태 칸·색상은 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US176 계획

src/ui.rs의 Stage 파일 상태 셀에서 공백을 포함한 단일 문자열 렌더링을 두 개의 동일한 고정 폭 자식 칸으로 바꾼다. `FileChange::stage_status()`의 기존 두 문자 형식을 왼쪽·오른쪽 칸에 각각 매핑하고 공백은 비어 있는 칸으로 그린다. 비례 글꼴의 M 등 넓은 문자도 겹치지 않도록 각 칸을 14px로 두며 Stage 글자 크기·색상을 유지한다. History 상태 셀은 그대로 둔다. 포맷·컴파일·정적 검사를 수행한다.

## US177 계획

src/ui.rs의 Stage 상태 셀 전체 폭을 26px로, 두 자식 문자 칸의 폭을 각각 13px로 조정한다. 문자 매핑과 글꼴·크기·색상은 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US178 계획

src/ui.rs의 Stage 상태 셀 전체 폭을 22px로, 두 자식 문자 칸의 폭을 각각 11px로 조정한다. 두 상태의 독립된 배치와 기존 글꼴·크기·색상을 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US179 계획

src/ui.rs의 파일 상태 셀에 공통 `text_xs`를 적용한다. Stage는 History와 같은 상속 글꼴을 이미 사용하므로 크기 지정만 통일한다. 두 상태 칸의 폭과 색상은 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US180 계획

src/ui.rs의 Stage 상태 셀 전체 폭을 18px로, 두 자식 문자 칸을 각각 9px로 조정한다. 상태 문자 매핑과 글꼴·크기·색상은 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US181 계획

src/ui.rs의 Stage 상태 셀과 두 문자 칸을 행의 전체 높이로 채운다. 각 문자 칸에 파일명과 같은 24px `line_height`를 지정해 동일한 행 중앙 기준으로 그린다. 두 칸 폭·상태 글자 크기와 파일명 표현은 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US182 계획

src/ui.rs의 `branch_panel`에서 빈 목록 안내 요소에 `px_3`과 `py_2`를 지정해 공통 `empty`의 20px 패딩을 해당 호출 위치에서만 덮어쓴다. 다른 빈 상태 문구와 글자 스타일은 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US183 계획

src/ui.rs의 `file_row` 파일명 셀에서 14px 절대 크기를 `text_xs`로 바꾼다. 두 파일 탭에 공통 적용되는 셀이므로 별도 분기는 추가하지 않는다. 24px 줄 높이와 상태 칸·줄임표 표현은 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US184 계획

src/search_window.rs의 `FileHistoryWindow`에 Author·Date·Hash의 창 로컬 너비와 활성 열 드래그 상태를 저장한다. Description은 창 너비에서 세 고정 열의 폭을 뺀 나머지를 사용한다. 세 인접 열 경계의 이동량을 최소 너비에 맞춰 제한하는 순수 함수를 구현한다. 헤더의 각 경계에 7px 폭 리사이즈 핸들을 놓고 기존 목록 높이 핸들과 함께 루트 마우스 이동·해제 이벤트에서 열 드래그를 처리한다. 커밋 행과 빈 영역 가이드는 같은 너비 상태를 사용한다. 단위 테스트로 각 경계와 최소 폭을 확인하고 포맷·컴파일·정적 검사를 수행한다.

## US185 계획

src/ui.rs의 새 탭 시작 흐름에서 신규 저장소는 `prompt_for_paths`로 기존 디렉터리를 선택하고, Clone만 `prompt_for_new_path`를 사용한다. src/setup.rs에 기존 디렉터리 초기화 함수를 추가해 경로 유효성 및 `.git` 부재를 확인하고 `native_git::init_repository`를 호출한다. Windows에서는 기존 파일 속성을 보존하며 숨김 비트만 추가하는 `SetFileAttributesW`를 사용한다. Windows가 아닌 환경에서는 `.git` 이름 그대로 둔다. Clone용 `setup::create`의 기존 새 경로 정책은 유지한다. 기존 파일 보존·중복 방지·숨김 속성 회귀와 포맷·컴파일·정적 검사를 수행한다.

## US186 계획

src/ui.rs의 `최근 저장소 관리` 팝업 footer에서 `cancel-recent-action` 버튼만 제거한다. 두 관리 동작과 팝업 바깥 클릭·Esc 처리, 별도 저장소 삭제 확인창의 취소 버튼은 유지한다. 포맷·컴파일·정적 검사를 수행한다.

## US187 계획

src/ui.rs의 Clone 시작 경로에서 `prompt_for_new_path`를 제거하고 다른 로컬 작업과 같은 디렉터리 선택 창을 사용한다. src/setup.rs에 기존 빈 디렉터리로 복제하는 함수를 추가한다. URL 검증 후 대상이 디렉터리인지, 내부 항목이 없는지 확인하고 기존 `native_git::clone_repository`를 호출한다. Windows에서는 복제 후 `.git`에 기존 숨김 속성 함수를 사용한다. 기존 `setup::create`의 테스트용 새 경로 동작은 변경하지 않는다. 빈 폴더 복제·내용 보호·숨김 속성 회귀와 포맷·컴파일·정적 검사를 수행한다.

## US188 계획

src/ui.rs의 삭제 확인 팝업에서 취소 버튼을 없애고 문구를 영구 삭제로 바꾼다. src/setup.rs의 기존 `deletion_target` 검증을 재사용한 뒤 `remove_dir_all`로 저장소 폴더를 직접 삭제한다. Windows 휴지통 COM 구현과 불필요한 기능 의존성을 제거한다. 임시 저장소만 대상으로 직접 삭제와 보호 검증을 수행한다.

## US189 계획

src/ui.rs의 Clone 실행 버튼 표시 문자열을 `Clone 실행`으로 바꾼다. 해당 버튼은 US187의 복제 흐름을 실행한다.

## US190 계획

src/shortcut.rs의 새 탭 명령에 기본 `ctrl-n`을 등록하고 새 탭 화면의 `clone` 범위를 포함한다. src/ui.rs에서 명령 해석 뒤 기존 `add_empty_tab`을 호출하고 창 포커스를 작업 공간으로 돌린다. 팝업 범위에서 실행하지 않는다. 기본 및 변경 키 해석을 시험한다.

## US191 계획

src/ui.rs의 새 탭 작업에 전용 FocusHandle 목록을 만들고 Tab·Shift+Tab 순환 순서를 구성한다. Clone URL 편집기와 최근 저장소의 열기·삭제를 포함하며 목록 변동에 맞춰 포커스 대상을 갱신한다. Space에서는 포커스된 대상의 클릭 동작을 호출한다. 대상에 포커스 강조 스타일을 표시하고 처리 중 실행을 막는다. 순서 계산은 순수 함수로 분리해 테스트한다.

## US192 계획

src/shortcut.rs의 프로젝트 탭 명령에 기본 `ctrl-x`를 등록한다. 프로젝트 탭 범위 중 저장소가 활성화된 상태에서만 src/ui.rs의 기존 `close_tab`을 호출한다. 팝업은 명령 범위에서 제외하고 Clone URL 입력창은 텍스트 편집 동작을 유지한다. 탭 닫기 경로와 단축키 해석을 단위 테스트 및 포맷·컴파일·정적 검사로 확인한다.
