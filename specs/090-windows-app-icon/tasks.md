# 실행 작업 — Windows 앱 아이콘

## 단계 1 — 사용자 이야기 1

- [ ] T001 [US1] `Cargo.toml`, `Cargo.lock`, `build.rs`, `design/favicon.rc`에 기존 ICO의 Windows 실행 파일 링크를 구성한다. FR-002, FR-004
- [ ] T002 [US1] `src/ui.rs`의 기본 창 제목과 기존 드래그 영역에 포함된 정적 PNG를 표시한다. FR-001, FR-003, FR-004

## 단계 2 — 검증

- [ ] T003 [US1] 개발·배포 Windows 빌드와 테스트를 실행하고 `target/release/feather-git.exe`에서 아이콘을 확인한다. SC-001
- [ ] T004 [US1] `quickstart.md`에 따라 제목, 실행 중인 작업표시줄과 창 제어를 검증하고 결과를 기록한다. SC-002, SC-003

## 단계 3 — 제목표시줄 여백 조정
- [x] T005 [US1] `src/ui.rs` 왼쪽 패딩을 8px로 조정한다. FR-005, SC-004
- [x] T006 [US1] 검증 결과를 `validation.md`에 기록한다. SC-004
