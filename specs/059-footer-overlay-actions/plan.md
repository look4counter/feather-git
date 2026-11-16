# 구현 계획: 공용 인코딩 오버레이와 하단 버튼

## 요약과 기술 맥락

Rust 2024/GPUI Windows 앱이다. 기존 Dialog의 중앙 카드·어두운 배경·입력 차단을 재사용하고 외부 의존성은 추가하지 않는다. 새 EncodingDialog가 인코딩 목록과 키보드 커서를 소유하고 선택/취소 이벤트를 Workspace에 전달한다.

## 기존 코드 분석과 영향 범위

- src/ui.rs 하단은 고정 32px이며 notice 왼쪽에 민트 원형, 오른쪽 인코딩은 ContextMenu를 연다. 이 블록을 status_bar로 추출하고 세 동작 버튼에 status_button을 공통 적용한다. 브랜치 배지는 정적 표시로 유지한다.
- src/dialog.rs는 모든 공용 팝업의 배경 입력·휠 차단과 중앙 배치를 담당한다. 수정 없이 재사용한다.
- src/repository_dialog.rs/file_history_dialog.rs의 엔티티 구독과 포커스 복구 패턴을 따른다. repository_busy에 인코딩 대화상자를 포함하여 기존 배경 조작 보호에 참여시킨다.
- src/text_encoding.rs의 목록·글로벌 선택과 DiffViewer 구독을 그대로 사용하므로 디코딩과 부분 쓰기는 변경하지 않는다.
- 기존 open_file_location은 백그라운드 경로 확인 후 open_with_system으로 탐색기를 연다. 폴더 확인을 src/workspace_backend.rs에 작은 함수로 추가하고 같은 Shell 호출을 재사용한다.
- 터미널은 검증된 폴더를 current_dir에 지정한 Windows PowerShell로 실행한다. 인수는 고정값이며 경로를 -Command나 셸 문자열에 삽입하지 않는다. CREATE_NEW_CONSOLE로 사용자가 요청한 대화형 창을 연다. 파일 확인·프로세스 실행은 backend::spawn 내부에서 수행한다.

## 상태와 설계

Workspace가 대화상자 엔티티/구독/이전 포커스와 폴더 작업 진행 여부를 가진다. 클릭 시 프로젝트/경로를 캡처하고 완료 시 여전히 같은 저장소일 때만 오류·탐색기 동작을 반영한다. 실행 중 폴더 버튼은 중복 실행을 막는다. 경로는 기본 안내와 분리해 표시하며 작업·오류 notice는 유지한다.

## 헌법 검사

헌법은 미작성 템플릿이므로 규범 검사는 생략한다. UI/백엔드 책임, 공개 데이터·파일 불변, 공용 컴포넌트 재사용을 유지하며 핵심 의존성·아키텍처 변경은 없다. 설계 후에도 충돌 없음.

## 검증 방법

목록의 현재값·키보드 순환·선택/취소를 검증한다. 폴더 유효성, 공백·한글·명령 특수문자 경로가 별도 작업 디렉터리로 전달되는지와 고정 인수를 검증하며 실제 프로세스는 실행하지 않는다. Workspace의 빈/새 탭·모달/진행 상태·오래된 응답 가드를 검증한다. 전체 cargo test, cargo clippy --all-targets, 변경 파일 포맷 검사와 실행 중 앱을 덮어쓰지 않는 별도 출력 빌드를 수행한다. GUI 검증 가능 여부는 별도 기록한다.

## 구조

추가 src/encoding_dialog.rs, 수정 src/main.rs/src/ui.rs/src/workspace_backend.rs. 문서는 specs/059-footer-overlay-actions에 둔다. 기존 히스토리·Diff 알고리즘·공용 Dialog 자체는 변경하지 않는다.

## 추가 설계: 커밋 배지와 현재 저장소 행

src/ui.rs의 commit_grid_row는 배지 아이콘에 텍스트와 같은 색을 전달한다. 로컬/태그일 때만 흰색을 전달하는 표시 정책을 적용하고 텍스트/배경은 보존한다. src/history.rs는 PreparedHistoryRow.references를 최초 준비 때 Rc로 캐시한다. 준비 시 종류 순위를 사용하는 안정 정렬을 적용하고 같은 종류 순서를 유지한다. 렌더마다 정렬하거나 저장소 데이터를 수정하지 않는다.

저장소 목록은 현재 이름 뒤 flex_1이 현재 배지를 오른쪽으로 민다. 이 spacer를 제거하고 아이콘/이름 그룹의 간격을 배지와 같은 4px로 적용한다. active는 기존 로컬 탭 및 project.branch 이름 일치 조건을 유지하고 해당 행의 아이콘/이름에만 ACCENT를 적용한다. 커밋 그래프/증분 조회는 변경하지 않는다.

추가 검증: 혼합 참조 종류와 같은 종류 여러 항목의 안정 순서, 캐시 재사용, 배지 아이콘 흰색과 현재 행 민트색/비현재 기본색 정책을 검증한다. 기존 준비된 참조 테스트의 기대 순서는 새 요구에 맞춰 갱신하며 종류/이름/캐시 검증은 보존한다.

## 추가 분석: 그래프 중심 어긋남

src/history_graph.rs의 선은 snap_graph_coordinate로 선 두께에 맞춘 중심을 사용한다. node_geometry는 지름을 반올림한 뒤 원의 왼쪽/위 경계를 별도로 반올림한다. 배율 1에서 중심 10과 지름 9의 원은 경계 6, 중심 10.5가 되어 선보다 오른쪽으로 이동한다. render도 원에는 보정 전 중심을 전달한다.

node_geometry가 선과 같은 중심 보정을 사용하고 해당 중심에서 반지름을 빼서 경계를 계산하도록 수정한다. 지름/확장/테두리 계산은 유지한다. 홀수 지름 원과 짝수 선의 중심을 일치시키려면 원 경계의 반 픽셀을 허용해야 한다. 기존 경계 정수 픽셀 테스트는 이 변경 근거에 따라 지름/테두리 정수 및 중심 정확 일치·경계 반 픽셀 정렬 검증으로 갱신한다. 기존 크기/간격/연결 테스트는 유지한다. 신규 중심 재현 테스트의 실패를 먼저 확인하고 수정한다.

## 추가 설계: 직선 분기

src/history_graph.rs의 segment_path는 수직선은 채운 사각형으로, 다른 레인 연결은 cubic_bezier_to로 그린다. 비수직 경로만 line_to(end)로 바꾸어 두 연결점을 직접 잇는다. 원을 그리는 circle_path의 베지어는 원 모양을 위해 유지한다. RowGraph.segments, 레인/부모 계산과 경로 캐시는 변경하지 않는다. 기존 여러 배율의 경로 생성·행 연결·레인 간격·중심 테스트를 재사용한다.

## 추가 문구 변경

커밋 상세의 부모 목록 제목을 상위 항목에서 부모 커밋으로 변경한다. src/diff_viewer.rs의 표시 문자열과 관련 기대값만 수정하며 부모 ID 데이터와 조회 경로는 보존한다.

## 후속 계획: 우측 하단 배지 크기 버튼과 행 경계 꺾임

현재 status_bar의 경로 그룹 안에 터미널/탐색기 버튼이 있고 status_button은 고정 높이 24px·12px/4px 패딩·테두리를 가진다. 버튼을 오른쪽 묶음으로 옮기고 badge_base(TEXT)를 재사용해 브랜치 배지와 패딩·글꼴·높이 계산을 공유한다. 버튼 이벤트·비활성 가드는 변경하지 않는다.

현재 RowGraph.segments는 행 상단→중앙과 중앙→하단의 시작/끝 좌표를 보유하며 segment_path가 비수직 구간을 직접 연결한다. 원본 레인 좌표와 캐시 구조를 보존하고 화면 좌표/DPI 보정 전에 꺾임 점을 계산한다. 상단 절반은 시작점→같은 x의 y+4px→중앙, 하단 절반은 중앙→끝과 같은 x의 y-4px→끝으로 그린다. 단일 PathBuilder의 연결된 선으로 꺾임 접합을 처리하고 수직선은 기존 사각형 경로를 유지한다.

변경 파일은 src/ui.rs와 src/history_graph.rs다. 경계 수직성·위/아래 반쪽·좌/우 분기·100~200% DPI·소수/음수 원점·경계 연속성·원 중심/크기 및 기존 그래프 회귀를 검증한다. 렌더마다 추가 I/O나 그래프 재계산은 없다. UI/그래프 책임과 기존 공개 형식을 보존하며 헌법 템플릿 외 규범 충돌은 없다.

## 후속 계획: Git Bash·PowerShell 구분, 좁은 곡선 그래프, 헤더 높이

src/workspace_backend.rs의 DirectoryAction::Terminal과 terminal_command는 현재 PowerShell 전용이다. 이를 GitBash/PowerShell/Explorer로 구분하고 기존 명령 빌더는 powershell_command로 이름을 명확히 한다. Git Bash는 PATH에 직접 있는 git-bash.exe, PATH의 git.exe 근처 Git 설치 루트, ProgramFiles/ProgramFiles(x86)/LOCALAPPDATA의 Git 설치 위치에서 실행 파일을 확인한다. 절대 경로 후보만 사용하며 찾지 못하면 명시적으로 안내한다. Git Bash 런처는 명령 인수 없이 current_dir로 저장소를 전달한다. 파일 조회와 실행은 기존 백그라운드 작업 경로를 유지한다.

src/ui.rs는 Git Bash·PowerShell(Windows만)·탐색기·인코딩·브랜치 순서를 사용한다. PowerShell은 cfg!(windows)로 표시를 제한하고 모든 기존 버튼 크기/가드/비동기 오류 처리를 유지한다.

src/history_graph.rs는 레인 간격을 18px로 줄인다. 기존 현재 원이 일부 배율에서 14px이므로 최소 4px 여백을 유지할 수 있는 간격이다. 4px 경계 수직 구간은 유지하고 나머지 두 점 사이를 수직 접선을 갖는 cubic_bezier_to로 잇는다. 기존 원 크기·중심·경계 좌표·캐시는 보존한다. 원/간격/경계/배율 회귀 테스트를 사용한다.

src/ui.rs의 commit_grid_header는 32px이며 메인과 파일 히스토리 목록도 헤더 인수 32를 넘긴다. 공용 헤더에 ROW_HEIGHT와 border_b_1/LINE을 적용하고 src/file_history_dialog.rs를 포함한 두 사용처의 헤더 인수를 같은 ROW_HEIGHT로 바꾼다.

추가 의존성·외부 설정 변경 없음. 설치 탐색·미설치·특수문자 경로·명령 빌더와 기존 그래프/그리드 테스트, 전체 테스트·정적 분석·포맷·별도 빌드로 검증한다. 실제 셸/GUI 실행 검증은 별도로 보고한다.

## 검증 중 확인한 영향

기본 그래프 열 폭은 graph_width(5)를 공유하므로 19→18px 변경에 따라 96→92px로 바뀐다. src/history_grid.rs와 src/ui.rs의 두 고정 기대값을 새 간격에 맞춰 갱신하고 5개 레인 수용·접기/확장·총 폭 보존 검증은 유지한다.

150%에서 원 사이 실제 간격은 6 물리 픽셀(4 논리 px)이지만 f32 논리 좌표 뺄셈 결과는 3.9999962px였다. 최소 여백 검증은 물리 픽셀로 환산해 정수 정렬 오차가 0.001px 미만인지 확인하고 최소 ceil(4*scale) 물리 픽셀을 요구하도록 정밀도를 바로잡았다. 최소 여백 요구 자체는 낮추지 않았다.

## 버그 수정 계획: 참조 이동 후 커서 동기화

호출 경로는 select_repository_row → ReferenceRequest::execute → repository::reference_snapshot → (Project, PreparedHistory) → finish_reference다. repository는 대상까지 확장한 커서와 실제 누적 길이를 반환한다. finish_reference는 현재 commits/more_commits/그래프만 바꾸므로 기존 커서가 남는다. queue_history_page는 새 목록 길이를 offset으로, 기존 history_cursor를 세션으로 사용하여 서버 위치 불일치를 유발한다.

src/ui.rs의 성공 결과 반영에 history_cursor도 같은 Project 결과에서 함께 교체한다. 새로운 조회 계층이나 재시도 우회는 추가하지 않는다. 기존 finish_history_page의 커서 정체성/오프셋 검사는 유지하여 이전 스냅샷 결과를 거절한다. install_history의 스크롤 핸들 보존과 기존 선택 흐름도 유지한다.

수정 전 실제 임시 저장소에서 초기 범위 밖 중간 참조로 이동한 뒤 추가 페이지를 읽어 오류를 재현한다. 진행 중인 기존 페이지를 뒤늦게 전달하는 경우, 끝까지 읽는 참조 이동의 커서 제거, 200개 추가/나머지/순서·유일성·선택/스크롤 보존을 검증한다. 기존 참조 이동 및 증분 조회 테스트를 보강하고 전체 회귀·정적 분석·빌드·수렴 점검한다. 수정 범위는 src/ui.rs이며 백엔드/그래프/저장 형식은 유지한다.

추가 하단 표시 요청: src/ui.rs status_bar의 마지막 브랜치 배지만 제거한다. branch_label이 더 이상 사용되지 않으면 해당 표시 전용 함수도 제거한다. 실제 프로젝트의 branch 데이터와 저장소/커밋 배지 렌더링은 변경하지 않는다.
