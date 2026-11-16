# 구현 계획

## 기존 구조와 분석

Rust·GPUI에서 virtual_list 콜백은 Workspace.select_repository_row와 select_history_row의 반환값이 true일 때만 notify한다. 저장소 선택은 SingleSelection.select 반환을 무시해 동일 행도 참조 조회 취소와 세대 증가, focus_reference_commit → select_commit → refresh_file_data를 수행한다. 커밋 기록의 동일 행 가드는 file_history 조건이 있어 작업 트리 모드에서는 재조회가 발생한다.

## 구현

src/ui.rs의 저장소 선택에서 유효성 검사 뒤 SingleSelection.select가 false이면 즉시 반환한다. 기존 조회·선택 흐름 앞에서 종료하여 비동기 요청도 보존한다. 커밋 기록 가드에서 file_history 조건만 제거한다. UI·비즈니스·Git 계층 책임과 공개 형식·의존성을 바꾸지 않는다. 다른 행 선택 시 불필요 조회 최적화는 별도 범위다.

## 검증과 영향

저장소 네 탭의 재선택을 조회 대기/완료 상태에서 검증하고 기존 요청·세대·본문·선택·스크롤이 바뀌지 않는지 검사한다. 작업 트리 모드 커밋 재선택과 다른 커밋 선택도 검증한다. 기존 테스트 중 현재 커밋 0 클릭을 초기 조회 수단으로 쓰던 테스트는 다른 커밋 선택 상태를 준비하여 원래 검증 목적을 유지한다. 반복 선택으로 다른 커밋 복귀를 기대하던 테스트는 새 요구사항의 무동작을 명시적으로 검증한다.

cargo fmt --check, cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행한다. 실제 GUI 검증과 실행 파일 잠금 제한을 별도 명시한다. Constitution은 미작성 템플릿으로 구체 원칙 검사는 적용하지 않는다.
