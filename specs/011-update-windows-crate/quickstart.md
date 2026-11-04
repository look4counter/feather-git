# 검증 안내

Rust 및 Windows 빌드 도구가 필요하다.

1. Cargo.toml과 Cargo.lock에서 직접 windows 의존성 0.62.2를 확인한다.
2. 저장소 루트에서 cargo check --locked --all-targets를 실행한다.
3. 컴파일 오류가 없고 기존 기능 플래그가 유지되는지 확인한다.
4. git diff로 의존성 관련 변경만 추가되었는지 확인한다.

## 검증 결과

2026-11-05 Windows 환경에서 cargo check --offline --all-targets로 잠금 파일을 갱신한 후 cargo check --offline --locked --all-targets가 성공했다. 직접 windows 의존성은 0.62.2이며 기존 기능 플래그를 보존했다. 호출부 호환성 수정은 필요하지 않았다.

## 수렴 점검

기능 요구사항 3개, 성공 기준 2개, 인수 시나리오 2개와 계획의 직접 의존성 갱신·전이 버전 공존·기능 유지 결정을 확인했다. 헌장은 미작성 템플릿으로 추가 원칙 검사를 생략했다. 누락·부분 구현·모순·범위 밖 추가 변경은 없으며 남은 작업은 0개이다. 실제 UI 실행 검증은 수행하지 않았다.
