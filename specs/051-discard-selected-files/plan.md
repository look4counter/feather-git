# 구현 계획

## 기존 흐름과 영향

Rust·GPUI에서 Workspace.open_files_menu가 선택 상태/세대를 검사하고 기존 배치 Stage를 실행한다. workspace_backend.stage_selection은 경로 중복을 제거한다. diff_git는 gix 읽기/인덱스 쓰기와 IndexLock, file_index_state를 제공한다. Alert는 확인 버튼으로 닫기만 지원하므로 파괴 작업에는 기존 Dialog를 재사용하여 취소/폐기 버튼을 명확히 나눈다.

## Git 설계

src/diff_git.rs에 폐기 준비/실행 API와 내보내는 미리보기/결과 타입을 추가하고 src/backend.rs의 래퍼에 등록한다. 준비는 선택 FileChange가 실제 상태와 같은지 확인하고 경로별 HEAD 원본·인덱스 모든 stage·작업 파일 바이트/권한을 보존한다. HEAD가 없는 저장소도 지원한다. 경로는 일반 상대 경로만 허용하고 .git/링크/재분석 지점/디렉터리/서브모듈을 거부한다.

실행은 파일별 IndexLock 아래 HEAD·인덱스·작업 파일을 재검증한다. 기존 git2 checkout_tree를 정확한 경로/force/update_index(false)/disable_pathspec_match로 사용해 HEAD의 일반 파일을 줄바꿈/필터를 반영해 복구한다. HEAD에 없는 파일은 fs::remove_file로만 제거한다. gix 인덱스에서 해당 경로만 HEAD 상태로 교체해 잠금을 기록한다. 실패 시 원래 작업 파일과 권한 복구를 시도하고 복구 실패도 오류에 함께 명시한다. 다른 경로의 인덱스는 보존한다. 파일별 부분 성공을 반환한다.

새 모듈에서 인덱스 잠금을 중복 구현하는 대안 대신 diff_git 내부 기존 잠금/원본 검사를 재사용한다. 기존 공개 형식은 유지하고 외부 의존성은 추가하지 않는다.

## UI 설계

Workspace에 준비/확인/실행 상태를 추가한다. 준비와 실행은 기존 file_stage_busy로 차단하고 확인 중 상태는 repository_busy에 포함한다. 메뉴는 선택 대상/세대를 캡처하며 기존 선택 보호를 재사용한다. 준비 완료 후 현재 프로젝트에서 대상 목록과 명시적 취소/폐기 팝업을 연다. Escape는 취소, Enter는 실행하지 않는다. 포커스 저장/복원과 Dialog 오버레이 차단을 재사용한다. 명시적 폐기 버튼 이후에만 백엔드 쓰기를 호출한다.

완료는 기존 viewer_changed로 파일 목록/뷰어만 갱신한다. 일부 실패를 성공 수와 파일별 오류로 알린다. 커밋·그래프·저장소 전체 갱신을 추가하지 않는다.

## 검증

임시 저장소는 target 아래에 생성하고 Git 이력/파일 폐기는 테스트 저장소에서만 실행한다. 준비 전후 바이트/인덱스 불변과 실제 폐기 결과·다른 파일 보존·오래된 HEAD/인덱스/바이트·잠금/링크/디렉터리 거부를 검증한다. UI 취소/확인 상태 모델과 기존 Stage 회귀를 검증한다. cargo fmt --check, cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행한다. GUI와 실행 파일 잠금 제한은 기록한다. Constitution은 미작성 템플릿이므로 구체 규칙 검사는 생략한다.

공통 메뉴의 단축키가 없는 항목은 빈 배지를 그리지 않도록 src/context_menu.rs를 보완한다. 후속 요청에 따라 폐기 항목에는 Ctrl + D 배지를 표시한다.

## 후속 단축키 연결 계획

src/ui.rs의 files_key_down에 Ctrl+D를 구분하여 기존 포커스/창 활성/쓰기 보호 뒤 prepare_file_discard를 호출한다. 기존 menu_stage_requests로 조회/모달/확인 상태와 선택을 검증한다. src/context_menu.rs는 표시된 단축키와 입력의 key/modifiers를 비교하여 메뉴가 열린 상태에도 같은 콜백을 호출한다. 키 매칭의 Ctrl 단독/다른 수정키 거부 및 메뉴 매칭 테스트와 기존 폐기/Stage 테스트·서식·Clippy를 실행한다.
