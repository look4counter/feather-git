# 검증 안내

1. cargo test --offline text_layout_cache로 폭 변경 회귀를 확인한다. 테스트는 실제 공급 GPUI의 판정 소스를 포함한다.
2. 전체 cargo test --offline와 cargo fmt --check, cargo check --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행한다.
3. 새 빌드의 최근 저장소 이름과 경로가 충분한 폭에서 보이는지 확인하고 창을 좁혔다 넓혀 잘린 텍스트가 복원되는지 확인한다.
4. 이번 세션에서는 기존 화면 접근 승인 거절을 우회하지 않으므로 실제 GUI 확인과 소스/자동 검증을 구분하여 보고한다.
