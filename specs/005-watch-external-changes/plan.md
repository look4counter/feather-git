# 구현 계획: 외부 변경 자동 감지

날짜: 2026-11-01
명세: [기능 명세](spec.md)

## 요약과 기술 문맥
Rust 2024, GPUI 0.2.2, gix 0.87.1, git2 0.21 및 기존 windows 0.61을 사용한다. Windows ReadDirectoryChangesW의 비동기 알림을 전용 스레드에서 수신한다. 주기적으로 전체 디렉터리를 스캔하지 않는다. 기존 windows 의존성에 IO·Threading 기능만 추가한다.

## 규칙 확인
헌법은 미작성 템플릿이므로 확정 원칙 검사는 생략한다. 한글 문서 및 전체 speckit 절차를 준수하며 설계 전후 위반 없음.

## 조사와 설계
- src/repository_watch.rs: 변경 범위, 시간 기반 병합 상태, Windows 감시 수명과 무시 필터를 제공한다. 작업 디렉터리와 별도 gitdir·commondir을 감시하며 중복 루트는 제거한다. 읽기 이벤트와 객체·로그·lock 임시 파일은 무시한다. HEAD·refs·packed-refs·config·통합 상태는 전체 범위, index·무시 규칙은 Stage 범위다.
- src/ui.rs: 저장소 탭별 감시 핸들과 스케줄러를 소유한다. GPUI 100ms 타이머는 큐와 상태만 확인한다. 비활성 탭은 병합만 한다. 쓰기 작업·모달·기존 조회가 진행 중이면 시작하지 않는다.
- src/ui.rs: 자동 갱신은 별도 백그라운드 조회로 Stage 전용 또는 전체 snapshot을 얻고 현재 선택 키를 기준으로 적용한다. UI 세대와 선택을 재검증하고 오래된 결과는 폐기·재예약한다. 히스토리 갱신 중에는 기존 데이터 표시를 유지한다.
- src/lib.rs: 감시 모듈을 공개한다.
- Cargo.toml: 기존 windows 의존성 기능을 확장한다.

## 검증
가상 시계로 500ms·2초·동시 실행 방지·후속 병합·비활성 지연을 검증한다. 실제 Windows 임시 저장소로 저장·삭제·index/refs·무시 파일·추적된 무시 파일·연결된 worktree·종료를 검증한다. cargo test repository_watch, cargo check --all-targets, cargo fmt --check 및 clippy를 수행한다. 사용자 안내는 README의 새 절로 추가한다.

감시 시작과 인덱스 준비도 백그라운드에서 수행한다. 알림을 등록한 직후 한 번 전체 동기화를 예약해 초기 화면 조회와 감시 시작 사이의 변경을 보완한다. 전체 갱신에서는 기존에 읽은 이력 페이지 수를 가능한 한 유지하여 스크롤 위치를 보존한다.
