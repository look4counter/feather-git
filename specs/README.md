# 현재 명세 기준과 기능 색인

정리일: 2026-11-17. 범위: 기능 명세 001~080과 관련 계획·작업·검증 문서. 이번 정리는 문서만 변경하며 제품 동작·사용자 저장소·기존 검증 결과를 변경하지 않는다.

## 문서 적용 규칙

- 각 기능의 spec.md 본문은 변경 이력을 포함한다. 후속 명세가 명시적으로 바꾼 항목은 아래 현재 기준을 적용하고 나머지 기존 요구사항은 유지한다. 번호가 높다는 이유만으로 앞선 명세 전체를 폐기하지 않는다.
- plan.md의 기존 구현 분석은 작업 시작 당시의 상태다. tasks.md의 체크는 해당 단계의 기록이며 전체 제품의 현재 검증 완료를 의미하지 않는다.
- validation.md·quickstart.md·research.md는 작성 당시 결과·조사 이력이다. 과거 테스트 개수·의존성 부재·실행 파일 잠금을 현재 상태로 해석하지 않는다.
- data-model.md·contracts는 해당 기능의 설계 기록이다. 후속 명세가 바꾼 모델·UI 계약은 아래 색인의 후속 문서를 함께 적용한다.
- 소스와 명세가 다르면 차이를 보고하고 사용자 의도를 확인한다. 요구사항을 소스에 맞춰 없애거나 미실행 검증을 완료로 표시하지 않는다.

## 현재 유효 기준

| 항목 | 현재 기준 | 근거 명세 · 소스 |
|---|---|---|
| 초기 화면·샘플 | 런타임 샘플 제거. 저장된 프로젝트·시작 탭·최근 저장소 복원 | 020·021 / ui.rs·session.rs |
| 프로젝트 탭 | 높이 42px, 오른쪽 1px 경계, 활성 닫기 상시·비활성 닫기 호버 표시 | 001 FR-017~022 / ui.rs |
| 패널·목록 | 공통 패널 제목 36px, 가상 목록·기록 그리드 헤더/행 24px | 001·011·059 / ui.rs·virtual_list.rs |
| 그래프 크기 | 레인 간격 18px, 좌측 기준 10px. 초기 열 graph_width(5)=92px, 최소 0, 넘침 잘라 표시 | 018·059 / history_graph.rs·history_grid.rs |
| 그래프 부모 | 첫 부모는 현재 레인. 추가 부모는 같은 부모를 기다리는 최소 레인 및 색 재사용, 없으면 새 레인 | 080 FR-023 / graph.rs |
| 기록 조회 | 최초 1,500개, 하단 50행 이내에서 기존 커서로 200개씩 추가 | 058 / repository.rs·workspace_backend.rs |
| 커밋상세 행 | 파일 목록의 첫 가상 행. 대기 중 이전 화면과 선택 배경 보존 | 027·035 / ui.rs |
| 지연·빈 패널 | 150ms 전 이전 내용은 표시 전용. 이후 로딩 안내는 본문 양축 중앙·12px. 빈 상태도 중앙 | 035 / loading.rs·virtual_list.rs·diff_viewer.rs |
| 백엔드 | 최대 2개 전용 작업 스레드, 최신 읽기 요청 취소·오래된 결과 거부 | 024·025 / backend_worker.rs |
| 파일 정렬 | Staged 우선, 그룹 내 U→M→D→A→기타, 폴더 순회 경로순 | 047 / diff_git.rs |
| 새로고침 | 수정키 없는 F5는 전역 로컬 새로고침. Fetch 없음. Stage 완료 자동 갱신은 파일·diff 한정 | 045·080 / ui.rs·workspace_backend.rs |
| 뷰어 제한 | 미리보기 1 MiB·5,000줄 상한 유지. 가상 목록이 읽기 제한을 없애지는 않음 | 019 / diff_git.rs·diff_viewer.rs |
| 입력 | 공통 TextInput, IME·읽기 전용·비밀 보호, Shift+Home/End 현재 줄 선택 | 006·063·064·066·080 / text_input.rs |
| 인코딩 | 11개 항목을 임시 선택 후 확인으로 적용. 취소는 무변경 | 058·059·060 / encoding_dialog.rs |
| 체크아웃 | 기록 후보가 하나여도 브랜치/Detached HEAD 선택 제공. detached 전환은 경고 확인 | 070·071·080 / ui.rs |
| 원격 체크아웃 | 동명 로컬 브랜치는 재사용, 없으면 브랜치 생성 창. 무조건 detached 전환하지 않음 | 071·072 / ui.rs·reference_actions.rs |
| 실행 확인 | Merge·Rebase·툴바 Push·Pull 병합/ff-only는 확인 뒤 시작 | 080 / ui.rs·git_action_dialog.rs |
| Git 후속 처리 | Merge·Rebase·Revert 상태별 버튼. Revert 취소 확인, 앱 no-commit 충돌 계속 후 자동 커밋하지 않음 | 078·079·080 / commit_history_actions.rs·ui.rs |
| 찾기 | Enter 검색은 포커스·IME 검증. 커밋 선택 즉시 이동·커밋 메뉴 없음. 파일 메뉴는 파일 이력만, 복사 없음 | 080 FR-021~022 / search_panel.rs |
| 공통 UI | 입력·체크박스·버튼은 form_controls.rs. settings_controls.rs는 과거 명칭 | 060·062·063 / form_controls.rs |

## 기능별 소스와 후속 변경

소스 열은 해당 계획·작업 문서에 언급된 현재 존재 파일의 탐색 지점이다. 전체 호출 관계나 개별 수용 기준의 전수 실행 증명이 아니다. 후속 열은 변경 관련 문서이며 앞선 기능 전체의 폐기를 뜻하지 않는다.

| 기능 | 관련 소스 탐색 지점 | 후속 변경 참고 | 작업 문서 상태 |
|---|---|---|---|
| [001 데스크톱 기본 화면](001-desktop-shell/spec.md) | [ui.rs](../src/ui.rs) · [main.rs](../src/main.rs) | [005](005-repository-dialogs/spec.md) · [020](020-remove-sample-projects/spec.md) · [021](021-persist-workspace/spec.md) · [035](035-viewer-loading-message/spec.md) · [065](065-pull-options/spec.md) · [080](080-history-revert-reset/spec.md) | 미완료 8개 |
| [002 기존 프로젝트 방식의 커밋 그래프](002-history-graph/spec.md) | [ui.rs](../src/ui.rs) · [graph.rs](../src/graph.rs) · [history_graph.rs](../src/history_graph.rs) · [main.rs](../src/main.rs) | [008](008-history-performance/spec.md) · [018](018-remove-graph-minimum/spec.md) · [059](059-footer-overlay-actions/spec.md) · [080](080-history-revert-reset/spec.md) | 미완료 2개 |
| [003 단일 임시 새 탭](003-single-start-tab/spec.md) | 문서 본문 참조 | [005](005-repository-dialogs/spec.md) · [020](020-remove-sample-projects/spec.md) · [021](021-persist-workspace/spec.md) | 미완료 2개 |
| [004 재사용 alert 팝업](004-alert-component/spec.md) | [alert.rs](../src/alert.rs) · [main.rs](../src/main.rs) · [theme.rs](../src/theme.rs) · [ui.rs](../src/ui.rs) | [005](005-repository-dialogs/spec.md) · [062](062-dialog-buttons-escape/spec.md) | 미완료 1개 |
| [005 저장소 설정 팝업과 실제 프로젝트 열기](005-repository-dialogs/spec.md) | [main.rs](../src/main.rs) · [repository.rs](../src/repository.rs) · [repository_dialog.rs](../src/repository_dialog.rs) · [text_input.rs](../src/text_input.rs) | [020](020-remove-sample-projects/spec.md) · [029](029-global-repository-progress/spec.md) · [062](062-dialog-buttons-escape/spec.md) · [063](063-shared-input-checkbox/spec.md) | 미완료 2개 |
| [006 Windows 한글 조합 뒤 영문 입력 보존](006-korean-ime/spec.md) | 문서 본문 참조 | [063](063-shared-input-checkbox/spec.md) · [080](080-history-revert-reset/spec.md) | 미완료 1개 |
| [007 공통 진행 표시와 Dialog 기반](007-shared-progress/spec.md) | [dialog.rs](../src/dialog.rs) · [main.rs](../src/main.rs) · [progress.rs](../src/progress.rs) · [repository_dialog.rs](../src/repository_dialog.rs) | [021](021-persist-workspace/spec.md) · [029](029-global-repository-progress/spec.md) | 미완료 2개 |
| [008 커밋 히스토리 렌더링 최적화와 레인 겹침 방지](008-history-performance/spec.md) | [history_graph.rs](../src/history_graph.rs) · [history.rs](../src/history.rs) · [main.rs](../src/main.rs) · [ui.rs](../src/ui.rs) | [015](015-history-grid-columns/spec.md) · [018](018-remove-graph-minimum/spec.md) · [059](059-footer-overlay-actions/spec.md) · [080](080-history-revert-reset/spec.md) | 미완료 1개 |
| [009 로컬 탭의 브랜치 표시](009-local-branch-list/spec.md) | 문서 본문 참조 | [010](010-panel-tab-buttons/spec.md) · [028](028-independent-repository-tabs/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [010 패널 탭 버튼과 전환](010-panel-tab-buttons/spec.md) | [ui.rs](../src/ui.rs) | [028](028-independent-repository-tabs/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [011 공통 가상화 목록](011-shared-virtual-list/spec.md) | [ui.rs](../src/ui.rs) · [virtual_list.rs](../src/virtual_list.rs) · [main.rs](../src/main.rs) | [017](017-common-list-scroll/spec.md) · [026](026-list-delayed-loading/spec.md) · [035](035-viewer-loading-message/spec.md) · [048](048-worktree-multiselect/spec.md) · [049](049-ignore-reselected-rows/spec.md) · [071](071-list-keys-detached-warning/spec.md) | 미체크 작업 없음 |
| [012 커밋 원격 참조 아이콘](012-remote-reference-icon/spec.md) | [history.rs](../src/history.rs) · [repository.rs](../src/repository.rs) · [ui.rs](../src/ui.rs) | [014](014-reference-kind-icons/spec.md) · [044](044-reference-badge-colors/spec.md) · [059](059-footer-overlay-actions/spec.md) | 미체크 작업 없음 |
| [013 저장소 패널과 커밋 선택 연동](013-repository-history-selection/spec.md) | [history.rs](../src/history.rs) · [repository.rs](../src/repository.rs) · [ui.rs](../src/ui.rs) | [028](028-independent-repository-tabs/spec.md) · [049](049-ignore-reselected-rows/spec.md) · [068](068-commit-push-checkout/spec.md) · [071](071-list-keys-detached-warning/spec.md) | 미체크 작업 없음 |
| [014 로컬·원격·태그 아이콘](014-reference-kind-icons/spec.md) | [history.rs](../src/history.rs) · [repository.rs](../src/repository.rs) · [ui.rs](../src/ui.rs) | [020](020-remove-sample-projects/spec.md) · [059](059-footer-overlay-actions/spec.md) | 미체크 작업 없음 |
| [015 커밋 그리드 정렬 및 컬럼 크기 조절](015-history-grid-columns/spec.md) | [ui.rs](../src/ui.rs) · [history_graph.rs](../src/history_graph.rs) · [history.rs](../src/history.rs) · [history_grid.rs](../src/history_grid.rs) | [018](018-remove-graph-minimum/spec.md) · [059](059-footer-overlay-actions/spec.md) | 미체크 작업 없음 |
| [016 히스토리의 고정 커밋 상세 행](016-pinned-commit-detail/spec.md) | [ui.rs](../src/ui.rs) · [virtual_list.rs](../src/virtual_list.rs) | [027](027-history-detail-list-item/spec.md) · [035](035-viewer-loading-message/spec.md) | 미체크 작업 없음 |
| [017 공통 목록 스크롤·마우스 조작](017-common-list-scroll/spec.md) | [scrolling.rs](../src/scrolling.rs) · [ui.rs](../src/ui.rs) · [history.rs](../src/history.rs) · [main.rs](../src/main.rs) | [027](027-history-detail-list-item/spec.md) · [054](054-scroll-axis-grid-loading/spec.md) | 미체크 작업 없음 |
| [018 그래프 컬럼 최소 너비 제거](018-remove-graph-minimum/spec.md) | [history_grid.rs](../src/history_grid.rs) | [059](059-footer-overlay-actions/spec.md) | 미체크 작업 없음 |
| [019 기존 프로젝트와 동등한 Diff Viewer](019-diff-viewer/spec.md) | [partial_stage.rs](../src/partial_stage.rs) · [conflict.rs](../src/conflict.rs) · [diff_model.rs](../src/diff_model.rs) | [035](035-viewer-loading-message/spec.md) · [041](041-diff-fixed-gutter/spec.md) · [042](042-staged-first-hunk-select/spec.md) · [050](050-panel-context-menus/spec.md) · [055](055-diff-text-menu-line-discard/spec.md) · [056](056-diff-line-ending-blank-menu/spec.md) · [058](058-diff-encoding-picker/spec.md) | 미체크 작업 없음 |
| [020 샘플 프로젝트 제거](020-remove-sample-projects/spec.md) | [ui.rs](../src/ui.rs) · [main.rs](../src/main.rs) · [repository.rs](../src/repository.rs) | [021](021-persist-workspace/spec.md) | 미체크 작업 없음 |
| [021 프로젝트 탭 복원과 최근 저장소](021-persist-workspace/spec.md) | [ui.rs](../src/ui.rs) · [main.rs](../src/main.rs) · [repository.rs](../src/repository.rs) · [session.rs](../src/session.rs) | [029](029-global-repository-progress/spec.md) · [032](032-recent-repository-removal/spec.md) · [033](033-hold-repository-deletion/spec.md) | 미체크 작업 없음 |
| [022 창 종료와 잔류 프로세스 원인 조사](022-quit-on-last-window/spec.md) | 문서 본문 참조 | — | 미체크 작업 없음 |
| [023 백엔드 공통 지연 옵션](023-backend-throttle/spec.md) | [backend.rs](../src/backend.rs) · [diff_viewer.rs](../src/diff_viewer.rs) · [main.rs](../src/main.rs) · [session.rs](../src/session.rs) | [024](024-responsive-backend/spec.md) · [025](025-backend-resource-limits/spec.md) | 미체크 작업 없음 |
| [024 백엔드 지연에 막히지 않는 UI](024-responsive-backend/spec.md) | [ui.rs](../src/ui.rs) · [diff_viewer.rs](../src/diff_viewer.rs) · [history.rs](../src/history.rs) · [backend.rs](../src/backend.rs) | [025](025-backend-resource-limits/spec.md) · [026](026-list-delayed-loading/spec.md) · [028](028-independent-repository-tabs/spec.md) · [035](035-viewer-loading-message/spec.md) · [054](054-scroll-axis-grid-loading/spec.md) | 미완료 2개 |
| [025 백엔드 자원 사용 제한](025-backend-resource-limits/spec.md) | [backend.rs](../src/backend.rs) · [backend_worker.rs](../src/backend_worker.rs) · [diff_git.rs](../src/diff_git.rs) · [diff_viewer.rs](../src/diff_viewer.rs) | — | 미체크 작업 없음 |
| [026 목록의 지연 로딩 안내 공통화](026-list-delayed-loading/spec.md) | [ui.rs](../src/ui.rs) · [virtual_list.rs](../src/virtual_list.rs) · [loading.rs](../src/loading.rs) | [028](028-independent-repository-tabs/spec.md) · [035](035-viewer-loading-message/spec.md) · [054](054-scroll-axis-grid-loading/spec.md) | 미체크 작업 없음 |
| [027 히스토리 커밋상세를 목록 항목으로 표시](027-history-detail-list-item/spec.md) | 문서 본문 참조 | [035](035-viewer-loading-message/spec.md) | 미체크 작업 없음 |
| [028 저장소 탭 전환과 변경 파일 조회 분리](028-independent-repository-tabs/spec.md) | [workspace_backend.rs](../src/workspace_backend.rs) · [ui.rs](../src/ui.rs) | [035](035-viewer-loading-message/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [029 저장소 작업 전역 진행 표시와 최근 목록 가독성](029-global-repository-progress/spec.md) | [progress.rs](../src/progress.rs) · [repository_dialog.rs](../src/repository_dialog.rs) · [ui.rs](../src/ui.rs) | [030](030-text-truncation-cache/spec.md) · [035](035-viewer-loading-message/spec.md) | 미체크 작업 없음 |
| [030 최근 저장소 텍스트 말줄임 캐시 수정](030-text-truncation-cache/spec.md) | [main.rs](../src/main.rs) | — | 미체크 작업 없음 |
| [031 저장소 경로의 표시용 접두사 정리](031-display-repository-paths/spec.md) | [ui.rs](../src/ui.rs) · [diff_viewer.rs](../src/diff_viewer.rs) · [main.rs](../src/main.rs) · [path_display.rs](../src/path_display.rs) | — | 미체크 작업 없음 |
| [032 최근 저장소 삭제](032-recent-repository-removal/spec.md) | [backend.rs](../src/backend.rs) · [main.rs](../src/main.rs) · [recent_repository_dialog.rs](../src/recent_repository_dialog.rs) · [repository.rs](../src/repository.rs) | [033](033-hold-repository-deletion/spec.md) | 미체크 작업 없음 |
| [033 길게 눌러 저장소 영구 삭제](033-hold-repository-deletion/spec.md) | 문서 본문 참조 | — | 미체크 작업 없음 |
| [034 비교 뷰어 패널 헤더 정리](034-commit-detail-header/spec.md) | 문서 본문 참조 | [035](035-viewer-loading-message/spec.md) · [050](050-panel-context-menus/spec.md) | 미체크 작업 없음 |
| [035 Diff Viewer 지연 로딩 안내](035-viewer-loading-message/spec.md) | [ui.rs](../src/ui.rs) · [diff_viewer.rs](../src/diff_viewer.rs) · [virtual_list.rs](../src/virtual_list.rs) · [file_history_dialog.rs](../src/file_history_dialog.rs) | — | 미체크 작업 없음 |
| [036 작성자·커미터 사진과 두 줄 표시](036-commit-identity-avatars/spec.md) | [avatar.rs](../src/avatar.rs) · [diff_model.rs](../src/diff_model.rs) · [diff_viewer.rs](../src/diff_viewer.rs) · [main.rs](../src/main.rs) | [037](037-github-avatar-cache/spec.md) | 미체크 작업 없음 |
| [037 GitHub 프로필 사진과 세션 캐시](037-github-avatar-cache/spec.md) | [avatar.rs](../src/avatar.rs) · [backend.rs](../src/backend.rs) · [diff_viewer.rs](../src/diff_viewer.rs) · [repository.rs](../src/repository.rs) | — | 미체크 작업 없음 |
| [038 십자 패널 크기 조절과 그래프 간격](038-panel-widths-graph-spacing/spec.md) | 문서 본문 참조 | [059](059-footer-overlay-actions/spec.md) | 미체크 작업 없음 |
| [039 패널 클릭 포커스](039-panel-focus/spec.md) | 문서 본문 참조 | [071](071-list-keys-detached-warning/spec.md) | 미체크 작업 없음 |
| [040 커밋 재선택 방지와 단어 선택](040-viewer-word-selection/spec.md) | [diff_viewer.rs](../src/diff_viewer.rs) · [ui.rs](../src/ui.rs) · [text_selection.rs](../src/text_selection.rs) | [049](049-ignore-reselected-rows/spec.md) | 미체크 작업 없음 |
| [041 DiffViewer 고정 여백과 기호 정렬](041-diff-fixed-gutter/spec.md) | [diff_viewer.rs](../src/diff_viewer.rs) · [virtual_list.rs](../src/virtual_list.rs) | [042](042-staged-first-hunk-select/spec.md) · [050](050-panel-context-menus/spec.md) · [055](055-diff-text-menu-line-discard/spec.md) | 미체크 작업 없음 |
| [042 Staged 우선 정렬과 구간 선택](042-staged-first-hunk-select/spec.md) | [diff_git.rs](../src/diff_git.rs) · [diff_viewer.rs](../src/diff_viewer.rs) · [ui.rs](../src/ui.rs) · [workspace_backend.rs](../src/workspace_backend.rs) | [047](047-status-flat-tree-sort/spec.md) · [050](050-panel-context-menus/spec.md) | 미체크 작업 없음 |
| [043 패널별 하단 단축키 배지](043-panel-shortcut-badges/spec.md) | [backend.rs](../src/backend.rs) · [diff_git.rs](../src/diff_git.rs) · [diff_viewer.rs](../src/diff_viewer.rs) · [ui.rs](../src/ui.rs) | [050](050-panel-context-menus/spec.md) | 미체크 작업 없음 |
| [044 참조 배지 종류별 색상](044-reference-badge-colors/spec.md) | 문서 본문 참조 | [045](045-stage-refresh-whitespace/spec.md) · [059](059-footer-overlay-actions/spec.md) | 미체크 작업 없음 |
| [045 Stage 갱신 범위와 diff 공백 표시](045-stage-refresh-whitespace/spec.md) | [ui.rs](../src/ui.rs) · [diff_model.rs](../src/diff_model.rs) · [diff_git.rs](../src/diff_git.rs) · [workspace_backend.rs](../src/workspace_backend.rs) | [047](047-status-flat-tree-sort/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [046 깨끗한 작업 트리의 잘못된 변경 표시 수정](046-clean-worktree-status/spec.md) | 문서 본문 참조 | — | 미체크 작업 없음 |
| [047 상태와 평면 폴더 트리 기준 정렬](047-status-flat-tree-sort/spec.md) | [diff_git.rs](../src/diff_git.rs) · [workspace_backend.rs](../src/workspace_backend.rs) | — | 미체크 작업 없음 |
| [048 작업 트리 다중 선택과 여러 파일 diff](048-worktree-multiselect/spec.md) | [ui.rs](../src/ui.rs) · [diff_viewer.rs](../src/diff_viewer.rs) · [virtual_list.rs](../src/virtual_list.rs) · [workspace_backend.rs](../src/workspace_backend.rs) | [050](050-panel-context-menus/spec.md) · [052](052-file-row-common-menu/spec.md) | 미체크 작업 없음 |
| [049 저장소·커밋 기록 중복 선택 무시](049-ignore-reselected-rows/spec.md) | 문서 본문 참조 | — | 미체크 작업 없음 |
| [050 패널 공통 컨텍스트 메뉴](050-panel-context-menus/spec.md) | [ui.rs](../src/ui.rs) | [051](051-discard-selected-files/spec.md) · [052](052-file-row-common-menu/spec.md) · [055](055-diff-text-menu-line-discard/spec.md) | 미체크 작업 없음 |
| [051 선택 파일 변경사항 폐기](051-discard-selected-files/spec.md) | [diff_git.rs](../src/diff_git.rs) · [ui.rs](../src/ui.rs) | [052](052-file-row-common-menu/spec.md) · [055](055-diff-text-menu-line-discard/spec.md) | 미체크 작업 없음 |
| [052 변경 파일 행 공통 메뉴](052-file-row-common-menu/spec.md) | [path_display.rs](../src/path_display.rs) | [053](053-file-history-overlay/spec.md) | 미체크 작업 없음 |
| [053 파일 변경 이력 오버레이](053-file-history-overlay/spec.md) | [file_history.rs](../src/file_history.rs) · [file_history_dialog.rs](../src/file_history_dialog.rs) · [history_grid.rs](../src/history_grid.rs) · [ui.rs](../src/ui.rs) | [054](054-scroll-axis-grid-loading/spec.md) · [057](057-history-row-context/spec.md) · [069](069-file-history-detail/spec.md) | 미체크 작업 없음 |
| [054 스크롤 축 보호와 커밋 그리드 지연 로딩](054-scroll-axis-grid-loading/spec.md) | 문서 본문 참조 | [035](035-viewer-loading-message/spec.md) | 미체크 작업 없음 |
| [055 Diff 텍스트 메뉴와 체크 줄 폐기](055-diff-text-menu-line-discard/spec.md) | [diff_git.rs](../src/diff_git.rs) · [diff_viewer.rs](../src/diff_viewer.rs) · [partial_stage.rs](../src/partial_stage.rs) | [056](056-diff-line-ending-blank-menu/spec.md) · [058](058-diff-encoding-picker/spec.md) | 미체크 작업 없음 |
| [056 줄바꿈 비교 정규화와 빈 줄 메뉴](056-diff-line-ending-blank-menu/spec.md) | [diff_git.rs](../src/diff_git.rs) · [line_endings.rs](../src/line_endings.rs) · [main.rs](../src/main.rs) | — | 미체크 작업 없음 |
| [057 파일 이력 행 컨텍스트 메뉴](057-history-row-context/spec.md) | [backend.rs](../src/backend.rs) · [file_history.rs](../src/file_history.rs) · [file_history_dialog.rs](../src/file_history_dialog.rs) | — | 미체크 작업 없음 |
| [058 Diff 인코딩 선택·하단 표시 정리·커밋 증분 조회](058-diff-encoding-picker/spec.md) | [backend.rs](../src/backend.rs) · [ui.rs](../src/ui.rs) · [diff_git.rs](../src/diff_git.rs) · [diff_viewer.rs](../src/diff_viewer.rs) | [059](059-footer-overlay-actions/spec.md) · [060](060-settings-overlay/spec.md) | 미체크 작업 없음 |
| [059 공용 인코딩 오버레이와 하단 버튼](059-footer-overlay-actions/spec.md) | [ui.rs](../src/ui.rs) · [encoding_dialog.rs](../src/encoding_dialog.rs) · [main.rs](../src/main.rs) · [repository_dialog.rs](../src/repository_dialog.rs) | [060](060-settings-overlay/spec.md) · [062](062-dialog-buttons-escape/spec.md) | 미체크 작업 없음 |
| [060 환경설정 오버레이](060-settings-overlay/spec.md) | [auth_settings.rs](../src/auth_settings.rs) · [settings.rs](../src/settings.rs) · [auth_profiles.rs](../src/auth_profiles.rs) · [ui.rs](../src/ui.rs) | [062](062-dialog-buttons-escape/spec.md) · [063](063-shared-input-checkbox/spec.md) · [064](064-settings-list-focus/spec.md) | 미체크 작업 없음 |
| [061 빌드 경고 정리](061-build-warnings/spec.md) | [diff_git.rs](../src/diff_git.rs) · [diff_model.rs](../src/diff_model.rs) · [repository.rs](../src/repository.rs) · [backend.rs](../src/backend.rs) | — | 미체크 작업 없음 |
| [062 팝업 버튼 및 ESC 통일](062-dialog-buttons-escape/spec.md) | [alert.rs](../src/alert.rs) · [auth_settings.rs](../src/auth_settings.rs) · [encoding_dialog.rs](../src/encoding_dialog.rs) · [recent_repository_dialog.rs](../src/recent_repository_dialog.rs) | — | 미체크 작업 없음 |
| [063 공통 입력창·체크박스 및 단어 선택](063-shared-input-checkbox/spec.md) | [auth_settings.rs](../src/auth_settings.rs) · [repository_dialog.rs](../src/repository_dialog.rs) · [settings.rs](../src/settings.rs) · [diff_viewer.rs](../src/diff_viewer.rs) | [064](064-settings-list-focus/spec.md) · [066](066-commit-dialog/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [064 설정 목록 디자인 및 포커스 선택 해제](064-settings-list-focus/spec.md) | [settings.rs](../src/settings.rs) · [auth_settings.rs](../src/auth_settings.rs) · [form_controls.rs](../src/form_controls.rs) · [text_input.rs](../src/text_input.rs) | — | 미체크 작업 없음 |
| [065 Pull 옵션 실행 메뉴](065-pull-options/spec.md) | [synchronization.rs](../src/synchronization.rs) · [repository.rs](../src/repository.rs) · [backend.rs](../src/backend.rs) · [context_menu.rs](../src/context_menu.rs) | [079](079-history-rebase/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [066 Commit 팝업과 실행](066-commit-dialog/spec.md) | [committing.rs](../src/committing.rs) · [commit_dialog.rs](../src/commit_dialog.rs) · [repository.rs](../src/repository.rs) · [ui.rs](../src/ui.rs) | [068](068-commit-push-checkout/spec.md) · [078](078-history-operation-toolbar/spec.md) | 미체크 작업 없음 |
| [067 Push와 새 브랜치](067-push-new-branch/spec.md) | [backend.rs](../src/backend.rs) · [repository.rs](../src/repository.rs) | [068](068-commit-push-checkout/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [068 커밋 확인·빈 푸시·참조 체크아웃](068-commit-push-checkout/spec.md) | [backend.rs](../src/backend.rs) · [repository.rs](../src/repository.rs) · [ui.rs](../src/ui.rs) | [071](071-list-keys-detached-warning/spec.md) · [072](072-reference-delete/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [069 파일 이력 커밋 상세](069-file-history-detail/spec.md) | 문서 본문 참조 | — | 미체크 작업 없음 |
| [070 체크아웃 컨텍스트 메뉴](070-checkout-context-menu/spec.md) | [backend.rs](../src/backend.rs) · [repository.rs](../src/repository.rs) · [ui.rs](../src/ui.rs) · [branch_actions.rs](../src/branch_actions.rs) | [071](071-list-keys-detached-warning/spec.md) · [072](072-reference-delete/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [071 목록 키보드 이동과 detached HEAD 확인](071-list-keys-detached-warning/spec.md) | [backend.rs](../src/backend.rs) · [repository.rs](../src/repository.rs) · [branch_actions.rs](../src/branch_actions.rs) · [branch_dialog.rs](../src/branch_dialog.rs) | [072](072-reference-delete/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [072 저장소 항목 삭제 메뉴](072-reference-delete/spec.md) | [repository.rs](../src/repository.rs) · [backend.rs](../src/backend.rs) · [ui.rs](../src/ui.rs) | — | 미체크 작업 없음 |
| [073 선택 커밋에 태그 추가](073-commit-add-tag/spec.md) | [repository.rs](../src/repository.rs) · [backend.rs](../src/backend.rs) · [branch_dialog.rs](../src/branch_dialog.rs) · [committing.rs](../src/committing.rs) | [074](074-tag-delete-history-stash/spec.md) | 미체크 작업 없음 |
| [074 원격 태그 삭제·커밋 더블클릭·Stash](074-tag-delete-history-stash/spec.md) | [backend.rs](../src/backend.rs) · [repository.rs](../src/repository.rs) · [ui.rs](../src/ui.rs) · [tag_actions.rs](../src/tag_actions.rs) | — | 미체크 작업 없음 |
| [075 스태시 적용 메뉴](075-stash-apply/spec.md) | [repository.rs](../src/repository.rs) · [backend.rs](../src/backend.rs) · [stash_actions.rs](../src/stash_actions.rs) · [ui.rs](../src/ui.rs) | — | 미체크 작업 없음 |
| [076 스태시 충돌 경로와 목록 표시](076-stash-conflict-paths/spec.md) | [repository.rs](../src/repository.rs) · [alert.rs](../src/alert.rs) · [backend.rs](../src/backend.rs) · [stash_actions.rs](../src/stash_actions.rs) | — | 미체크 작업 없음 |
| [077 커밋 기록 병합](077-history-merge/spec.md) | [repository.rs](../src/repository.rs) · [synchronization.rs](../src/synchronization.rs) · [backend.rs](../src/backend.rs) · [ui.rs](../src/ui.rs) | [078](078-history-operation-toolbar/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [078 커밋 기록 작업 상태 툴바](078-history-operation-toolbar/spec.md) | [backend.rs](../src/backend.rs) · [repository.rs](../src/repository.rs) · [ui.rs](../src/ui.rs) · [workspace_backend.rs](../src/workspace_backend.rs) | [079](079-history-rebase/spec.md) · [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [079 커밋 기록 양방향 리베이스](079-history-rebase/spec.md) | [repository.rs](../src/repository.rs) · [backend.rs](../src/backend.rs) · [ui.rs](../src/ui.rs) · [workspace_backend.rs](../src/workspace_backend.rs) | [080](080-history-revert-reset/spec.md) | 미체크 작업 없음 |
| [080 Git 작업 확인·전역 새로고침·찾기](080-history-revert-reset/spec.md) | [backend.rs](../src/backend.rs) · [repository.rs](../src/repository.rs) | — | 미체크 작업 없음 |

## 남은 검증과 기록 한계

총 845개 작업 중 미체크 작업은 21개다. 기존 체크 상태는 변경하지 않았다.

- [001-desktop-shell/tasks.md](001-desktop-shell/tasks.md): T007, T013, T017, T022, T026, T029, T032, T035.
- [002-history-graph/tasks.md](002-history-graph/tasks.md): T006, T010.
- [003-single-start-tab/tasks.md](003-single-start-tab/tasks.md): T006, T009.
- [004-alert-component/tasks.md](004-alert-component/tasks.md): T006.
- [005-repository-dialogs/tasks.md](005-repository-dialogs/tasks.md): T009, T010.
- [006-korean-ime/tasks.md](006-korean-ime/tasks.md): T006.
- [007-shared-progress/tasks.md](007-shared-progress/tasks.md): T010, T013.
- [008-history-performance/tasks.md](008-history-performance/tasks.md): T007.
- [024-responsive-backend/tasks.md](024-responsive-backend/tasks.md): T019, T021.

- 001~008의 미완료 항목은 대부분 실제 창·IME·DPI·포커스 및 원격 Clone 검증이다. 초기 데모·36px 탭·호버 전용 닫기 등 폐기된 조건은 현재 기준으로 바꿔 검증해야 한다. 삭제하거나 완료로 처리하지 않는다.
- 001 T007의 과거 vendor 부재는 현재 Cargo.toml과 vendor 소스가 존재하므로 현재 차단 원인이 아니다. 해당 작업의 실제 GUI 확인 범위는 여전히 남는다.
- 024 T019·T021은 당시 기본 실행 파일 교체 검증 미완료다. 최근 별도 파일명 빌드 통과는 기본 실행 파일 교체 및 실제 앱 검증을 대신하지 않는다.
- 최근 080의 자동 검증 기록은 505개 중 502개 통과·3개 성능 테스트 제외, check·엄격한 Clippy·별도 빌드 통과다. 이번 문서 정리에서 테스트나 GUI를 새로 실행한 결과는 아니다.
- .specify/memory/constitution.md는 미작성 템플릿이다. 이번 정리로 아키텍처 원칙을 임의로 제정하지 않는다.
- 001의 손상 문구는 현재 소스·남아 있는 작업 설명으로 확인한 동작만 복원했다. 손상된 과거 실행 결과를 새 검증 결과로 재구성하지 않는다.

## 유지 방법

후속 요구사항을 반영할 때 해당 spec.md의 현재 기준, 이 색인의 항목·후속 관계, 관련 모델·계약을 함께 갱신한다. 검증 일자·대상·미실행 범위를 기록하고 과거 기록과 구분한다. 이 색인은 소스보다 우선하여 사용자 요구사항을 바꾸는 문서가 아니다.

## 이번 문서 정리 검증

80개 기능 명세에 색인 안내를 연결하고 관련 설계·계약을 보완했다. 색인 및 정리 안내 문서 140개의 로컬 링크 729개를 검사하여 경로 누락 0건, 001의 손상 문구 잔존 0건을 확인했다. 작업 기록 845개·미체크 21개를 보존했다. 그래프 간격·부모 레인 선택·기록 초기 조회 수·파일 크기 제한은 현재 소스 상수 및 구현과 대조했다. 소스코드는 변경하지 않았고 제품 테스트·빌드·GUI 검증은 재실행하지 않았다.
