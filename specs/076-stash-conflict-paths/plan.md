# 구현 계획

## 기존 구조와 영향 분석
Rust/GPUI/git2. stash_actions.rs의 apply_stash는 안전한 checkout과 refs/stash 잠금을 사용하지만 anyhow 문자열 오류만 반환한다. workspace_backend.rs가 오류를 문자열화하면서 구조 정보를 잃고 ui.rs는 공용 Alert로 표시한다. Alert는 메시지·확인만 제공한다. repository::panel_references가 stash@{순번}: 설명을 만들며 ui.rs의 locator 파서와 준비 설명 검사, 관련 테스트가 이 형식에 의존한다.

## 충돌 경로 설계
stash_actions.rs에 경로를 보관하는 StashApplyFailure 오류 타입을 추가해 기존 Result<()> API를 유지한다. checkout.notify_on(CONFLICT) 콜백에서 경로를 수집하고, 인덱스 충돌의 ancestor/our/their 경로를 적용 전후 읽어 합친다. 저장소 상대 PathBuf를 정렬·중복 제거한다. 콜백은 처리를 계속해 여러 경로를 수집하며 강제 checkout은 사용하지 않는다. 반환 성공이어도 미해결 인덱스 충돌이 있으면 경로와 충돌 안내를 반환한다.

repository.rs/backend.rs에서 오류 타입을 재노출한다. ToolbarResult에 conflict_paths를 추가하고 문자열화 전에 downcast로 경로를 보존한다. 기존 완료 함수의 계약은 유지하며 ui.rs는 유효한 저장소·작업 결과에 대해서만 경로를 Alert에 전달한다. 기존 상태 재조회는 유지한다.

Alert에 선택적 경로 데이터를 추가한다. 경로가 있을 때만 개수와 높이 200픽셀·축소 방지·세로 스크롤 영역을 메시지 아래 렌더링한다. 모든 항목을 보존하고 긴 경로를 생략하지 않는다. 경로 없는 기존 알림은 유지한다. 확인·Enter·Escape·포커스 복원은 그대로다.

## 표시 형식 설계
repository.rs의 순번: 설명 표시 함수를 목록 생성과 UI 설명 검사에서 공유한다. 실제 stash locator는 유지한다. ui.rs는 표시된 숫자를 파싱하고 행 순번과 일치하는지 검사한 뒤 stash@{순번}으로 변환한다. 기존 적용·삭제의 객체 검증은 바꾸지 않는다. 표시 형식에 의존하는 테스트만 새 요구사항에 맞게 갱신하고 기존 보호 검증을 유지한다.

## 제약·검증·헌법
변경: src/stash_actions.rs, src/repository.rs, src/backend.rs, src/workspace_backend.rs, src/ui.rs, src/alert.rs. 새 의존성·계층 변경·기존 적용 API 파괴 없음. 내부 작업 결과의 새 필드는 테스트 생성자도 갱신한다. 헌법은 미작성 템플릿으로 유효 규범 검사는 생략한다. AGENTS.md의 기존 변경 보존·재사용·최소 변경·한글 문서를 준수한다.

격리 저장소에서 여러 dirty 충돌 경로·인덱스 충돌·stash 보존·경로 중복 제거·새 목록으로 두 번째 항목 적용/삭제를 검증한다. Alert 데이터 테스트로 많은 경로 보존·일반 오류의 빈 목록을 확인하고 실제 GUI 고정 높이·스크롤은 미검증 시 명시한다. fmt·전체 test·check·엄격한 clippy·별도 빌드 후 수렴 점검한다.
