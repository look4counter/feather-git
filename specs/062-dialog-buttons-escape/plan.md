# 구현 계획

## 기술과 기존 구조 분석
Rust/GPUI의 공용 Dialog를 유지하고 src/dialog.rs에 역할별 공통 버튼 생성 함수를 추가한다. Open Repository의 확인은 32 높이, px_4, SELECTED 배경/ACCENT 테두리이며 보조 버튼은 LINE 테두리, 호버는 RAISED이다. 글자는 TEXT와 13 크기로 명시하고 비활성 상태를 공통화한다. 기존 각 창의 이벤트 리스너는 그대로 연결한다. 최근 저장소 삭제의 길게 누르기와 진행 레이어는 보존한다.

현재 인코딩·알림·파일 이력·삭제·폐기·저장소 창에는 ESC 처리가 있고 환경설정 및 하위 창만 없다. Workspace 설정 오버레이에서 조합 중 여부를 확인한 뒤 기존 close_settings를 호출한다. 이 함수는 하위 창부터 닫고 저장 중 보호와 포커스를 복원하므로 새 닫기 로직을 중복하지 않는다. 뷰어 단축키 폼은 기존 ESC 경로를 유지하고 버튼만 통일한다.

## 범위와 책임
변경 대상은 dialog.rs, repository_dialog.rs, encoding_dialog.rs, settings.rs, auth_settings.rs, alert.rs, file_history_dialog.rs, recent_repository_dialog.rs, ui.rs, diff_viewer.rs이다. UI 스타일 및 취소 처리만 변경하며 Git·인증 저장 계층과 설정 형식은 유지한다. 공통 버튼은 스타일만 담당하고 실제 작업 가드는 호출자가 유지한다.

## 불변 조건과 연구 결과
헌법은 미작성 템플릿이며 AGENTS.md의 최소 변경과 계층 분리를 따른다. 새 의존성은 없다. 문맥 메뉴·상태바·체크박스와 진행 표시를 일반 버튼으로 바꾸지 않는다. 기존 단축키를 추가 복원하지 않는다. 기술적 미해결 사항 없음.

## 검증
공통 역할별 팔레트/ESC 판단의 경계 조건과 기존 취소·삭제 보호 테스트를 확인한다. 스타일 변경 자체를 복제하는 테스트는 추가하지 않는다. 전체 cargo test, fmt, 경고를 오류로 처리하는 clippy 및 일반 빌드를 실행한다. 실제 GUI 검증 제한을 보고한다.

## 폐기 버튼 순서 변경 계획

src/ui.rs의 discard_dialog는 파일 전체와 체크한 줄 폐기가 공유한다. footer의 자식 순서만 확인·취소로 바꾸고 폐기 라벨을 확인으로 변경한다. confirm=true/false와 ESC 콜백은 유지한다. 별도 구현과 의존성은 필요 없다. 기존 폐기 관련 테스트, 포맷, 엄격한 정적 분석과 빌드로 검증한다.
