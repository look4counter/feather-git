# 검증 안내

1. cargo test --offline로 백엔드 실행기·지연·조회·저장 회귀 테스트를 실행한다.
2. cargo fmt --check, cargo check --offline, cargo clippy --offline --all-targets로 정적 검증한다.
3. cargo run --offline -- --backend-delay-ms 1500으로 커밋·참조·파일을 빠르게 선택하고 마지막 결과가 표시되는지 확인한다.
4. 조회 중 프로젝트 닫기·시작 탭 전환을 하고 이전 결과가 나타나지 않는지 확인한다.
5. 작업 관리자에서 CPU·메모리·스레드 수와 디스크 사용을 0/1500 각각 비교하고 다른 프로그램 반응성을 확인한다. 실제 PC 측정 없이 무영향으로 단정하지 않는다.
