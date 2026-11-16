# 구현 계획

## 기존 흐름 분석

Workspace.focusable_panel의 Right 핸들러는 패널 전체에서 메뉴를 열며 open_files_menu는 작업 트리만 허용한다. files의 multi_virtual_list 내부 파일 콘텐츠 행에는 우클릭 처리가 없다. 커밋 기록 목록의 0행은 커밋상세일 수 있고 실제 FileChange는 detail 오프셋 뒤에 있다. 기존 컨텍스트 메뉴는 항목/배지/키 매칭과 포커스 복원을 제공한다.

## 행과 대상 설계

src/ui.rs에서 파일 행 콘텐츠에만 Right 핸들러를 연결하고 패널 전체 Right 처리를 제거한다. 이벤트에 프로젝트·패널 세대·탭·행을 캡처하고 상태 검증 뒤 우클릭 행의 원본 FileChange를 얻는다. 커밋상세 행은 핸들러가 없으며 오래된/유효하지 않은 행은 거부한다. 우클릭한 행이 미선택이면 기존 일반 클릭 API로 단일 선택하고, 선택된 행이면 다중 선택을 유지한다.

공통 열기/위치/복사를 먼저 구성하고 작업 트리에서만 기존 스테이지/폐기를 덧붙인다. 열기/위치는 클릭한 행의 경로를 캡처한다. 복사는 현재 선택 집합의 모든 원본 경로를 사용한다. 키보드 Ctrl+Shift+C는 같은 파일 패널 포커스에서 현재 선택으로 같은 복사 메서드를 사용한다. 단축키의 다른 수정키 조합은 거부한다.

## OS와 경로 설계

src/path_display.rs에 저장소 절대경로와 Git 원본 상대경로를 안전하게 결합하는 순수 함수를 추가한다. 존재 여부로 canonicalize하지 않아 삭제/과거 파일도 복사 가능하며 표시 경로는 기존 display_path로 Windows 확장 접두사만 정리한다.

src/workspace_backend.rs의 준비 함수가 작업 스레드에서 경로 존재 여부와 위치 보기를 위한 가장 가까운 기존 상위 폴더를 확인한다. src/ui.rs는 결과를 받은 뒤 현재 요청 문맥을 재검증하고 GPUI App.open_with_system으로 파일 또는 위치 폴더를 연다. 해당 API의 Windows 구현은 OS 기본 앱/탐색기 호출을 비동기로 수행한다. 새 셸 명령이나 의존성은 추가하지 않는다. 복사는 App.write_to_clipboard를 사용한다. OS 실행 API의 반환형은 없으므로 실행 자체 실패는 기존 플랫폼 로그에 남고 준비 단계의 파일/접근 오류는 사용자 알림으로 표시한다.

## 검증과 아키텍처

행 오프셋·두 탭·클릭 대상/복사 선택 분리·중복·빈 선택·오래된 세대/모드·Ctrl+Shift+C 매칭을 UI 상태 테스트로 검증한다. 경로 결합과 누락 파일의 상위 폴더 fallback은 임시 파일로 검증한다. 실제 OS 앱은 테스트 중 열지 않는다. cargo fmt --check, cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행한다. GUI/OS 통합과 실행 파일 잠금 한계를 별도 기록한다. Constitution은 미작성 템플릿으로 구체 규칙 검사는 생략한다.

## 후속 원인 분석과 수정 계획

기존 Windows reveal_path는 작업 스레드에서 SHGetDesktopFolder/ParseDisplayName/SHOpenFolderAndSelectItems를 호출하며 실패는 로그로만 기록한다. 앱은 canonical 저장소의 Windows 확장 경로를 그대로 전달한다. 이 경로는 Explorer Shell에 맞는 일반 경로로 변환하지 않고, 파일 선택 COM 경로는 이 기능에 불필요한 의존성을 갖는다. 실제 실패 HRESULT는 GUI 로그에서 수집하지 못했으므로 특정 시스템 오류를 단정하지 않는다.

prepare_file_location은 탐색기 요청 시 기존 파일 여부와 무관하게 가장 가까운 기존 부모 폴더를 반환한다. UI는 OS 경계에서 display_path로 확장 접두사를 제거하고 open_with_system을 사용한다. 실제 파일 상태 확인에는 원본 경로를 계속 사용한다. 결과의 reveal 플래그는 불필요하므로 제거한다.

공통 메뉴는 기존 min-width만 있고 이름의 flex 기본 최소 너비와 단축키 축소가 허용되어 텍스트가 침범할 수 있다. src/context_menu.rs에서 충분한 명시적 너비를 주고 이름은 min_w_0/truncate, 단축키는 flex_shrink_0/whitespace_nowrap으로 분리한다. 단순 스타일을 복제하는 테스트는 추가하지 않는다. 기존 폴더 준비 테스트 기대를 먼저 갱신해 실패를 재현하고 수정 후 관련/전체 회귀·서식·Clippy를 확인한다.

## 후속 구분선 계획

공통 메뉴는 MenuItem 목록을 순회하며 인덱스로 키 탐색/실행한다. 별도 비실행 항목을 목록에 삽입하는 대신 MenuItem.separator_after 표시 속성을 추가하고 항목 아래에 장식 선을 렌더한다. 뒤 항목이 있을 때만 그리므로 키 탐색 인덱스는 바꾸지 않는다. src/ui.rs의 경로복사 항목에 해당 표시를 지정한다. 서식·Clippy로 검증하며 단순 장식을 복제하는 테스트는 추가하지 않는다.

## 후속 선택·집계 설계

open_files_menu는 현재 file_menu_row/뷰어 busy를 검사한 후 바로 메뉴를 만든다. 그 사이 prepare_file_menu_selection을 추가하여 유효성 검사 뒤 미선택 행만 기존 select_file_list_row_with_modifiers의 기본 수정키로 선택한다. 커밋상세 오프셋은 기존 매핑을 재사용하고 같은 행/선택 집합은 건드리지 않는다. 선택이 바뀌면 notify한 뒤 메뉴 활성/복사 대상을 계산한다.

현재 헤더는 changed_files의 len을 표시한다. ChangedFile.status의 S· 접두사를 분리해 staged 수와 M/D/A 수를 동시에 집계하는 순수 함수를 추가한다. 전체는 기존 행 수를 유지한다. 작업 트리 헤더에는 전체/Staged/M/D/A, 커밋 기록에는 전체/M/D/A를 표시한다. src/ui.rs와 관련 테스트만 변경하며 Git 조회를 추가하지 않는다. 기존 보호·수정키/범위·메뉴 대상/복사·빈/다른 상태 집계를 검증한다.

## 후속 개수 표시 원복 계획

최신 지시에 따라 src/ui.rs의 FileCounts·files_heading 및 세부 집계 전용 테스트를 제거하고 기존 heading(변경 파일, files.len()) 호출로 복구한다. 전체 개수/헤더 높이·여백은 기존 공통 구현을 재사용한다. prepare_file_menu_selection과 우클릭 테스트는 유지한다. 서식·관련 우클릭 테스트·Clippy로 검증하며 단순 원복의 전체 회귀/배포 빌드는 반복하지 않는다.

## 후속 상태 색상 수정 계획

files 렌더는 표시 상태를 A와 직접 비교하므로 S·A는 다른 색상으로 바뀌며 D도 다른 일반 상태와 같은 황색이다. src/ui.rs의 상태 색상 함수에서 S· 접두사만 제거하여 A=ACCENT, D=0xffa7b2, 그 외=기존 0xe6bc73을 반환하고 기존 상태 글자에 적용한다. 표시 문자열·정렬·Git 조회/쓰기는 변경하지 않는다. A/D/M 등과 Staged 쌍의 색상 회귀 테스트·서식·Clippy로 검증한다.
