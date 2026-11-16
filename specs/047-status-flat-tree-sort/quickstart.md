# 검증 안내

1. cargo test --offline로 실제 저장소 정렬과 경로 비교 및 기존 회귀 테스트를 확인한다.
2. cargo fmt --check, cargo clippy --offline --all-targets, cargo build --offline를 실행한다.
3. 실제 작업 트리에서 Staged 그룹이 먼저 나오고 각 그룹이 U·M·D·A 순인지 확인한다.
4. 같은 상태의 index.html → backend/z.rs → backend/src/gemini.rs 순서와 각 행의 diff를 확인한다. 커밋 기록 파일 순서는 기존대로여야 한다.

2026-11-16 충돌 우선 갱신: 각 그룹에서 U → M → D → A → 기타 순서를 확인한다. 자동 검증 및 수동 검증 한계는 validation.md의 최신 항목을 참고한다.
