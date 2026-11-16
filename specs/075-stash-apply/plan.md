# 구현 계획

## 기존 구조 분석
Rust/GPUI/git2. ui.rs의 open_checkout_menu는 저장소 탭과 커밋 메뉴를 공유하고 스태시는 삭제만 제공한다. repository_delete_target은 저장소·탭·행·선택과 stash locator를 검증한다. prepare_reference_deletion은 참조·객체 ID·스태시 reflog 설명을 읽어 반환한다. stash_actions.rs에는 생성만 있고 적용은 없다. ToolbarAction/Request/Result는 백그라운드 실행·전역 진행·오류·성공 및 실패 후 snapshot 재조회를 공유한다.

## 설계와 영향 범위
기존 스태시 스냅샷을 재사용한다. ui.rs에 스태시 탭 전용 대상 조회를 추가하고 적용 메뉴에서 프로젝트·경로·선택·표시 이름을 캡처한다. 비동기 준비 결과의 세대와 동일 선택을 재검사하고 현재 reflog 설명이 표시 항목과 일치하는지 확인한 뒤 ApplyStash 작업을 시작한다. 다른 메뉴와 삭제 확인 흐름은 유지한다.

stash_actions.rs에 apply_stash를 추가한다. 입력은 기존 ReferenceDeletion 형태의 객체 스냅샷이며 stash locator만 허용한다. 별도 Repository 핸들의 transaction으로 refs/stash를 잠근 동안 순번·객체·설명을 재검사하고 git2::stash_apply를 안전한 checkout 옵션으로 실행한다. 스태시 참조와 reflog는 변경하지 않는다. 작업 폴더·HEAD·진행 상태·충돌·인덱스 잠금을 검사한다. 오류 시 스태시 보존을 알리며 기존 작업 완료 경로에서 상태를 재조회한다.

repository.rs/backend.rs에 적용 함수를 노출하고 workspace_backend.rs에 ApplyStash 변형을 추가한다. UI의 실행 가능 검사·진행·완료 갱신을 재사용하고 실패 제목은 스태시 적용 실패로 구분한다. 생성 팝업을 닫는 분기에는 포함하지 않는다.

## 제약·검증·헌법
새 의존성·아키텍처 변경·기존 공개 API 파괴 없음. 헌법은 미작성 템플릿으로 유효 규범 검사 생략. AGENTS.md의 최소 변경·중복 방지·계층·한글 문서를 준수한다. 변경 파일은 src/stash_actions.rs, src/ui.rs, src/repository.rs, src/backend.rs, src/workspace_backend.rs이다.

격리 저장소에서 선택 항목 적용·stash 보존·HEAD 유지·다른 항목과 미추적 파일 보존·순번 변경·dirty 충돌·잠금·잘못된 대상 거부를 검증한다. UI 상태로 스태시 전용 메뉴 대상과 오래된 입력을 검사한다. fmt·전체 test·check·엄격한 clippy·별도 빌드 후 수렴 점검한다. 실제 GUI 조작은 미검증 시 명시한다.
