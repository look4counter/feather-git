# 구현 계획: 저장소 컨텍스트 결함 수정

## 기술 맥락

Rust 2024, GPUI 0.2.2, git2 0.21.0 및 기존 Windows 의존성을 사용한다. 신규 의존성·공개 API 형식 변경은 없다.

## 기존 구현 분석

- commit_history_actions.rs는 선택한 repo를 검증하지만 Git에는 -C workdir만 전달한다. synchronization.rs의 abort_merge와 rebase_actions.rs의 abort_rebase에도 같은 패턴이 있다.
- repository.rs와 diff_git.rs는 동위 메타데이터를 제외하지만 repository_search.rs는 원시 미추적 목록을 별도로 검색한다.
- workspace_backend.rs는 터미널의 current_dir만 바꾸어 Git 디렉터리 선택을 잃는다.
- vendor/gpui의 Windows file_open_dialog는 폴더 선택 시 FOS_PICKFOLDERS만 추가하며 숨김 강제 표시 옵션이 없다.

## 설계와 영향 범위

1. src/repository.rs에 내부 Git 명령 생성 함수를 두고 --git-dir·--work-tree·-C를 실제 경로로 전달한다. 기존 네 Git 경로 환경변수 제거·비대화형 입력·Windows 숨김 실행은 보존한다. 이력 변경과 Merge/Rebase 중단에서 재사용한다.
2. 기존 중복 메타데이터 디렉터리 수집을 src/repository.rs의 공통 함수로 모으고 src/diff_git.rs·src/repository_search.rs가 재사용한다. 검색은 파일 내용 조회 전에 메타데이터 경로를 제외한다.
3. src/workspace_backend.rs에서 선택 repo를 연 상태로 작업 폴더를 결정하고, 터미널 명령에만 GIT_DIR·GIT_WORK_TREE를 설정하며 GIT_INDEX_FILE·GIT_COMMON_DIR를 제거한다. 베어 저장소는 GIT_WORK_TREE를 제거한다.
4. vendor/gpui/src/platform/windows/platform.rs의 폴더 옵션에 FOS_FORCESHOWHIDDEN을 추가한다. 공용 PathPromptOptions 필드를 추가하지 않는다.
5. src/backend.rs에는 공통 메타데이터 수집의 기존 I/O 경계를 등록한다.

## 헌장 점검

constitution.md는 미작성 템플릿이다. 사용자 AGENTS.md에 따라 기존 UI·저장소·백그라운드 책임을 유지하며 주요 아키텍처·의존성 변경은 없다. 설계 전후 충돌이 없다.

## 검증

- 기존 기능 테스트 파일 안에 잘못된 저장소 변경을 먼저 재현한다.
- Reset/Revert의 단독·동위·상위 저장소 및 계속·중단을 확인한다. Merge/Rebase 중단도 .git2 실제 충돌 상태에서 검증한다.
- 검색에서 실제 .git2·사용자 지정 bare 메타데이터를 제외하면서 일반 파일을 남기는지 검증한다.
- 실제 터미널 명령 구성의 환경을 검사하고 같은 환경의 자식 Git 명령이 올바른 저장소를 가리키는지 확인한다.
- Windows 폴더 옵션의 플래그를 코드로 검토하고 실제 Windows 빌드로 확인한다. 단순 플래그 추가를 복제하는 테스트는 만들지 않는다.
- 전체 cargo test, cargo check --all-targets, cargo clippy --all-targets 및 diff 검사를 실행한다.
- 이전 QA 보고서에 수정 결과와 실제 UI·원격 인증 미검증을 갱신하고 수렴 검사한다.

## 구현 중 확인한 경로 제약

Windows Git은 GIT_DIR 환경변수의 확장 경로 접두사에 대해 저장소를 찾지 못하는 경우를 실제 자식 명령으로 확인했다. 내부 경로와 저장소 식별은 정규화 경로를 유지하고 Git CLI 인수·터미널 환경에 전달할 때만 기존 path_display::display_path로 일반 드라이브/UNC 경로를 사용한다. 한글·공백·특수문자 경로와 Windows 표준 정규화 경로를 회귀 테스트에 포함했다.
