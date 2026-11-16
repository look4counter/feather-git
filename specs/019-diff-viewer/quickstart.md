# 검증 가이드

cargo test·cargo check·cargo fmt --check·cargo clippy --all-targets를 실행한다. cargo rustc --bin feather-git -- -o target/feather-git-diff-viewer.exe로 별도 출력 빌드한다.

임시 저장소에서 staged/unstaged가 같은 파일에 있는 경우 두 행을 선택한다. 히스토리 파일 및 고정 상세 행을 선택한다. 긴 줄의 가로 이동·텍스트 드래그/복사·키보드/설정·줄 체크/Stage·구간 Stage·충돌3종 선택/전체 Stage를 확인한다. 실제 사용자의 저장소는 검증 목적으로 변경하지 않는다.
