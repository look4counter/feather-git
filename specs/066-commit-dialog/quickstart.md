# 검증 안내

Commit 왼쪽 위치, 여러 줄 입력, 두 체크박스, 확인·취소와 ESC를 확인한다. staged-only 및 amend 메시지 전환·푸시 부분 실패를 임시 저장소로 검증한다. 실제 GUI/외부 계정 미검증은 별도로 보고한다.

## 검증 결과 (2026-11-15)

- `cargo test --offline`: 389개 통과, 0개 실패, 기존 3개 무시.
- `cargo clippy --offline --all-targets -- -D warnings`: 통과.
- `cargo fmt --all -- --check`: 통과. include 대상 committing.rs는 별도 rustfmt 적용.
- `cargo rustc --offline --bin feather-git -- -o target/commit-dialog-verified.exe`: 성공. 별도 출력 경로 지정과 Windows import library 생성에 따른 경고 3개가 있으며 소스 경고 검사는 통과했다.
- 격리 저장소에서 staged-only, amend, 병합, 충돌, 잠금, 로컬 bare 원격 푸시와 거부 및 부분 성공을 검증했다. 사용자 저장소에는 커밋·푸시를 실행하지 않았다.
- 실제 GUI의 TextArea 표시·한글 IME·포커스와 외부 원격 계정 인증은 미검증이다.
