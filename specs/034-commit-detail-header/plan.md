# 구현 계획

## 실제 구조
Rust 2024·GPUI 앱이다. diff_viewer.rs의 헤더는 Detail에만 36px·px_5·SEMIBOLD 스타일을 적용하고 나머지 대상은 버튼 두 개가 있다. set_target은 선택 정보를 즉시 저장하지만 data를 기본값으로 초기화한다. 현재 제목은 응답의 data.title을 사용하므로 대기 중 기본 제목이 나타난다. ui.rs의 변경 파일 선택은 FileChange.label을 포함한 ViewerTarget::File을 즉시 전달한다.

## 설계와 영향
src/diff_viewer.rs에서 헤더의 대상별 조건을 제거해 모든 대상에 높이 36px, px_5, gap_3, SEMIBOLD, 아래 1px LINE 구분선을 적용하고 두 버튼을 제거한다. 제목은 응답 데이터 대신 현재 ViewerTarget의 label·상세 제목·Text 제목에서 얻는다. 기존 세대 검증과 비동기 조회는 유지한다. 백엔드 호출·새 의존성·공개 API 변경은 없다. 단축키 편집 버튼의 제거 외 설정 기능 정리는 범위에 넣지 않는다.

## 헌법 점검
헌법은 미작성 템플릿이므로 규범 검사를 건너뛴다. AGENTS.md의 최소 변경과 계층 분리를 준수한다.

## 검증
존재하지 않는 루트를 가진 선택 대상에서도 제목을 즉시 얻고 파일 전환·상세·빈 제목을 처리하는 회귀 테스트를 작성한다. 스타일만 복제하는 테스트는 추가하지 않는다. fmt/check/test/clippy/build를 실행하고 GUI 검증·기본 exe 잠금 한계를 보고한다.
