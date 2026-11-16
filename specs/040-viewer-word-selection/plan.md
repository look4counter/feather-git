# 구현 계획

## 기존 구조와 데이터 흐름
Rust 2024·GPUI 앱이다. src/ui.rs의 history 목록은 select_history_row → select_commit → refresh_file_data로 이어지며 매번 조회를 갱신하고 파일 선택을 초기화한다. 참조 이동은 select_commit(..., false)를 직접 사용하므로 기록 행 재선택 방지는 select_history_row 입구에 둔다. file_history가 거짓이면 최초 기록 진입이므로 기존 전환을 유지한다.
src/diff_viewer.rs의 selectable_text는 커밋 상세와 diff 공통 본문이다. 캔버스에서 closest_index_for_x로 커서 경계를 찾고 original_column으로 탭 확장 좌표를 원문 문자 위치로 변환한다. Selection은 문자 인덱스를 사용하며 text()가 복사 문자열을 생성한다. 드래그는 dragging 참일 때 커서를 덮어쓴다.

## 변경 설계
select_history_row에서 같은 프로젝트·커밋이며 기록 모드이면 false를 반환해 기존 작업을 시작하지 않는다. 현재 모드가 작업 트리이면 같은 인덱스도 최초 진입을 허용한다.
src/text_selection.rs의 Selection에 select_word를 추가한다. 유니코드 영숫자·밑줄은 연속 범위, 공백은 연속 범위, 기호는 한 글자로 선택한다. 빈 줄과 범위 밖은 줄 끝의 빈 선택이다.
더블클릭은 GPUI click_count == 2로 판별한다. index_for_x로 글자의 왼쪽 경계를 찾고 탭 확장을 원문 포함 문자 위치로 변환한다. 단어 선택 후 dragging을 끄고 미세 이동에 따른 선택 축소를 방지한다. 일반 클릭은 기존 가장 가까운 커서 경계와 Shift 처리를 유지한다.

## 영향 범위와 아키텍처 검토
src/ui.rs, src/text_selection.rs, src/diff_viewer.rs 및 각 파일의 기존 테스트만 변경한다. 공개 데이터·백엔드·의존성·포커스 구조는 유지한다. Constitution은 미작성 템플릿이므로 적용 규칙이 없다. 기존 재선택 시 상세 초기화를 요구한 테스트는 변경된 요구사항에 맞춰 서로 다른 행 이동과 같은 행 보존을 구분한다.

## 검증 계획
구현 전 재선택 보존과 단어 경계 테스트를 추가해 실패를 확인한다. 구현 후 전체 cargo test --offline, cargo fmt --check, cargo clippy --offline --all-targets, cargo build --offline를 실행한다. 실행 파일 잠금 등 환경 제한은 별도 보고하고 GUI 동작을 검증했다고 주장하지 않는다.
