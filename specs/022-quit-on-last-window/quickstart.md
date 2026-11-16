# 검증 안내

cargo test --offline, cargo fmt --check, cargo check --offline, cargo build --offline를 실행한다.
임시 실행 검증은 target/lifecycle-check의 기존 앱 코드 사본에 다음 프레임 창 제거와 종료 마커만 주입해 실행한다. 제품 코드나 사용자 설정을 변경하지 않는다.
직접 검증 시 cargo run으로 앱을 열고 마지막 창을 닫은 뒤 프롬프트가 반환되는지 확인한다. Get-Process feather-git으로 프로세스 상태를 확인하고 다시 cargo run을 실행한다. 실제 사용자의 앱을 임의로 강제 종료하지 않는다.
