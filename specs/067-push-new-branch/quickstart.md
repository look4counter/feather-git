# 검증 안내

격리 저장소에서 push_branch·create_branch 테스트를 실행하고 전체 cargo test --offline, cargo fmt --all -- --check, cargo clippy --offline --all-targets -- -D warnings 및 별도 출력 빌드를 확인한다. 실제 화면에서 버튼 순서와 New Branch의 공용 입력·체크박스·확인/취소·ESC/Tab을 확인한다. 실제 계정/원격은 자동 검증에서 사용하지 않는다.

## 검증 결과 (2026-11-15)

- 추가된 핵심 테스트 8개 통과: Git 동작 4개, UI 결과 처리 3개, 백그라운드 재조회 통합 1개.
- `cargo test --offline`: 397개 통과, 실패 없음, 기존 3개 무시.
- `cargo fmt --all -- --check`와 include 대상 `rustfmt --edition 2024 --check src/branch_actions.rs src/committing.rs`: 통과.
- `cargo clippy --offline --all-targets -- -D warnings`: 통과.
- `cargo rustc --offline --bin feather-git -- -o target/push-branch-verified.exe`: 성공. 별도 출력 경로 지정 및 Windows import library 생성 안내에 따른 경고 3개가 있다.
- 실제 GUI 키보드·포커스·레이아웃과 외부 원격 인증은 미검증이다. 개발 중 실제 사용자 저장소의 커밋·푸시·브랜치 생성은 실행하지 않았다.

## 수렴 점검

요구사항 FR-001~006, 사용자 이야기 2개 및 계획의 전송 재사용·계층 경계·파일 보존·부분 성공·비동기 진행/갱신을 실제 코드와 대조했다. 미구현·부분 구현·상충·범위 외 추가 발견은 0건이다. 헌법은 미작성 템플릿으로 규범 검사를 생략했다. 추가 구현 작업은 없으며 위 GUI/외부 인증 미검증 제한은 유지한다.
