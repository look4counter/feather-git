# 구현 계획

## 기존 구조와 영향 분석
Rust/GPUI, git2, backend 매크로·background executor 경계를 유지한다. CommitDialog의 can_confirm은 상태·메시지·스테이지 조건으로 버튼을 제한하고 있다. UI의 참조 목록은 virtual_list의 단일클릭을 사용해 기록만 탐색한다. 이미 multi_virtual_list가 MouseDownEvent를 넘기므로 공용 목록 API를 중복 추가하지 않고 재사용한다. ToolbarAction/Request/Result와 sync_busy/revision·전역 진행·재조회·오래된 결과 검사를 체크아웃에도 재사용한다.

## 설계
1. commit_dialog.rs에서 확인 버튼의 enabled를 항상 true로 하고 확인 이벤트에서 busy/IME를 막는다. 준비 중/준비 실패는 inline 안내하며 draft가 있으면 backend 검증을 수행한다. 기존 staged-only와 메시지 검증은 committing.rs에 남긴다.
2. branch_actions.rs의 Push는 unborn HEAD를 무시한다. 인증 콜백으로 대상 원격 참조를 조회하고 local/remote ahead-behind로 보낼 커밋을 검사한다. 원격 tip 객체가 없으면 해당 참조의 객체만 download하며 refs·FETCH_HEAD는 갱신하지 않는다. 동일하거나 behind-only면 전송을 생략하고 실제 오류는 유지한다. Commit의 즉시 푸시 정책은 변경하지 않는다.
3. branch_actions.rs에 checkout_reference를 추가하고 backend.rs/repository.rs로 노출한다. refs/heads·remotes·tags만 허용하고 커밋으로 peel한다. 작업 진행 상태를 검사하고 HEAD 및 로컬 대상 참조를 잠근 뒤 CheckoutBuilder::safe로 작업 파일/인덱스를 갱신한다. 로컬은 symbolic HEAD, 원격/태그는 direct HEAD를 transaction으로 저장한다. 후속 참조 저장 실패는 파일 갱신 사실을 포함해 알린다.
4. workspace_backend.rs에 ToolbarAction::Checkout {reference}를 추가한다. ui.rs의 branches만 multi_virtual_list로 전환하며 왼쪽 2회 클릭은 검증된 정확한 참조로 체크아웃한다. 단일클릭은 기존 select_repository_row를 유지한다. 타이틀바 제외 진행 표시·요청 무효화·소유자 검증·완료 재조회를 공유한다.

## 불변 조건
새 의존성 없음. 기존 4개 Pull 옵션·Branch 생성·커밋 실행 계약을 유지한다. 자동 로컬 브랜치 생성·강제 checkout·강제 push 없음. 헌법은 미작성 템플릿으로 규범 검사는 생략한다. 실제 사용자 저장소에는 Git 변경을 실행하지 않는다.

## 검증
임시 저장소와 로컬 bare 원격에서 초기/동일/뒤처짐 Push no-op과 분기 거부·누락 원격 오류를 검증한다. 로컬/원격/일반 및 주석 태그 checkout과 인덱스·작업 파일 반영, 미커밋 충돌·미추적 충돌 보호, 참조/잠금/진행 상태 오류를 검증한다. 클릭 라우팅·Commit 클릭 검증과 background 결과 갱신을 검사하고 전체 회귀·fmt·엄격한 Clippy·별도 빌드 및 수렴 점검을 수행한다. 실제 GUI와 외부 인증은 미검증 범위를 보고한다.

## 검증 중 확인한 사항
libgit2 checkout_tree는 index.lock이 이미 존재해도 파일을 먼저 갱신한 뒤 index 저장에서 실패할 수 있었다. 재현 테스트에 따라 파일 갱신 전 index.lock 상태를 확인하며, HEAD/대상 브랜치 잠금은 참조 transaction으로 확보한다. 디스크 IO 등 파일 갱신 후의 실패까지 원자적으로 되돌리는 기능은 추가하지 않는다.
