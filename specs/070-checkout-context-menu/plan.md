# 구현 계획

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

## 기존 구조 및 영향
Rust 2024, GPUI, git2. ui.rs branches는 multi_virtual_list의 왼쪽 입력, history는 virtual_list와 commit_grid_row를 사용한다. 기존 파일 메뉴는 행 우클릭에서 선택을 반영한 후 context_menu를 연다. ToolbarAction::Checkout은 workspace_backend → repository → branch_actions::checkout_reference로 전달되며 전역 작업 차단·비동기 처리·재조회가 이미 있다.

## 설계
ui.rs에서 행 콘텐츠에 오른쪽 입력을 추가한다. 참조는 기존 정확한 참조 이름 검사를 재사용하고 커밋은 정확한 ID를 사용한다. 메뉴 실행 직전 프로젝트, 경로, 탭, 행, 선택과 대상 문자열 및 작업 가능 상태를 재검사한다. 공용 메뉴 실행 함수를 사용한다. branch_actions.rs의 기존 함수에 전체 OID 처리만 추가하고 안전한 checkout 및 HEAD transaction을 공유한다. 기존 공개 함수 및 ToolbarAction 형태를 유지한다. 추가 의존성 없음.

## 검증과 헌법
임시 저장소 테스트로 커밋 ID 전환, 충돌 보호, 잘못된 대상 거부를 검증한다. UI 순수 상태 검사로 참조와 커밋 선택 및 오래된 대상 거부를 확인한다. cargo fmt --check, cargo test, cargo check, cargo clippy -- -D warnings 실행. 실제 GUI 조작은 별도 미검증으로 보고한다. 헌법은 미작성 템플릿으로 유효 규범 검사를 생략하며 AGENTS.md의 재사용·계층·최소 변경을 준수한다.

## 브랜치 우선 기록 체크아웃 계획

기존 구현 분석: ui.rs의 checkout_menu_target과 history_checkout_target은 커밋 ID를 반환하고 각각 우클릭·더블클릭에서 ToolbarAction::Checkout으로 연결한다. ID는 메뉴·선택의 안정 식별자이므로 그대로 유지하고 공통 기록 체크아웃 진입점에서 참조 후보를 해석한다. start_toolbar_action은 원격 참조의 기존 로컬 재사용·추적 생성 및 Detached HEAD 확인을 담당한다.
설계: 선택 Commit.references에서 refs/heads 후보를 정렬·중복 제거하고 있으면 그것만 사용한다. 없으면 refs/remotes 후보를 쓰되 /HEAD 별칭은 제외한다. 후보 0개는 기존 ID 흐름, 1개는 즉시 분기, 여러 개는 BranchCheckoutPrompt로 프로젝트·경로·커밋·행·후보·선택·공용 스크롤을 보관한다. 공용 Dialog·multi_virtual_list·포커스/Tab/Enter/Escape·확인/취소를 사용하고 repository_busy에 포함한다. 실행 직전 후보·문맥을 재검증하고 팝업을 한 번 소비한다.
로컬 후보는 CheckoutCommitBranch {reference, id} 요청으로 전달한다. branch_actions.rs의 기존 체크아웃 코어를 공유하여 expected commit을 참조 잠금 전에 검증하고 기존 잠금 후 재검증을 유지한다. 기존 checkout_reference 서명은 보존한다. 원격 후보는 기존 원격 흐름을 사용한다.
영향: src/ui.rs·src/branch_actions.rs·src/repository.rs·src/backend.rs·src/workspace_backend.rs 및 내부 테스트다. UI 렌더링에서 I/O를 하지 않으며 새 의존성·세션 형식 변경 없음. 헌장은 미작성 템플릿이고 기존 UI/백엔드 계층·공용 목록 원칙에 충돌 없음.
검증: 실제 저장소의 attached HEAD와 stale ref 거절, UI 후보·다중 선택·취소·문맥 보호·기존 기록 체크아웃 회귀를 먼저 작성한다. 역방향 복귀 수정과 함께 전체 회귀·컴파일·엄격한 Clippy·별도 빌드 후 수렴한다.

## 목록 표시 오류 분석 및 보완 계획
branch_checkout_dialog의 높이 지정 래퍼는 일반 블록이고 multi_virtual_list 루트는 flex_1 기반이다. 내부 목록이 고정 높이를 전달받도록 래퍼를 세로 flex 컨테이너로 만들고 축소를 방지한다. 공용 가상 목록이나 후보·체크아웃 계층은 변경하지 않는다. 기존 후보·팝업 상태 테스트, 컴파일·엄격한 Clippy·별도 실행 파일 빌드로 검증한다. 실제 화면 접근은 도구 권한 제한으로 불가능하여 시각 검증은 미검증으로 명시한다.

## 인코딩 선택 스타일 적용 계획
분석: EncodingDialog는 폭 400, 높이 28의 행, 간격 4, px_3·gap_2·rounded_sm, 16픽셀 체크 표시 영역, SELECTED 선택 배경과 RAISED hover를 사용한다. 브랜치 목록의 공용 multi_virtual_list는 selectable_row에서 높이를 24와 사각 배경으로 고정하므로 이 UI에 필요한 스타일을 직접 수용하지 못한다.
결정: 같은 GPUI uniform_list와 보존된 ListScrollHandle.vertical을 사용하여 인코딩 행 스타일을 렌더링한다. 공용 목록 자체의 API·행 스타일은 변경하지 않는다. 32픽셀 항목 안에 높이 28의 행과 아래 간격 4를 배치한다. 기존 Dialog·확인 취소·포커스·후보·백엔드는 유지한다. 영향 범위는 ui.rs의 branch_checkout_dialog만이다. 추가 의존성·데이터 형식 변경 없음. 헌법은 템플릿이며 기존 계층 분리를 유지한다.
검증: 관련 history_checkout_와 encoding_dialog 테스트, cargo check 및 엄격한 Clippy, 별도 실행 파일 빌드. 실제 GUI 확인은 이전 앱 접근 거부가 유지되어 미검증으로 기록한다. 스타일 속성만 반복하는 단위 테스트는 추가하지 않는다.
