# 검증 안내

cargo run -- --backend-delay-ms 1500으로 커밋·프로젝트·참조·패널을 빠르게 선택한다. 행 강조가 즉시 바뀌고 늦은 응답이 최신 선택을 덮지 않아야 한다. 히스토리 파일 목록의 150ms 지연 안내 외에 새 로딩 표시는 나타나지 않아야 한다. 정상 UI 경로에서 UI 스레드 동기 호출 로그가 없어야 한다.
단축키 설정과 저장·최근 목록·앱 종료 후 재실행도 확인한다. 정상 종료 후 세션 저장이 끝날 때까지 프로세스가 유한하게 대기할 수 있다.
cargo test --offline, cargo fmt --check, cargo check --offline, cargo clippy --offline --all-targets, cargo build --offline로 자동 검증한다.

cargo run -- --backend-delay-ms 1500으로 커밋 행을 선택하면 변경 파일 히스토리에 150ms 뒤 ‘조회 중 입니다..’가 보여야 한다. 완료 후 사라지고 목록/오류로 바뀌어야 한다. 빠른 응답에는 문구가 보이지 않아야 하며 빠른 재선택·작업 트리 전환·프로젝트 닫기 뒤 이전 문구가 남으면 안 된다. 지연값은 src/loading.rs의 LOADING_DELAY 한 곳에서 관리한다.
