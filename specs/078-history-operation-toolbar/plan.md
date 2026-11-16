# 구현 계획: 기록 작업 상태 툴바

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

## 요약과 기술 맥락

Rust 2024·GPUI 0.2.2·git2 0.21 Windows 앱이다. 기존 저장소 스냅샷·PanelRequest·ToolbarRequest·SyncRequest·CommitWorkRequest를 재사용한다. 새 의존성·저장 형식·공개 API 변경 없음.

## 기존 구조와 호출·데이터 흐름

- src/repository.rs의 snapshot_with_target이 저장소를 열어 Project를 만든다. RepositoryWorkState에 작업 종류·인덱스 충돌·조회 오류를 담고 Project에 추가한다. 기존 스냅샷에서 같은 핸들로 상태를 수집한다.
- src/backend.rs의 install 매크로가 저장소 I/O를 감싸므로 작업 상태 읽기도 이 경계에 등록한다.
- src/workspace_backend.rs의 PanelRequest는 파일·기록을 백그라운드에서 읽는다. 작업 상태도 함께 읽는다. 초기 열기와 툴바·Pull·커밋 결과의 refreshed_project도 Project 상태를 전달한다.
- src/ui.rs의 finish_panel은 프로젝트·revision·경로·선택을 검증한 뒤 Project 작업 상태를 갱신한다. 실패는 확인 실패로 전달한다. 렌더링 중 I/O는 금지한다.
- 기존 sync_busy와 commit_busy가 실행 보호를 담당한다. sync_operation 문구만 추가해 기존 플래그의 수명에 맞춘다. 실행 문구가 저장소 스냅샷보다 우선하며 결과 처리 시 비운다.
- history는 제목·가상 목록으로 구성된다. 제목 아래에 조건부 단일 행을 추가하고 목록 영역·스크롤·열 크기 로직은 유지한다. 긴 문구는 줄임 처리한다.

## 헌장 검사

헌장은 미작성 템플릿이다. 기존 UI·비동기·Git 계층과 최소 변경 원칙을 유지하며 설계 전후 위반 없음.

## 설계와 영향 범위

소스 수정은 위 네 기존 파일 및 내부 테스트다. 상태 필드는 영속 세션에 저장하지 않는다. 병합·리베이스·체리픽·되돌리기·이분 탐색·패치를 표시한다. 충돌 여부에 따라 해결 필요 또는 완료 대기를 표시한다. 외부 상태는 기존 갱신 시 반영하며 실시간 감시와 병합 이외의 Git 조작 버튼은 범위 밖이다.

## 검증 전략

명세·명확화 후 모델·계약·작업을 작성하고 읽기 전용 분석을 통과한 뒤 상태 전이·표시 우선순위·프로젝트 경계·조회 실패 테스트를 먼저 작성한다. 실제 충돌 저장소로 읽기를 검증하고 전체 테스트·cargo check·엄격한 clippy·별도 실행 파일 빌드 후 수렴한다.

## 병합 버튼 확장 계획

- 기존 open_commit_dialog → prepare_commit → commit_staged가 충돌 거절·병합 메시지·두 부모·상태 정리를 담당하므로 재사용한다.
- src/ui.rs의 툴바가 cx를 받아 공용 dialog::button으로 두 버튼을 렌더링한다. 병합 상태에서만 표시하고 repository_busy·panel_waiting·조회 오류·충돌을 검사한다.
- 별도 병합 취소 확인 상태는 프로젝트·경로·HEAD를 캡처한다. 공용 Dialog와 포커스·Tab·Escape·취소 기본값을 사용하고 repository_busy에 포함한다. 확인 상태를 먼저 소비한 뒤 유효 대상을 재검사한다.
- src/synchronization.rs에 abort_merge(path, expected_head)를 추가한다. 로컬 Git의 merge --abort 지원을 확인했다. libgit2의 강제 reset 대안은 로컬 변경을 버릴 수 있어 제외하고 표준 취소를 사용한다. Windows 콘솔을 숨기며 셸 문자열을 조합하지 않는다. 병합 여부·예상 HEAD·잠금을 확인하고 오류 시 강제 fallback을 하지 않는다.
- src/repository.rs·src/backend.rs에 기존 I/O 경계로 노출하고 ToolbarAction::AbortMerge로 기존 진행 표시·오류·성공 실패 갱신을 공유한다. 새 의존성과 아키텍처 변경 없음. 헌장 충돌 없음.
- 검증: 버튼 및 확인 소비 단위 테스트, 실제 충돌·해결 후 취소·로컬 변경 보존·잠금·HEAD 변경·정상 저장소 거절, 기존 병합 커밋 및 전체 회귀를 수행한다.

## 병합 버튼 표시 개선 계획

호출·영향 분석: merge_abort_dialog는 dialog::button의 기본 32px·좌우 16px 패딩을 사용한다. history_operation_toolbar도 같은 버튼을 사용하여 높이가 크고, hover 배경 RAISED가 툴바 배경과 동일해 취소 버튼의 효과가 보이지 않는다. 기존 status_button은 소형 배지와 활성 시 SELECTED hover를 공용으로 사용한다.
수정은 src/ui.rs에 한정한다. 확인창 버튼 문구만 확인·취소로 바꾸고 두 툴바 버튼을 status_button으로 재사용한다. 높이 24px·좌우 8px 패딩·글자 12px로 줄인다. 두 버튼의 활성화 검사·콜백·확인 기본 포커스와 키 처리는 그대로 둔다. 공용 컴포넌트 자체와 다른 화면을 변경하지 않는다.
헌장은 미작성 템플릿이며 기존 구조·최소 변경 원칙과 충돌 없음. 별도 추상화·의존성은 추가하지 않는다. 기존 버튼·확인 보호 회귀 테스트와 컴파일·엄격한 정적 분석·별도 빌드로 검증한다. 표시 문구를 복제하는 새 테스트는 추가하지 않는다.
