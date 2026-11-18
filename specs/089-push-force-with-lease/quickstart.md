# 검증 안내

cargo test --offline force_with_lease, cargo test --offline git_action_dialog, cargo test --offline git_confirmations, cargo test --offline push 및 cargo check --offline, cargo clippy --offline -- -D warnings를 실행한다.

GUI에서는 Push 확인 팝업을 열어 기본 해제와 클릭·Tab·Space 변경, 취소 후 재진입 초기화를 확인한다. 외부 원격 푸시는 수행하지 않는다.
