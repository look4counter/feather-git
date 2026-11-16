# 구현 및 검증 결과

## 단계별 결과

speckit-specify로 세 사용자 이야기와 FR-001~006을 정의했다. speckit-clarify에서 우클릭은 현재 선택을 유지하고 실행 불가 메뉴는 비활성으로 표시하며 기존 Space 직접 실행을 유지한다고 명확히 했다. speckit-plan은 Stage/체크/하단 안내/GPUI anchored·deferred의 기존 호출 흐름과 변경 영향을 분석했다. speckit-tasks로 공통 메뉴→패널 연결→안내 제거→회귀 검증을 정의했다. speckit-analyze에서 요구사항·계획·작업 사이의 누락·모순·중복이 없었다. 확장 훅 설정은 없으며 Constitution은 미작성 템플릿이다.

## 구현 결과

- src/context_menu.rs: 재사용 가능한 항목·단축키 배지·비활성 상태·실행 콜백 메뉴를 추가했다. 하나의 메뉴를 최상단 deferred/anchored로 렌더하고 창 안으로 위치를 보정한다. 외부 클릭·Escape·스크롤·창 비활성화 시 닫고 이전 포커스를 복원한다. 방향키는 활성 항목을 순환하고 Enter/Space로 실행한다.
- src/main.rs: 공통 메뉴 모듈을 등록했다.
- src/ui.rs: 작업 트리 우클릭에 선택한 파일 스테이지 토글 / Space 메뉴를 연결했다. 기존 Space와 메뉴는 같은 비동기 배치 실행 메서드를 사용한다. 프로젝트·패널 세대와 선택/쓰기 상태를 재검증한다. 선택을 바꾸지 않는다. 빈 선택 헤더는 선택된 파일 0개다. 하단 단축키 안내 렌더를 제거했다. 모달/프로젝트 화면 이탈 시 메뉴를 닫는다.
- src/diff_viewer.rs: 선택한 줄 체크 / Space 메뉴를 연결했다. 변경 줄이 있는 원래 선택 범위와 조회 세대를 보존하고 실행 시 다시 검사한다. 자식 패널 우클릭은 전파를 막아 해당 파일에만 적용한다. 기존 toggle을 재사용한다. 메뉴가 열린 동안 뒤쪽 본문의 전역 마우스 캡처가 선택을 바꾸지 않도록 차단한다. 하단 안내 전용 코드/테스트를 메뉴 실행 조건 검증으로 교체했다. 구간 버튼은 구간 체크다.

## 검증

- cargo test --offline: 252개 통과, 실패 0개, 기존 3개 제외.
- 메뉴 탐색 테스트: 비활성 항목 건너뛰기·순환·모두 비활성·빈 항목을 검증한다.
- 파일 메뉴 테스트: 빈 선택 헤더·현재 선택 실행 가능·다른 프로젝트/오래된 세대 차단·선택 보존·쓰기 중/커밋 기록 모드 차단을 검증한다.
- 체크 메뉴 테스트: 읽기 전용/변경 없는 범위/빈 범위/범위 밖 차단, 추가·삭제 줄 허용, 원래 범위 유지, 다른 조회 세대/비활성 상태 차단을 검증한다.
- 기존 다중 파일 Stage·중복 경로 Stage 우선·부분 실패·본문 범위 선택·구간 체크 회귀 테스트가 통과했다.
- cargo fmt --check: 통과.
- cargo clippy --offline --all-targets: 통과. 기존 경고만 존재한다.
- cargo build --offline: 컴파일·링크 후 실행 중 target/debug/feather-git.exe 제거가 액세스 거부(os error 5)로 실패했다. 실행 파일 교체와 현재 실행 중 앱 반영은 미완료다.

## 수렴 점검

speckit-converge로 FR-001~006, SC-001~002와 세 사용자 이야기 및 계획의 메뉴/공유 처리/상태 보호/안내 제거를 코드와 대조했다. T001~T006 구현 및 자동 검증이 문서와 일치한다. 미해결 구현 차이는 없고 추가 작업은 없다. tasks.md는 수렴 단계에서 추가 수정하지 않았다.

## 남은 검증 한계

실제 GUI 입력·메뉴 외형·가장자리 위치·닫기/포커스 복원·다중 파일에서 메뉴 클릭을 직접 검증하지 못했다. 해당 경로는 API와 코드로 검토했으며 자동 GUI 검증 통과로 주장하지 않는다. 현재 실행 파일도 교체되지 않았다.

## 후속 버튼명 변경 검증

speckit-specify로 FR-007과 수용 기준을 추가하고 speckit-clarify에서 문구만 변경하며 단축키/동작을 유지함을 명확히 했다. speckit-plan에서 stage_button_label → render 호출과 기존 테스트를 분석했고 speckit-tasks로 T007~T009를 정의했다. speckit-analyze에서 문서/작업의 불일치는 없었다. speckit-implement로 src/diff_viewer.rs의 기본/단축키 포함 문구와 기존 회귀 기대값을 체크한 줄 스테이지로 변경했다.

cargo fmt --check와 cargo clippy --offline --all-targets가 통과했다. 기존 버튼명·Enter 제외·사용자 지정 단축키 테스트 1개가 통과했다. 단순 문구 수정으로 전체 테스트와 배포 실행 파일 빌드는 반복하지 않았다. 테스트 실행 파일은 컴파일·실행됐다. 실제 GUI 확인과 실행 중 앱 교체는 수행하지 않았다.

speckit-converge에서 FR-007과 버튼명/단축키 표시 수용 기준 및 T007~T009를 코드와 대조했다. 문구가 일치하고 Stage 처리 변경은 없으며 미해결 구현 차이와 추가 작업은 없다. 수렴 단계에서 tasks.md는 수정하지 않았다.

## 후속 체크 메뉴 표시 조건 검증

speckit-specify로 FR-004와 수용 기준을 갱신하고 speckit-clarify에서 현재 줄/범위에 체크 가능한 변경 줄이 있고 실행 가능할 때만 항목을 표시하며 빈 메뉴도 열지 않는다고 명확히 했다. 작업 트리 비활성 메뉴 정책은 유지한다. speckit-plan으로 check_menu_rows → open_check_menu → 공통 메뉴 호출을 분석하고 speckit-tasks로 T010~T012를 정의했다. speckit-analyze에서 문서/작업의 충돌·누락은 없었다. speckit-implement로 src/diff_viewer.rs에서 기존 체크 가능 조건이 없으면 메뉴 열기 전에 반환하고 전파를 막도록 변경했다. 가능할 때는 활성 메뉴를 열며 실행 시 조건/세대 재검증을 유지한다.

cargo fmt --check, cargo clippy --offline --all-targets 및 기존 check_menu_requires_changed_rows_in_the_current_range 테스트 1개가 통과했다. 이 테스트는 변경 줄 존재·범위 밖·문맥 줄·빈 범위·읽기 전용·세대 불일치·비활성 상태 조건을 확인한다. 메뉴 호출 분기와 부모 전파 차단은 코드로 확인했다. 전체 테스트와 배포 실행 파일 빌드는 반복하지 않았으며 테스트 실행 파일 컴파일·실행은 통과했다. 실제 GUI 입력과 실행 중 앱 교체는 수행하지 않았다.

speckit-converge에서 갱신 FR-004/006의 체크 가능 조건·빈 메뉴 생략·현재 선택 보존·다중 파일 전파·실행 재검증과 T010~T012를 코드에 대조했다. 미해결 구현 차이와 추가 작업은 없다. tasks.md는 수렴 단계에서 수정하지 않았다.

## 후속 파일 메뉴 이름 변경 검증

speckit-specify로 FR-003 표시 이름을 선택한 파일 스테이지로 갱신하고 speckit-clarify에서 이름만 변경하며 기존 Stage 토글/Space 정책을 유지함을 명확히 했다. speckit-plan으로 open_files_menu → MenuItem 제목 전달을 분석하고 speckit-tasks로 T013~T015를 정의했다. speckit-analyze에서 명세·계획·작업의 누락/모순은 없었다. speckit-implement로 src/ui.rs의 메뉴 제목 하나를 변경했다.

cargo fmt --check와 cargo clippy --offline --all-targets가 통과했다. 단순 문구 변경이므로 테스트를 추가하거나 전체 회귀/배포 실행 파일 빌드를 반복하지 않았다. 실제 화면 확인과 실행 중 앱 교체도 수행하지 않았다.

speckit-converge에서 FR-003의 새 이름과 기존 배지·실행 콜백·비활성 조건 보존을 코드에 대조했다. 미해결 구현 차이와 추가 작업은 없으며 tasks.md는 수렴 단계에서 수정하지 않았다.
