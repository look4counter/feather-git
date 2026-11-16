# 구현 계획: 커밋 태그 추가

## 기존 구조와 호출 분석
Rust/GPUI/git2. ui.rs의 open_checkout_menu는 메인 커밋 행과 저장소 행을 공유한다. 커밋 행의 tab=None과 checkout_menu_target으로 선택된 정확한 ID를 얻는다. BranchDialog는 이름·체크박스·확인·취소·IME·포커스·오류·busy 처리를 갖고 있으며 Workspace가 branch_owner로 저장소 소유자를 검증한다. ToolbarAction/Request/Result는 비동기 작업·진행 표시·오류·재조회를 공유한다. committing::push_to는 인증·서버 거부·비강제 단일 참조 Push를 구현하지만 목적지를 브랜치로 제한한다.

## 설계
기존 BranchDialog에 태그 모드를 추가해 대상 ID·태그명·원격에 Push를 표시한다. 별도 push 상태와 태그 대상 ID를 보관하고 태그 확인 이벤트를 추가한다. 일반 브랜치 모드는 유지한다. ui.rs는 커밋 행에만 태그 추가 메뉴를 넣고 경로·선택 ID를 검증한 뒤 팝업을 연다. 기존 branch_owner와 팝업 생명주기·busy·실패 입력 보존·취소 포커스 복원을 재사용한다.

tag_actions.rs에서 전체 커밋 ID·이름을 검증하고 git2의 비강제 경량 태그를 생성한다. Push가 없으면 원격을 읽지 않는다. Push가 있으면 기존 브랜치 원격 정책으로 목적지를 결정하고 committing::push_to를 재사용한다. 공용 Push 함수는 refs/heads와 refs/tags만 허용하도록 최소 확장하며 기존 브랜치 상위 검사는 유지한다. 로컬 생성 후 Push 실패는 성공과 안내 메시지로 반환하고 로컬 태그를 보존한다.

repository.rs/backend.rs로 생성 API를 노출하고 ToolbarAction::CreateTag를 추가한다. 실행 전 팝업 소유자와 고정된 대상 ID를 검증하고 기존 전역 작업 흐름을 사용한다. 완료 후 apply_refreshed_project로 기록·태그를 갱신한다. 태그 생성 실패는 입력 보존, 부분 성공은 팝업 닫기 후 안내를 따른다.

## 영향·제약·검증
변경: src/branch_dialog.rs, src/ui.rs, src/tag_actions.rs, src/repository.rs, src/backend.rs, src/workspace_backend.rs, src/committing.rs. 새 의존성·아키텍처 변경 없음. 헌법은 미작성 템플릿으로 유효 규범 검사를 생략한다. AGENTS.md의 한글 문서·최소 변경·기존 경로 재사용을 준수한다.

격리 저장소와 로컬 bare 원격에서 비HEAD 대상·잘못된 이름·기존 태그·누락 커밋·단일 태그 Push·설정 원격 우선·원격 충돌 및 부분 성공·HEAD와 파일 보존을 검증한다. UI 상태로 오래된 메뉴·팝업 소유자·대상 검사를 검증한다. fmt·전체 test·check·엄격한 clippy·별도 실행 파일 빌드를 수행한다. 실제 GUI와 외부 서버 인증은 미검증 시 보고한다.

## 검증에서 확인한 보완
libgit2의 태그 Push는 비강제 refspec만으로 기존 태그 이동을 모두 거부하지 않았다. 원격 태그 충돌 재현 테스트에 따라 태그 Push negotiation에서 기존 객체 ID가 있고 새 객체 ID와 다르면 전송 전에 거부한다. 일반 브랜치 Push의 기존 fast-forward 동작은 유지한다.
