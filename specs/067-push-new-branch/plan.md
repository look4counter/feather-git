# 구현 계획

## 분석과 구조
Rust/GPUI 및 git2를 유지한다. repository.rs는 backend 매크로로 포함되고 모든 IO는 background executor를 통해 실행된다. 기존 committing.rs의 push_current는 자격 증명·비강제 refspec·서버 거부 콜백을 제공한다. 이를 공유 함수로 노출하되 기존 Commit의 upstream 자동 설정 정책은 유지한다. 별도 Push만 설정 없는 경우 추적 설정을 추가한다.

## 설계와 영향
repository 하위 branch_actions.rs에 push_branch와 create_branch, BranchOutcome을 추가하고 backend.rs 경계로 노출한다. Push는 committing의 전송을 재사용한다. Branch는 유효성/중복/상태/HEAD 검사 후 현재 커밋에서 force=false로 생성한다. 같은 커밋의 새 브랜치로 HEAD만 전환하여 작업 파일·인덱스를 보존한다. 전환 실패는 생성된 브랜치를 유지한 부분 성공 결과다.

branch_dialog.rs는 공용 Dialog, TextInput, form_controls를 사용한다. ui.rs는 별도 팝업 수명과 이전 포커스를 보관한다. workspace_backend.rs의 ToolbarRequest/Result는 Push 또는 CreateBranch를 실행하고 기존 refreshed_project로 상태를 재조회한다. 기존 sync_busy/sync_revision과 전역 진행·작업 차단을 공유하고 결과 소유자를 검사한다. Pull 요청 형식과 4개 옵션은 변경하지 않는다. 진행 중 팝업 렌더를 숨기고 실패하면 같은 초안을 복원한다. split_button.rs의 toolbar_action에 Push/Branch 아이콘을 추가한다.

## 제약 및 검증
새 의존성·자동 스테이지·강제 푸시·기존 브랜치 덮어쓰기 없음. 헌법은 미작성 템플릿으로 규범 검사는 생략하고 계층 분리와 호환성은 유지한다. 격리 Git 저장소/로컬 bare 원격 테스트, 결과 적용/초안/오래된 결과 검사, 전체 회귀·fmt·엄격한 clippy·별도 실행파일 빌드로 검증한다. 실제 GUI 및 외부 인증은 미검증 범위를 보고한다.
