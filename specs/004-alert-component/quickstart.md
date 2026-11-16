# 실행 및 검증

1. cargo fmt --check, cargo check --offline, cargo test --offline을 실행한다.
2. cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-alert.exe로 빌드한다.
3. + → Create로 팝업 제목·메시지·확인 버튼을 확인한다.
4. 확인·Enter·Esc로 각각 닫고 다시 연다. 배경 클릭은 팝업을 닫거나 프로젝트를 바꾸지 않아야 한다.
5. 팝업의 줄바꿈/가용 크기와 포커스 이동·복원을 확인한다. 실제 조작하지 못한 경우 미검증으로 기록한다.
