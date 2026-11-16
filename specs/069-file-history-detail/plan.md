# 구현 계획: 파일 이력 커밋 상세

## 기술 맥락 및 기존 구조

Rust 2024, GPUI 사용. FileHistoryDialog는 파일 이력을 비동기로 읽고 첫 행을 선택하며 select 및 update_viewer를 통해 오른쪽 DiffViewer를 갱신한다. 커밋 상세는 DiffViewer의 ViewerTarget::Detail로 조회, 표시, 스크롤, 오류 및 오래된 응답 차단을 이미 구현한다.

## 설계 및 영향 범위

src/file_history_dialog.rs에 독립적인 상세 DiffViewer 엔터티를 추가한다. 선택 대상 생성은 FileHistoryState에서 두 대상의 쌍으로 구성하여 동일 선택을 보장한다. update_viewer에서 양쪽을 갱신하고 deactivate에서 양쪽을 정리한다. 왼쪽 외곽 안에 높이 50%인 목록과 상세 래퍼를 두며 최소 높이 0, 축소 방지 및 overflow 처리를 적용한다. 오른쪽 및 기존 행 메뉴와 열 조절은 보존한다. 공개 API, 백엔드 및 의존성 변경 없음.

## 헌법 점검

헌법은 미작성 템플릿이므로 유효 원칙 검사는 생략한다. AGENTS.md의 최소 변경, 기존 구현 재사용, 계층 유지 및 한글 문서를 준수한다. 설계 전후 충돌 없음.

## 검증

선택 대상에 대한 회귀 테스트: 조회 중, 최초 선택, 전환, 잘못된 인덱스, 빈 결과, 실패, 종료. cargo fmt --check, cargo test, cargo check 실행. 실제 화면의 픽셀 높이와 스크롤은 수동 확인 절차로 구분한다.
