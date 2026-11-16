# 검증 안내

cargo run -- --help로 옵션을 확인한다. cargo run -- --backend-delay-ms 1500으로 시작·저장소 열기·최근 저장소·커밋/참조 선택·diff·부분 스테이징·설정 저장을 확인한다. UI 스레드 진단 로그가 나오며 화면이 멈추면 해당 동기 호출이 드러난 것이다. 이 옵션은 자동으로 비동기화를 수행하지 않는다.
cargo run -- --backend-delay-ms 0 또는 일반 cargo run으로 원래 속도로 실행한다.
cargo test --offline, cargo fmt --check, cargo check --offline, cargo clippy --offline --all-targets, cargo build --offline로 자동 검증한다. 테스트 정책은 스레드 지역으로 주입해 기존 테스트에 전파하지 않는다.
