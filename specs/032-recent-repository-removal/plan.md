# 구현 계획

## 구조와 실제 흐름
Rust 2024·GPUI 앱이다. ui.rs의 시작 화면이 최근 목록을 렌더링하며 SessionWriter가 세션을 저장한다. backend.rs의 공통 게이트웨이가 repository.rs의 동기 I/O를 백그라운드로 전달한다. 기존 Dialog와 ProgressOverlay를 재사용한다. projects의 인덱스를 유지하며 열린 탭 인덱스만 제거한다.

## 설계
recent_repository_dialog.rs는 세 선택지와 안내만 렌더링한다. Workspace는 확인 항목·포커스와 실행 항목을 별도 보관한다. repository_busy로 열기와 삭제의 상호 배제를 통합한다. 삭제 작업은 취소 가능한 조회 슬롯을 쓰지 않고 backend::spawn에서 실행한다. 성공 시 영향받는 탭과 캐시를 정리한다. 항목만 삭제는 기존 save_session을 사용한다. 디스크 삭제 실패 시 목록과 탭을 유지하며 일부 파일 삭제 가능성을 알린다.

repository.rs는 절대 경로·루트·리파스 포인트·정확한 저장소 루트 여부를 검증하고 Git 핸들을 닫은 뒤 폴더를 제거한다. 경로 구성 요소의 정션도 거절한다. 자식 링크는 표준 remove_dir_all이 링크 자체만 제거한다. 삭제 요청에서 표시용 경로를 사용하지 않는다.

## 영향과 제약
변경: src/ui.rs, src/backend.rs, src/repository.rs, src/main.rs. 추가: src/recent_repository_dialog.rs. 기존 세션 스키마·Operation은 유지한다. Constitution은 채워지지 않은 템플릿이므로 규범 검사는 건너뛰고 AGENTS.md의 계층 분리와 최소 변경을 준수한다.

## 검증
임시 저장소로 전체 삭제, 형제 보존, 비저장소·하위·메타데이터 경로 거절을 확인한다. Workspace 단위 테스트로 목록 전용·실패·성공과 열린 탭 처리를 검증한다. cargo fmt/check/test/clippy/build --offline을 실행한다. 실제 사용자 폴더는 삭제하지 않는다. 실행 중인 기본 exe 잠금은 별도 산출물로 검증하고 GUI 미검증은 보고한다.
