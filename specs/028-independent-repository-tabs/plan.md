# 구현 계획: 저장소 탭과 파일 조회 분리

## 구조·호출·데이터 흐름 분석

Rust 2024·GPUI·기존 2개 전용 백엔드 작업 스레드를 유지한다. src/ui.rs의 panel_tabs는 파일 탭과 저장소 탭 모두 refresh_panel_data를 호출한다. 이 함수는 파일 선택/데이터/로딩/요청 번호를 초기화하고 PanelRequest를 만든다. src/workspace_backend.rs의 PanelRequest는 태그/스태시 참조 조회와 파일 조회를 함께 수행한다. finish_panel은 참조 목록까지 같이 반영한다. 로컬·원격은 Project 스냅샷을 사용한다. 기존 ReferenceRequest는 참조 행 선택 시 커밋을 찾는 별도 작업이므로 목록 조회와 구별해야 한다.

## 설계와 영향 범위

- src/workspace_backend.rs: 내부 PanelRequest/PanelResult에서 stashes/references를 분리한다. RepositoryListRequest/RepositoryListResult는 요청 번호·프로젝트·경로·스태시 여부와 이름/오류만 보유하고 기존 repository::panel_references를 호출한다. 파일/그래프 계산을 수행하지 않는다. 공개 백엔드 함수와 실제 파일 형식은 변경하지 않는다.
- src/ui.rs: 저장소 목록용 revision/pending/ReadSlot/waiting을 추가한다. refresh_repository_list는 이 상태와 저장소 선택/참조 목록만 초기화하며 Tags/Stashes에만 요청을 예약한다. Local/Remote는 기존 스냅샷을 사용한다.
- select_repository_tab은 현재 탭이면 무시하고 탭이 바뀌면 저장소 스크롤 드래그·행 선택·이전 참조 선택 요청 및 이전 목록 요청만 초기화한다. 파일 데이터·선택·스크롤·요청 번호·로딩 핸들·진행 중 파일 조회를 건드리지 않는다.
- dispatch_pending은 별도 ReadSlot으로 참조 목록을 제출한다. finish_repository_list는 목록 요청 번호·프로젝트·경로·현재 탭·열림 상태를 검증한다. 파일 완료는 참조 목록을 반영하지 않아 늦은 파일 결과가 새 탭을 덮지 않는다.
- 전체 refresh_panel_data와 invalidate_requests는 프로젝트 문맥 변경에 사용한다. 커밋/파일 모드/활성 파일 변경은 후속 요청에 따라 파일 전용 갱신으로 분리한다. 작업 트리·히스토리 150ms와 상세 항목 및 백엔드 실행 상한은 변경하지 않는다.
- 저장소 목록의 빈 결과 안내는 별도 repository_waiting을 사용한다. 태그·스태시에는 후속 요청에 따라 기존 공통 List의 지연 안내를 연결한다. 테스트 구동기는 새 요청도 별도 스레드에서 실행해 실제 완료 함수를 사용한다.

## 헌법과 제약

헌법은 미작성 템플릿으로 규범 검사를 생략한다. AGENTS.md의 UI/백엔드 책임·실제 동작 확인·기존 데이터/선택 호환성·최소 변경을 준수한다. 변경 파일은 src/ui.rs와 src/workspace_backend.rs 및 기능 문서로 제한한다. 새 의존성이나 캐시 정책 변경은 없다.

## 검증

재현 테스트를 먼저 작성한다. 두 파일 모드의 선택/뷰어/스크롤/요청/로딩/취소 토큰 보존, 진행 중 파일 요청 완료, 태그/스태시 단독 gateway 호출, 목록 결과의 역순·현재 오류·탭/프로젝트/경로/닫기 무효화, 이전 참조 행 선택 요청 무효화를 검증한다. 기존 stale 파일 결과 테스트에서는 제거된 참조 필드 주입만 새 단독 목록 테스트로 대체하고 파일 결과 검증은 유지한다. 전체 test, fmt, check, clippy, build를 실행하고 GUI와 실행 파일 잠금 제한을 보고한다.

## 태그·스태시 공통 로딩 연결

src/ui.rs만 보완한다. 기존 repository_waiting/revision/pending/ReadSlot이 이미 독립되어 있어 새 조회나 타이머는 필요 없다. 별도 repository_loading: ListLoading을 보존하고 refresh_repository_list에서 초기화 후 유효한 Tags/Stashes 요청 시작 시 begin한다. invalidate_requests와 유효한 finish_repository_list는 reset한다. dispatch_pending의 목록 작업에 요청별 completion 신호를 캡처해 실제 완료 직후 true를 저장한다.

branches의 VirtualList에 핸들을 연결하고 이름이 없어도 repository_waiting이면 목록을 렌더링하여 공통 타이머를 예약한다. 오류·빈 결과는 완료 후 기존 표시를 유지한다. 로컬/원격은 핸들이 비활성 상태다. 기존 패널 ListLoading과 분리되어 저장소 탭 전환으로 파일 조회나 로딩 수명을 변경하지 않는다.

먼저 두 탭의 경계·완료/오류·이전 요청·전환 격리 테스트를 작성한다. 기존 공통 List 타이머/표시와 요청 분리 회귀를 재사용하고 전체 test/fmt/check/clippy/build를 실행한다. 실행 중 exe 교체와 GUI 검증 제한을 보고한다.

## 파일 탭·커밋 선택의 반대 방향 결합 해소

원인은 refresh_panel_data의 전체 invalidate_requests·저장소 선택/데이터 초기화·refresh_repository_list 재예약이다. files 탭과 select_commit 및 viewer_changed가 모두 이를 호출한다. prepare_history는 프로젝트 열기/전환/닫기 이후 전체 갱신의 진입점이다.

- src/ui.rs에 invalidate_file_request와 invalidate_reference_request를 추출한다. 전체 invalidate_requests는 저장소 목록·참조 선택·파일을 모두 무효화하지만 파일 전용 refresh_file_data는 파일 ReadSlot/요청 번호/데이터/선택/로딩과 해당 파일 스크롤 드래그만 초기화한다.
- refresh_panel_data는 전체 스크롤 드래그 정리·참조 선택 무효화·파일 갱신·저장소 목록 갱신을 조합하고 prepare_history 등 프로젝트 문맥 경로에서 유지한다.
- select_file_tab은 파일 모드 변경 시 refresh_file_data만 호출한다. 같은 탭은 추가 갱신 없이 유지한다. select_commit은 기존 참조 선택 요청/행 선택을 무효화하고 파일 갱신만 수행한다. viewer_changed도 활성 작업 트리의 파일 갱신만 수행하되 기존 파일 식별 선택 보존을 유지한다.
- select_project는 시작 화면을 먼저 해제한 뒤 prepare_history를 호출하여 유효한 새 프로젝트의 저장소 목록 요청이 생략되지 않게 한다. 기존 순서는 시작 화면에서 복귀할 때 목록 예약의 활성 화면 가드에 걸릴 수 있다.
- 실제 백엔드/공통 List/데이터 형식은 변경하지 않는다. 저장소 전용 완료·오류·진행 중 신호와 스크롤은 그대로 유지한다.

테스트를 먼저 작성하여 네 저장소 탭의 데이터·선택·스크롤·요청·로딩 보존, 진행 중 태그/스태시 결과 반영, 커밋 선택의 이전 참조 선택 무효화, 프로젝트 전환/시작 화면 복귀의 전체 갱신, 기존 파일 변경 식별 보존을 검증한다. 전체 test/fmt/check/clippy/build와 수렴 점검을 수행한다.
