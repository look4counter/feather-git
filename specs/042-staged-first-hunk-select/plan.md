# 구현 계획

## 기존 구조와 데이터 흐름
Rust 2024·GPUI 앱이다. diff_git::status는 FileChange 목록을 경로 우선·같은 경로에서 Staged 우선으로 정렬한다. workspace_backend::changed_files는 이 순서를 그대로 표시 모델로 변환하고 Workspace는 같은 인덱스로 viewer_files에서 diff 대상을 찾는다. 따라서 표시 모델만 재정렬하면 잘못된 파일이 열리므로 원본 조회 결과에서 정렬 우선순위를 변경해야 한다. load_history는 별도 경로이므로 변경하지 않는다.
DiffViewer::diff_row의 구간 Stage 버튼은 stage(Some(index))로 직접 쓰기를 실행한다. 기존 toggle은 partial·busy 검사 뒤 추가·삭제 행만 필터링하고 모두 체크되었으면 해제, 아니면 전체 체크한다. partial_stage::hunk_rows가 정확한 구간 행을 이미 제공한다.

## 구현 설계와 영향 범위
src/diff_git.rs의 status 정렬을 staged 내림차순 → path 오름차순으로 변경한다. 별도 UI 정렬이나 중복 리스트는 추가하지 않는다.
src/diff_viewer.rs의 구간 버튼 이름을 구간 선택으로 바꾸고 hunk_rows → 기존 toggle 경로로 연결한다. 기존 체크 집합 변경 부분은 순수 함수 toggle_checked_rows로 추출해 체크박스·단축키·구간 선택이 같은 로직을 사용하며 테스트할 수 있게 한다. stage의 사용하지 않게 된 구간 매개변수를 제거하고 체크 집합만 Stage하도록 단순화한다. 실제 쓰기와 인덱스 업데이트는 기존 Stage 버튼 경로를 유지한다.
수정 범위는 두 소스 파일과 그 내부 테스트다. 공개 API·데이터 형식·백엔드 경계·의존성·가로 고정 스크롤 구조는 변경하지 않는다.

## 아키텍처 검토
Constitution은 미작성 템플릿이므로 적용 가능한 규칙은 없다. 기존 조회·표시·쓰기 책임을 유지하며 새로운 정렬 계층이나 구간 분석 구현을 중복하지 않는다.

## 검증 계획
실제 임시 Git 저장소에 경로 순서와 상태 순서가 다른 변경을 만들고 정렬 회귀 테스트가 구현 전 실패하는지 확인한다. 같은 경로의 두 상태와 표시 모델 대응도 검증한다. 구간별 선택·해제·부분 체크·다른 구간 보존·잘못된 행 테스트를 추가한다. 전체 cargo test --offline, cargo fmt --check, cargo clippy --offline --all-targets, cargo build --offline를 실행한다. 실제 GUI 미검증과 실행 파일 잠금은 결과에서 구분한다.
