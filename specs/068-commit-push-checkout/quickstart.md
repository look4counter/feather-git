# 검증 안내

Commit 빈 입력 확인과 상태 오류 안내, 동일/뒤처진 원격 Push 무시, 목록 단일클릭 탐색·더블클릭 체크아웃을 확인한다. 격리 fixture에서 backend 테스트와 UI 라우팅/결과 테스트를 수행하고 전체 cargo test --offline, fmt, 엄격한 Clippy, 별도 출력 빌드를 확인한다. GUI와 외부 원격 인증 제한은 별도 기록한다.

## 검증 결과 (2026-11-15)

- branch_actions 핵심 테스트 8개 통과. 기존 Push 테스트는 behind-only를 no-op으로 갱신하고 실제 분기된 커밋은 계속 거부함을 확인했다.
- 추가된 체크아웃 라우팅·HEAD/파일 재조회 UI/작업 테스트 3개 통과. 기존 Commit 백엔드의 빈 메시지·스테이지 없음 검증은 유지한다.
- 전체 `cargo test --offline`: 404개 통과, 실패 0개, 기존 무시 3개.
- `cargo fmt --all -- --check`, include 파일 별도 rustfmt 검사, `cargo clippy --offline --all-targets -- -D warnings`: 통과.
- `cargo rustc --offline --bin feather-git -- -o target/checkout-actions-verified.exe`: 성공. 별도 출력 경로 및 Windows import library 생성에 따른 경고 3개가 있다.
- 실제 사용자 저장소에 커밋·푸시·체크아웃을 실행하지 않았다. 실제 GUI 더블클릭·IME·포커스 및 외부 인증은 미검증이다. 파일 갱신 중 디스크 IO 등 예외의 자동 원복은 제공하지 않는다.

## 수렴 점검

FR-001~006과 사용자 이야기 3개, 계획의 기존 UI/IO 계층 재사용·안전 checkout·원격 실제 상태 비교·비동기 갱신을 구현과 대조했다. 누락·부분 구현·상충·범위 외 추가는 0건이며 새 구현 작업은 없다. 헌법은 미작성 템플릿으로 규범 검사를 생략했다. 위에 명시한 GUI/외부 인증 미검증 제한은 유지한다.
