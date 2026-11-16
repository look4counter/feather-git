# 구현 계획: 전역 저장소 진행 표시

## 기존 구현과 원인 분석

Rust 2024·GPUI·전용 백엔드 풀을 유지한다. src/ui.rs의 submit_repository는 RepositoryDialog를 Working으로 바꾸고 완료 시에도 같은 대화상자 엔티티가 존재하는지 검사한다. RepositoryDialog가 내부 ProgressOverlay를 렌더링하므로 대화상자를 즉시 닫으면 기존 완료 경로가 결과를 폐기한다. 최근 저장소는 opening_recent 별도 bool과 인라인 문구 및 finish_recent_repository를 사용한다. ProgressOverlay는 absolute/size_full이지만 현재 대화상자 카드 안에 배치된다.

최근 목록은 세로 flex의 목록·카드·텍스트에 명시적 폭이 없고 truncate를 사용한다. 저장된 이름은 비어 있지 않으며 사용자는 두 텍스트 모두 ‘...’만 보인다고 확인했다. 화면 접근은 자동 승인 검토에서 거절되어 실제 폭은 측정하지 못했다. 목록·카드·텍스트에 w_full/min_w_0을 명시하고 목록은 flex_1/min_h_0, 텍스트는 줄 높이/최소 높이와 flex_shrink_0으로 읽을 수 있는 공간을 확보한다. 원 데이터와 경로 정규화는 변경하지 않는다.

## 설계 및 영향 범위

- src/ui.rs: opening_recent을 repository_operation: Option<Operation>으로 통합한다. 순수 begin_repository_operation이 중복/복원/오류 경고를 검사하고 작업 상태·드래그를 제어한다. 공통 start_repository_operation은 포커스를 보관하고 전용 포커스를 만든 뒤 기존 backend::spawn으로 실행한다. 공통 finish_repository_operation은 진행 상태 해제 후 성공 프로젝트를 활성화하거나 오류를 반환한다.
- submit_repository는 유효한 유휴 대화상자인지 검사하고 입력 창/구독을 해제한 뒤 공통 실행 경로를 호출한다. 완료는 닫힌 대화상자를 참조하지 않는다. 최근 저장소도 같은 실행 경로를 사용하고 기존 인라인 문구를 제거한다. 오류 제목은 기존 진입 경로를 유지한다.
- root window_frame을 relative로 하고 content 뒤 마지막 자식에 ProgressOverlay를 배치하여 제목 표시줄을 포함한 앱 창을 덮는다. progress_focus와 progress_previous_focus로 작업 중 키 입력을 오버레이에 한정하고 완료 후 기존 포커스를 복원한 다음 오류 경고를 표시한다.
- 탐색/선택/열기 진입점에 진행 상태 가드를 적용하고 진행 시작 시 드래그/열 크기 조절을 종료한다. 렌더 시 목록 스크롤 핸들과 뷰어 입력을 modal 상태에 맞춰 중지한다. 기존 백엔드 결과 반영과 파일 변경 완료는 정상 처리한다.
- src/progress.rs: ProgressOverlay에 선택적 포커스 핸들을 받아 루트로 렌더링한다. 기존 애니메이션과 외형은 유지하며 마우스 이동·스크롤·키 입력을 차단한다. 별도 창/OS 전역 오버레이는 만들지 않는다.
- src/repository_dialog.rs: 내부 ProgressOverlay와 더 이상 사용하지 않는 Working 상태/표시 조건을 제거한다. 폴더 선택의 Browsing과 유휴 입력 검증은 유지한다.
- src/ui.rs의 최근 목록은 기존 이름/경로와 on_click 연결을 유지하면서 폭/높이 제약을 명시한다. 새 데이터 변환·저장·최근 목록 정책은 추가하지 않는다.

## 헌법과 제약

헌법은 미작성 템플릿이라 규범 검사를 생략한다. AGENTS.md의 UI/비즈니스/데이터 책임·기존 API/저장 호환성·최소 변경을 준수한다. 실제 저장소 작업·파일 로딩·시작 화면·세션 작성기는 변경하지 않는다. 새 외부 의존성은 없다.

## 검증

전역 작업의 Open/Create/Clone 시작·중복 방지·복원/오류 경고 차단·성공/실패 해제·탐색/선택 거절·최근 재열기 보존을 테스트한다. 기존 최근 열기 테스트는 통합 상태/완료 메서드로 갱신하고 대화상자 테스트는 제거된 내부 Working 대신 Browsing의 입력 차단을 검증한다. 최근 이름/경로의 데이터·세션 회귀는 유지한다. 스타일만 복제하는 테스트는 추가하지 않는다. 전체 test/fmt/check/clippy/build를 실행한다. 화면 접근 거절로 최근 목록 및 전역 배치의 실제 GUI 검증은 미수행으로 보고하며 다른 UI 접근 방식으로 우회하지 않는다.
