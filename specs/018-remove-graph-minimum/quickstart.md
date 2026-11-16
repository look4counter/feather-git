# 검증 가이드

cargo test, cargo check, cargo fmt --check, cargo clippy --all-targets를 실행한다. 사용 중인 실행 파일을 덮지 않도록 cargo rustc --bin feather-git -- -o target/feather-git-graph-width.exe로 빌드한다.
수동 검증은 많은 레인의 저장소에서 초기 폭 100·축소·0에서 재확장·그림 잘림·전체 너비 보존을 확인한다. 실행하지 못하면 결과에 명시한다.
