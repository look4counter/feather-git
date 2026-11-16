# 구현 계획

## 기존 흐름 분석

Rust·GPUI 앱이다. Workspace.files_key_down이 파일 배치 Stage를 비동기로 실행하며 selected_stage_requests가 상태를 검증한다. DiffViewer.toggle이 현재 범위의 변경 줄 체크를 처리한다. footer_shortcut과 DiffViewer.shortcut_hint/selection_shortcut이 하단 배지 안내를 구성한다. 빈 대상은 Workspace.viewer_target의 Text이며 제목이 Diff Viewer다. DiffViewer.render_files는 파일별 자식 엔티티를 재사용한다.

## 공통 메뉴 설계

src/context_menu.rs에 항목/배지/활성 상태/실행 콜백을 가진 공통 메뉴 엔티티를 만든다. 앱 내부 전역에는 열린 메뉴 엔티티 하나만 보관하고 Workspace 최상단에 overlay로 렌더한다. GPUI anchored/deferred로 창 좌표에 배치하고 경계를 보정한다. 메뉴 포커스와 이전 포커스를 보관하고 Escape/외부 클릭으로 닫는다. Enter/Space로 활성 항목을 실행하고 방향키로 이동한다. 창 비활성화·부모 모달/프로젝트 화면 이탈 시 닫는다. 메뉴 렌더 책임과 Git/체크 동작은 분리한다. 새 외부 의존성이나 공개 형식 변경은 없다.

## 연결과 보호

src/ui.rs에서 파일 Stage 실행 메서드를 추출하여 Space와 메뉴가 공유한다. 작업 트리 패널의 우클릭은 선택을 바꾸지 않고 포커스를 가져온다. 메뉴 콜백은 프로젝트/패널 세대를 확인한 뒤 기존 선택/쓰기 가드로 재검증한다. 실행 시 viewer busy도 검사한다. 기존 Space는 포커스/활성 창 조건을 유지한다.

src/diff_viewer.rs에서 우클릭 당시 대상 세대와 선택 범위를 캡처하고 실행 시 세대·상호작용·로딩/쓰기 상태를 다시 검증한다. 변경 줄이 있고 실행 가능한 범위에서만 메뉴를 연다. 체크 불가이면 빈 메뉴도 열지 않고 부모에 우클릭 이벤트를 전파하지 않는다. 자식 패널에서 우클릭 이벤트 전파를 막아 다중 파일의 다른 파일이나 부모 메뉴로 대체되지 않게 한다. 우클릭으로 본문 선택을 이동하지 않는다. 기존 toggle을 재사용한다.

footer_shortcut 렌더와 이제 불필요한 안내 전용 메서드/테스트를 제거한다. 기존 체크 알고리즘 테스트는 보존한다. 빈 헤더와 구간 체크 문구를 변경한다.

## 검증과 아키텍처

메뉴 상태의 활성/비활성 실행 선택, 빈 헤더, Diff 범위 체크 가능 조건과 오래된 대상 방어를 테스트한다. 기존 다중 Stage/체크 테스트 및 전체 회귀를 실행한다. cargo fmt --check, cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline를 수행한다. 실행 파일 잠금과 GUI 미검증 범위를 기록한다. Constitution은 미작성 템플릿이므로 구체 규칙 검사는 생략한다.

## 후속 버튼명 변경 계획

stage_button_label이 기본/사용자 지정 단축키의 버튼명을 생성하며 render에서 해당 함수를 사용한다. src/diff_viewer.rs의 문구와 기존 버튼명 회귀 테스트 기대값만 변경한다. Stage·체크 데이터 흐름과 의존성은 유지한다. 서식·해당 테스트·Clippy를 확인하고 빌드 교체 제한을 기록한다.

## 후속 체크 메뉴 표시 조건 분석

open_check_menu는 기존 check_menu_rows 결과를 enabled로 넘겨 비활성 항목도 표시한다. 기존 조건을 재사용하여 결과가 없으면 context_menu::open 호출 전에 종료하고 이벤트 전파를 막는다. 다중 파일 부모에서 메뉴가 다시 열리는 것을 방지한다. 체크 알고리즘과 현재 선택/포커스·실행 시 세대 재검증은 유지한다. 기존 범위/비활성/오래된 대상 조건 테스트와 서식·Clippy를 실행한다.

## 후속 파일 메뉴 이름 변경 계획

src/ui.rs의 open_files_menu가 MenuItem 제목을 직접 전달한다. 해당 문자열 하나만 선택한 파일 스테이지로 변경한다. 콜백·Space 배지·배치 Stage·비활성 조건은 유지한다. 서식과 Clippy로 검증한다. 단순 문구를 복제하는 테스트와 전체 회귀/빌드는 반복하지 않는다.
