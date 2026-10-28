# 구현 계획: 단일 화면 Git 클라이언트

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
