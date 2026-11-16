# 실행 및 검증

1. cargo fmt --check, cargo check --offline, cargo test --offline을 실행한다.
2. cargo tree --offline -i gpui로 vendor/gpui의 0.2.2 사용을 확인한다.
3. cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-ime.exe로 빌드한다.
4. + → Create/Open/Clone 입력에서 러 → 한/영 전환 → l로 러l을 확인한다. 쿼리ID, 한글·영문 왕복, 삭제·방향키·선택 교체 및 붙여넣기도 확인한다.
5. 실제 입력을 실행하지 못하면 자동화 통과와 구분해 기록한다.
