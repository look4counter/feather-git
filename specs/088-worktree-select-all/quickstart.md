# 검증 안내

관련 자동 테스트는 cargo test --offline select_all 및 cargo test --offline worktree_modifier_selection으로 실행한다. cargo check --offline과 cargo clippy --offline -- -D warnings도 실행한다.

실제 앱에서는 변경 파일 목록 클릭 후 Ctrl+A로 모든 행 선택과 다중 파일 차이를 확인한다. 반복·빈 목록·히스토리 전환·다른 패널 및 검색 입력란에서 Ctrl+A를 확인한다. 입력란에서는 텍스트만 선택되어야 한다.
