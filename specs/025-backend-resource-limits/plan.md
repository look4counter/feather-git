# 구현 계획: 백엔드 자원 사용 제한

## 기술 환경과 기존 구조 분석

Rust 2024, GPUI 0.2.2 로컬 공급본, gix 0.90, git2를 유지한다. 새 의존성은 없다.
UI의 dispatch_pending과 DiffViewer::set_target은 backend::spawn으로 동기 함수를 공용 실행기에 제출한다. backend::run은 중첩 호출을 제외하고 각 I/O 앞에서 sleep한다. 이전 요청은 결과 적용 시에만 무시하며 실행 취소와 상한이 없다. WindowsDispatcher는 모든 공용 작업을 High로 제출한다. gix 상태 조회는 내부 thread_limit 기본값이 없다. SessionWriter는 별도 단일 작성기로 저장을 직렬화한다.

## 헌법 및 아키텍처 검토

헌법 파일은 미작성 템플릿이므로 규범 검사에서 제외한다. AGENTS.md의 계층 책임·API·데이터 호환성·최소 변경 원칙을 적용한다. UI는 요청 수명, 백엔드는 실행·취소·지연, 저장 작성기는 쓰기 순서를 담당한다. 기존 경계와 공개 함수 서명은 유지한다.

## 설계와 영향 범위

- src/backend_worker.rs: 2개의 전용 스레드, 조건변수 대기열, 완료 Future와 Waker를 구현한다. 일반 작업과 조회 작업을 구분한다. 취소된 대기 조회를 제출 때 제거하여 마지막 요청만 유지한다. 작업 밖에서 잠금을 풀고 실행한다.
- 조회 소유자 ReadSlot은 취소 토큰을 보유하고 교체·취소·소멸 시 조건변수를 깨운다. 읽기 작업만 취소 문맥에 진입한다. gateway 지연에서 취소를 감지하면 내부 전용 unwind 값을 전달하고 조회 경계에서만 None으로 변환한다. 실제 패닉은 원래대로 전달하며 작업 스레드는 계속 살아 있다. panic hook을 호출하지 않는 resume_unwind를 사용한다.
- src/backend.rs: 일반 spawn은 전용 풀로 전달하고 공용 실행기에서는 완료 Future만 기다린다. run은 읽기 문맥에서 취소 가능한 지연을 사용하되 기존 I/O별 지연과 중첩 호출을 유지한다.
- src/ui.rs, src/diff_viewer.rs: 패널·참조·뷰어에 독립적인 ReadSlot을 보유하고 새 선택·무효화·비활성화 시 취소한다. 기존 revision/generation 검사는 유지한다. 취소 결과는 UI에 반영하지 않는다.
- vendor/gpui/src/platform/windows/dispatcher.rs: 작업 제출을 Normal로 바꾼다. 프로세스·시스템 설정은 바꾸지 않는다.
- src/diff_git.rs: 상태 조회 index_worktree_options의 thread_limit을 1로 제한한다. 디렉터리 순회 등 라이브러리 보조 스레드까지 2개로 보장한다는 의미는 아니다.

## 검증

먼저 전용 풀과 취소 회귀 테스트를 작성한다. 100회 교체, 1500ms 지연 중 취소, 작업 실행 상한 2, 이미 실행 중인 작업의 경계 취소, 일반 작업 보존, 오류와 패닉 후 복구, Future 깨우기를 확인한다. UI 무효화 연결과 기존 결과 검증 테스트를 유지한다. cargo fmt --check, cargo test --offline, cargo check --offline 및 clippy를 실행한다. PC 전체 CPU/GPU와 다른 앱 반응성은 이 자동 테스트만으로 보장하지 않는다.

## 순서

공통 실행기 → 조회 통합 → 우선순위/내부 병렬 제한 → 전체 검증 → 수렴 점검 순서로 수행한다.
