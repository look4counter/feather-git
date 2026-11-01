# 작업 목록: 외부 변경 자동 감지

## 1단계: 준비
- [x] T001 specs/005-watch-external-changes/spec.md와 설계 및 품질 점검을 검증한다.

## 2단계: 기반
- [x] T002 Cargo.toml의 Windows IO·Threading 기능과 src/lib.rs 감시 모듈을 연결한다.

## 3단계: 사용자 이야기 1
독립 검증: 가상 시간 및 실제 파일·인덱스·참조 이벤트의 범위를 확인한다.
- [x] T003 [US1] src/repository_watch.rs에 500ms·2초 병합 스케줄러와 결정적 테스트를 구현한다. FR-001·FR-003에 대응한다.
- [x] T004 [US1] src/repository_watch.rs에 Windows 감시·변경 분류·취소 수명을 구현한다. FR-002·FR-006·FR-008에 대응한다.
- [x] T005 [US1] src/ui.rs에 활성 저장소 자동 조회와 선택·스크롤 보존 및 세대 검증을 연결한다. FR-002·FR-003·FR-007에 대응한다.

## 4단계: 사용자 이야기 2
독립 검증: 무시 파일·추적 파일·비활성 탭·진행 중 이벤트와 종료를 확인한다.
- [x] T006 [US2] src/repository_watch.rs에 Git 무시 필터와 실제 파일·연결된 worktree 감시 테스트를 구현한다. FR-004·FR-006에 대응한다.
- [x] T007 [US2] src/ui.rs에 비활성 탭 지연·모달 및 작업 중 대기·탭 종료 해제를 연결한다. FR-003·FR-005·FR-006에 대응한다.

## 5단계: 마무리
- [x] T008 README.md 및 specs/005-watch-external-changes/quickstart.md에 사용법과 실제 검증 결과를 기록하고 검사·수렴을 완료한다.

## 의존성과 실행 전략
T001 → T002 → T003 → T004 → T005 → T006 → T007 → T008 순서다. 사용자 이야기 1 이후 부하 제한과 수명까지 완료한 전체가 제공 범위다. 공유 모듈과 통합 경로를 순서대로 수정하므로 병렬 작업은 없다.

