# 검증 안내

1. cargo test --offline clean_worktree로 stat 갱신만 있는 재현을 확인한다.
2. FEATHER_VERIFY_CLEAN_REPO를 대상 저장소 경로로 설정하고 cargo test --offline external_clean_repository -- --ignored --nocapture로 읽기 전용 검증을 실행한다.
3. Git은 --no-optional-locks status --porcelain으로 비교한다. 사용자 저장소 설정과 인덱스를 변경하지 않는다.
4. 전체 테스트·서식·Clippy·빌드를 실행한다. 실행 파일을 교체하지 못하면 실제 화면은 미검증으로 보고한다.
