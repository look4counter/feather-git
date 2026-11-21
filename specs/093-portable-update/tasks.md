# 실행 작업 — 단일 실행 파일 업데이트

- [x] T001 [US1] `Cargo.toml`, `Cargo.lock`, `src/updater.rs`에 XML 비교와 공식 릴리스 파일 다운로드, 교체 및 복구를 구현한다. FR-001, FR-003, FR-004
- [x] T002 [US1] `src/main.rs`, `src/update_dialog.rs`, `src/ui.rs`에서 도우미 시작 분기, 하단 버튼과 업데이트 창을 연결한다. FR-001, FR-002, FR-003, FR-004
- [x] T003 `src/updater.rs`의 버전·주소·교체·복구 테스트와 전체 대상 컴파일 및 릴리스 빌드를 실행하고 `validation.md`에 기록한다. SC-001, SC-002
- [x] T004 [US1] `src/settings.rs`와 `src/ui.rs`에서 업데이트 섹션과 요청 이벤트를 연결하고 하단 버튼을 제거한다. FR-005, SC-003
- [x] T005 전체 대상 컴파일과 업데이트 테스트를 실행하고 `quickstart.md` 및 `validation.md`에 새 경로와 검증 범위를 기록한다. FR-005, SC-003

- [x] T006 `src/update_certificates.rs`와 `src/main.rs`에서 CA 입력 검증, 목록 저장·조회·삭제 및 중복 처리를 연결한다. FR-006, FR-007
- [x] T007 `src/settings.rs`에 인증서 파일 선택과 목록 관리 UI를 연결한다. FR-006, FR-007
- [x] T008 `src/updater.rs`의 HTTPS 클라이언트에 CA 목록을 추가한다. FR-008
- [x] T009 전체 대상 컴파일과 검증을 실행하고 `validation.md`에 결과를 기록한다. SC-004

- [x] T010 `Cargo.toml`, `Cargo.lock`, `src/updater.rs`, `src/update_certificates.rs`, `src/avatar.rs`에서 업데이트에 Windows TLS를 적용하고 아바타의 기존 백엔드를 지정한다. FR-009, SC-005
- [x] T011 업데이트·인증서·아바타 테스트와 전체 대상 검사·빌드를 수행하고 `validation.md`에 결과를 기록한다. SC-005
