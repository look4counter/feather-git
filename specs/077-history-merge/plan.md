# 구현 계획: 커밋 기록 병합

작성일: 2026-11-16. 명세: [spec.md](spec.md)

## 요약 및 기술 맥락

Rust 2024, GPUI 0.2.2, git2 0.21.0 기반 Windows 앱이다. 새 의존성과 저장 형식 변경 없이 기존 비동기 툴바 실행을 재사용한다.

## 기존 구조·데이터 흐름·영향 범위

- src/ui.rs: open_checkout_menu가 우클릭 행을 선택하고 checkout_menu_target이 오래된 선택·작업 상태를 검증한다. 기록에만 병합을 추가하고 같은 검사로 실행을 보호한다.
- src/workspace_backend.rs: ToolbarAction::MergeCommit과 실행 분기를 추가한다. ToolbarRequest는 성공·실패 모두 refreshed_project를 실행하므로 파일·기록 갱신을 공유한다.
- src/repository.rs: merge_commit을 기존 Git 기능 경계에서 노출한다.
- src/synchronization.rs: with_locked_index, fast_forward, merge의 잠금·로컬 변경·충돌 처리를 재사용한다. 병합 메시지를 인자로 받아 Pull 메시지를 유지한다.
- 기존 committing 모듈이 충돌 해결 후 두 부모 커밋을 완성하므로 새 충돌 처리 계층을 만들지 않는다.

## 헌장 검사

constitution.md는 미작성 템플릿으로 유효 원칙이 없다. AGENTS.md의 기존 계층·데이터 보존·최소 변경 원칙을 적용한다. 설계 전후 위반 없음.

## 구현 및 검증 순서

명세·명확화 → 조사·모델·계약 → 작업 목록 → 읽기 전용 일관성 분석 → 테스트와 구현 → 전체 테스트·컴파일·엄격한 정적 분석·빌드 → 수렴 검사 순으로 진행한다. 저장소 테스트는 정상·충돌·로컬 변경·잘못된 상태를 포함한다.

## 프로젝트 구조

문서는 specs/077-history-merge에 두고 구현은 위 네 파일 및 내부 테스트에 한정한다. 아키텍처·공개 호환성 변경 없음.

추가 호출 경계 확인: src/backend.rs의 install 매크로가 repository 함수를 감싸므로 merge_commit 서명을 같은 I/O 경계에 등록한다. 기존 실행 정책을 유지하며 FR-004와 T004에 포함한다.

## 병합 실패 경로 진단 계획

### 기존 구현 분석

src/synchronization.rs의 fast_forward·merge는 checkout/merge의 모든 실패를 로컬 변경 또는 잠금으로 묶고 경로를 버린다. Git2의 Conflict는 -13, Locked는 -14이며 서로 다르다. 기존 stash_actions.rs는 CheckoutBuilder::notify와 인덱스 경로를 수집하여 StashApplyFailure를 전달한다. workspace_backend::ToolbarResult는 구조화된 conflict_paths를 보유하고 ui.rs는 유효 스태시 결과만 Alert::set_paths에 연결한다. Pull은 SyncResult로 별도 전달되므로 경로 필드를 추가한다.

### 설계와 영향 범위

- src/synchronization.rs에 MergeFailureKind 및 MergeFailure를 추가하고 원인과 경로를 보존한다. fast_forward·merge의 checkout CONFLICT 알림을 수집하고 성공 후 인덱스 충돌을 별도로 분류한다. 잠금 오류에는 추측 경로를 넣지 않는다. with_locked_index의 파일 존재와 권한 실패 문구를 구분한다.
- 기존 stash_actions.rs의 인덱스 충돌 수집기를 repository.rs의 공용 내부 도우미로 옮겨 중복 없이 두 기능에서 재사용한다. libgit2가 작업 인덱스를 잠금 경로에 연결한 상태에서도 해당 핸들을 읽는다.
- repository.rs와 backend.rs가 오류 타입을 노출한다. workspace_backend.rs는 ToolbarResult와 SyncResult에 오류의 경로를 추출한다. ui.rs는 기존 유효 대상 검사를 재사용하여 공통 Alert 목록에 전달한다. 일반 오류·스태시 동작은 보존한다.
- 경로는 PathBuf로 유지하고 표시 시 기존 display_path를 사용한다. 오류창 목록과 스크롤은 재사용한다. 새 의존성·세션 형식·공개 API 호환성 변경 없음.

### 헌장과 검증

헌장은 미작성 템플릿이다. 기존 계층·최소 변경·사용자 데이터 보존 원칙에 위반 없음. 명세·계획·작업 일관성 분석 후 재현 테스트를 먼저 추가한다. 임시 저장소의 stage/unstage·다중 파일·한글·공백·비관련 수정·빠른 전진·실제 잠금·진짜 병합 충돌과 작업 결과 전달을 검증한다. 전체 테스트·컴파일·엄격한 clippy·별도 빌드 후 수렴한다. nonmac은 수정하지 않는다.

### 재현에서 확인한 사전 차단 경로 보완

libgit2 merge.c의 merge_check_index·merge_check_workdir가 checkout 알림 이전에 Conflict를 반환함을 로컬 설치 소스에서 확인했다. 알림만으로 경로를 알 수 없는 분기 병합은 실패 시 merge_commits의 인메모리 결과를 계산해 같은 기준으로 진단한다. 현재 HEAD와 병합 결과 사이의 변경·충돌 경로에 한해 index→workdir 차이를 검사하며, staged 변경은 결과 인덱스와 실제로 다른 경로만 포함한다. 전체 dirty 목록을 원인으로 추정하지 않는다. 진단 비교 중 오류가 나면 원래 오류와 진단 실패를 함께 알리고 경로를 만들지 않는다. 인덱스·작업 트리·참조에 기록하지 않는다. 빠른 전진의 동일한 incoming 내용은 정상 처리될 수 있으므로 빠른 전진 차단 재현은 실제로 다른 로컬 내용으로 검증하고 분기 병합은 사용자와 같은 incoming 내용으로 검증한다.
