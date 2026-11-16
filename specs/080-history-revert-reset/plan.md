# 구현 계획: Git 작업 확인·전역 새로고침·찾기

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

## 기술 맥락과 기존 구조

Rust 2024·GPUI 0.2.2·git2 0.21 Windows 데스크톱 앱이다. ui.rs의 기록 컨텍스트 메뉴→start_toolbar_action→execute_toolbar_action→ToolbarRequest→repository 계층→성공/오류 후 refreshed_project를 사용한다. start_toolbar_action은 이미 detached 체크아웃 확인을 분리하고 있으므로 새 작업 확인도 같은 위치에 연결한다. Pull 병합은 start_sync에서 별도로 확인한다. 기존 EncodingDialog의 선택 행과 form_controls의 체크박스·Dialog 버튼을 재사용한다. 의존성·공개 세션 형식 변경은 없다.

## 설계와 영향

- src/commit_history_actions.rs를 repository 내부 모듈로 추가한다. CommitHistoryRequest는 target·expected_head·expected_reference를 보관한다. ResetMode 세 가지와 Revert/Reset 함수를 제공한다. 실제 저장소·HEAD·참조·대상·진행 상태·충돌을 검사한 뒤 Git CLI를 숨긴 창·인자 배열·비대화형 stdin으로 실행한다. 기존 Git CLI 실행과 같은 Git 환경 분리를 사용한다. no-commit·기본 커밋·병합 첫 부모·Soft/Mixed/Hard의 정확한 Git 의미를 재사용하고 실패의 충돌 경로를 보존한다.
- src/repository.rs와 backend.rs에서 기존 I/O 경계에 등록한다. workspace_backend.rs의 ToolbarAction과 오류 경로 수집에 연결하여 성공/실패 모두 기존 새 스냅샷 갱신을 사용한다.
- src/git_action_dialog.rs는 UI 전용 옵션·이벤트를 가진 공통 팝업이다. Revert는 no-commit, Reset은 인코딩과 같은 선택 목록, 나머지는 대상 설명만 표시한다. 확인·취소·Escape·Tab·방향키·포커스 복원을 처리하고 확인 전 작업을 실행하지 않는다.
- ui.rs는 프로젝트·경로·현재 HEAD/브랜치·선택 커밋·작업을 보관한다. modal 표시 중 배경 작업을 차단한다. 확인 이벤트의 엔티티와 문맥을 검증하고 한 번 소비한 뒤 기존 실행 함수를 호출한다. Revert/Reset 대상은 기록 메뉴에만 추가한다. start_toolbar_action에서 병합·리베이스·Push도 확인을 거치고 start_sync의 Pull 병합을 같은 팝업으로 연결한다.
- Revert 충돌 시 작업 파일·Git 상태를 보존하고 경로와 Git 계속/취소 안내를 표시한다. 추가 요청에 따라 기존 작업 툴바에 Revert 계속/취소 UI를 연결한다. 기본 no-commit 해제, Reset Mixed, 병합 커밋 첫 부모를 화면에 명시한다.
- 사용자가 철회한 현재 HEAD 표시 수정은 제외한다. 별도 후속 요청인 부모 레인 선택 수정은 마지막 그래프 계획을 따른다.

## 헌법·설계 점검

헌법은 미작성 템플릿이므로 규범 검사 생략. 기존 UI→비동기 요청→Git 계층을 유지하고 실제 사용자 저장소에는 자동 검증 작업을 실행하지 않는다. Hard는 구현된 옵션이며 테스트는 임시 저장소에만 수행한다. 확인 전에 파괴적인 동작을 실행하지 않는다. 새 공개 API의 기존 호환성을 깨지 않는다.

## 검증과 순서

명세·명확화→계획·조사·모델·계약→작업→일관성 분석→Git 의미·오류·옵션·취소·오래된 문맥 테스트→구현→전체 회귀·check·엄격한 Clippy·새 파일 포맷·별도 빌드→수렴. 임시 Git 저장소에서 Revert 두 모드·최초/병합 커밋·충돌·dirty 보존·잠금·HEAD 변경과 Reset 세 모드의 HEAD·인덱스·작업트리를 검증한다. UI의 옵션 기본값·키보드·단일 소비·문맥 변경은 순수 상태 테스트로 검증한다. 실제 GUI 미검증 범위는 별도 보고한다.

## Detached HEAD 선택 보완
변경 전 start_history_checkout은 후보 하나면 곧바로 브랜치를 체크아웃하고 여러 후보일 때 선택창을 띄웠다. 후보가 있으면 항상 선택창을 띄우며 마지막 행에 Detached HEAD를 추가한다. 후보 계산 함수의 기존 계약은 유지하고 selected==후보 수를 detached 값으로 해석한다. 후보·커밋·프로젝트 재검증 후 기존 detached 확인창을 거치게 한다. 기존 브랜치 선택·취소·오래된 후보 검증을 보존하며 회귀 테스트를 추가한다. FR-009를 T011로 연결하고 최종 검증 전에 완료한다.

## 전역 F5·찾기 탭 설계 보완

F5는 workspace-content의 capture_key_down에서 처리하여 자식 DiffViewer보다 먼저 실행한다. ToolbarAction::Refresh로 기존 비동기·갱신 경로를 재사용하며 Git 명령이나 Fetch는 실행하지 않는다. 선택 파일 경로를 작업에 보관하고 결과 후 패널 요청에 전달한다. 뷰어 강제 갱신은 패널 조회가 완료된 뒤 한 번 수행한다. 기존 파일 전용 F5 경로는 제거하고 테스트를 전역 의미로 갱신한다.

찾기는 old/src/git_search.rs의 검색 의미를 참고해 새 repository_search 모듈로 구현한다. ReadSlot 취소 checkpoint·경로 검증·현재 저장소 검증을 추가하고 불필요한 old 창/테마/단축키는 옮기지 않는다. 검색 UI는 별도 SearchPanel 엔티티로 분리하여 입력·리비전·결과·ListLoading·선택을 관리하고 Commit/File 이벤트만 Workspace에 보낸다. Workspace는 찾기 탭에 엔티티를 배치하고 프로젝트 변경 시 새 엔티티로 교체한다. 파일 이벤트는 기존 FileHistoryDialog를 재사용하고 커밋 이벤트는 기존 reference_snapshot을 대상 OID로 사용해 전체 초기 목록 밖 커밋도 찾는다. 새 세션 형식과 외부 의존성은 없다.

추가 일관성 분석: FR-010~014는 T012~014에 연결한다. 검색은 읽기 전용이며 새 쓰기 작업의 확인과 충돌하지 않는다. F5는 확인창에서 차단되며 현재 HEAD 표시 수정 철회와도 별개인 정상 전체 데이터 재조회이다. 추가 필수 질문 없음.

## Revert 계속·취소와 입력 기능 보완

분석: RepositoryWorkState는 Revert/RevertSequence를 이미 읽지만 history_operation_toolbar에는 버튼이 없다. Git 명령 공통 함수와 전역 진행/갱신은 기존 commit_history_actions 및 ToolbarAction을 재사용한다. 임시 저장소 실험에서 Git의 단일 no-commit Revert 충돌을 --continue하면 커밋이 생성됨을 확인했다. 따라서 원래 no-commit 의도를 앱에서 보존해야 한다.

설계: 앱 Revert 시작 시 .git의 전용 작은 JSON 문맥에 예상 HEAD·참조·대상·no-commit을 기록하고 완료/취소 후 제거한다. 진행 중 문맥은 실제 REVERT_HEAD와 검증한다. 일반 계속은 Git --continue를 비대화형 에디터로 실행한다. no-commit 계속은 해결된 인덱스를 검증한 뒤 --quit으로 Git 진행 상태만 마무리하여 변경을 Stage 상태로 남긴다. 외부 Git Revert는 문맥 없이 표준 --continue/--abort를 사용한다. 오류·잠금은 상태를 보존하고 원인을 알린다. 새 함수와 상태 판별을 repository/backend에 등록한다.

기존 취소 확인창의 작업 종류를 Merge·Rebase·Revert 열거형으로 확장하고 공통 포커스·확인 단일 소비·문맥 검증을 유지한다. Revert 계속·취소를 기존 소형 툴바 버튼과 ToolbarRequest에 연결한다. 메뉴 표기만 Merge·Rebase로 바꾼다.

TextInput에는 SelectHome/SelectEnd action과 두 입력 key context의 바인딩·handler를 추가한다. 기존 line_bounds와 select_to를 재사용하여 선택 기준점을 유지한다. 선택 범위 연산은 기존 select_to에서 작은 순수 함수로 추출해 한글·이모지·역방향·빈 줄 회귀를 검사한다. 기존 Home/End·문서 이동·입력 편집 정책은 변경하지 않는다.

SearchPanel의 입력 영역에 Enter handler를 추가하여 포커스·활성·IME·수정키를 검사한 뒤 기존 search 경로를 사용한다. 검색 버튼·컨텍스트 메뉴·세대 격리는 유지한다.

검증: 실제 임시 Git 충돌의 계속·취소·no-commit 재시작·충돌/HEAD/잠금 실패와 ToolbarResult 갱신, UI 버튼/취소 조건, 선택 범위/검색 키 경계를 자동 검증한다. 이후 전체 테스트·check·엄격한 Clippy·새/변경 파일 포맷·별도 빌드와 수렴을 수행한다. 헌법은 미작성 템플릿이며 기존 책임·의존성·공개 계약을 유지한다. 새 문맥은 앱 내부 Git 작업 정보이며 기존 세션 형식은 바꾸지 않는다.

찾기 추가 변경: SearchPanel의 선택 콜백이 커밋 이벤트를 직접 보내며 커밋 우클릭은 메뉴를 열지 않는다. 파일 메뉴에서 복사 항목을 없애고 파일 이력만 남긴다. 비동기 커밋 이동 응답은 세대뿐 아니라 현재 선택 커밋도 검사한다. FR-022가 FR-013의 이전 활성화 정책을 대체하며 새 단축키는 추가하지 않는다.

## 그래프 부모 레인 선택 보완

같은 부모를 기다리는 선이 여러 개일 때 기존 waiting_lanes HashMap 수집이 나중 항목으로 값을 덮어써 가장 오른쪽 레인을 선택했다. 실제 사례에서 첫 레인 0 대신 f5e33c1 쪽 레인 6을 선택한 원인이다. 앞선 수정은 이를 합류 자체의 문제로 오해해 새 레인을 추가했다. 사용자 정정에 따라 독립 레인 정책을 폐기하고 기존 입력 중 부모 ID가 같고 to_lane이 최소인 항목을 선택한다. 없을 때만 새 레인을 만든다. 저장소 조회·정렬·렌더러·첫 부모 처리는 유지한다.

명세·명확화·계획·작업 일관성 분석: FR-023 → T025~T027. 중복 대기 레인의 순서 및 색을 함께 검증하고 기존 페이지 테스트를 재사용한다. 기존 독립 레인 테스트는 잘못된 수용 기준이므로 가장 왼쪽 레인 재사용 테스트로 교체한다. 실제 git2 순서로 7817dfdf의 출력이 레인 0인지 확인하고 전체 테스트·Clippy·빌드를 실행한다. 헌법은 미작성 템플릿이며 추가 질문은 없다.
