# 구현 계획: 커밋상세 목록 항목

## 기존 구현과 호출 관계

Rust 2024와 GPUI를 유지한다. src/ui.rs의 files는 VirtualList와 별도로 selectable_row를 만들어 탭 아래에 고정한다. 실제 변경 파일은 changed_files 및 viewer_files에서 같은 인덱스로 대응하며 file_selection은 실제 파일 번호만 저장한다. 선택이 없으면 commit_detail_selected와 viewer_target이 기존 상세 자동 선택을 제공한다. refresh_panel_data는 목록/선택을 지우고 공통 ListLoading을 시작하며 finish_panel은 성공/오류를 반영하고 로딩을 초기화한다. 이벤트는 project·panel_revision·file_history·대기 여부로 검증한다.

## 설계 및 영향 범위

src/ui.rs만 변경한다. file_list_has_detail로 조회 완료·히스토리 성공·유효 커밋·활성 프로젝트를 판단한다. file_list_len은 상세 행 여부만큼 표시 항목 수를 더하고 file_list_selected_row는 기존 실제 파일 선택 번호를 화면 번호로 변환한다. select_file_list_row는 기존 문맥 검증 후 첫 상세 항목을 기존 select_commit_detail로 보내고 나머지는 오프셋을 제거하여 file_selection을 갱신한다. 실제 ChangedFile/FileChange에 가짜 항목을 삽입하지 않는다.

VirtualList의 렌더 범위를 표시 번호 기준으로 해석하여 0번 상세 행에는 기존 커밋 아이콘과 이름을, 이후 행에는 실제 파일 정보를 렌더링한다. 모든 행은 공통 selectable_row를 거치므로 가상화·스크롤·강조가 일치한다. 목록 바깥 고정 행과 관련 import를 제거한다. 목록의 비어 있음/렌더 여부는 표시 행 수를 사용하되 배지는 실제 파일 수를 유지한다.

기존 커밋 선택에 따른 상세 뷰 자동 선택과 viewer_target은 유지한다. select_commit_detail은 실제 상세 행이 있는 경우에만 사용자 행 이벤트를 수용한다. 오류에서는 상세 행이 없어도 기존 뷰어의 오류/상세 대상 처리는 유지한다. 이전 pinned 명칭 테스트는 새 의미로 이름을 바꾸고 행 가용성과 새 이벤트 거절을 검증한다.

## 아키텍처와 헌법

헌법은 미작성 템플릿으로 별도 규범 검사를 생략한다. AGENTS.md의 최소 변경·기존 API/데이터 보존·계층 책임을 준수한다. UI 합성 항목만 추가하며 공통 List·로딩 상태·백엔드·파일 데이터·스테이징은 변경하지 않는다. 새 의존성은 없다.

## 검증

먼저 표시 항목 수와 선택 매핑 테스트를 작성한다. 대기·성공·빈 파일·오류·유효 커밋 없음·작업 트리, 상세/첫/마지막 파일 및 동명 파일의 viewer_target, 이전 revision/다른 프로젝트/탭 이벤트를 검증한다. 기존 커밋 자동 선택·상세 오류·로딩·작업 트리와 쓰기 후 선택 보존 회귀를 유지한다. 전체 test, fmt, check, clippy, build를 실행한다. 기본 exe가 실행 중이면 강제 종료하지 않고 최신 링크 결과를 별도 경로에 준비하고 제한을 보고한다. GUI 검증은 실제 수행 여부를 구분한다.
