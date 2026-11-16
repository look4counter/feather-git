# 구현 계획: 공통 진행 표시

## 분석과 구조

RepositoryDialog::render의 repository-busy 오버레이가 진행 문구와 배경을 직접 그린다. 하나의 busy bool과 set_busy가 폼 입력/Enter/Esc 차단, 포커스 이동, 오버레이를 모두 제어한다. Workspace는 폴더 탐색과 Git 작업 둘 다 set_busy(true)를 호출하여 탐색에도 문구가 나타난다.

## 설계와 영향

- src/progress.rs: GPUI RenderOnce/IntoElement 기반 ProgressOverlay를 추가한다. 메시지, 기존 어두운 배경, 중앙 정렬과 입력 차단을 담는다. 부모의 상대 영역 안에서 동작하며 업무 상태나 자체 타이머를 가지지 않는다.
- src/repository_dialog.rs: busy bool을 Idle/Browsing/Working의 DialogActivity로 바꾸고 is_busy와 shows_progress로 상태 계약을 분리한다. 기존 포커스 이동/초기 필드 복귀를 set_activity에서 유지한다. Working에만 ProgressOverlay를 추가한다.
- src/ui.rs: 중복 실행·취소 가드는 is_busy를 사용한다. 탐색 시작은 Browsing, Git 시작은 Working, 종료/실패는 Idle로 전환한다. 기존 비동기 결과 검증과 Alert는 유지한다.
- src/main.rs: progress 모듈 등록.

## 헌장과 제약

헌장은 미작성 템플릿으로 검사 생략. UI → 공통 표시/폼/Alert 책임을 유지하고 진행 컴포넌트는 theme/GPUI에만 의존한다. 새 의존성, 공개 외부 API·저장 형식 또는 Git 서비스 변경은 없다.

## 검증

탐색 잠금·표시 없음과 Git 잠금·표시 있음, 대기 해제를 상태 테스트로 확인한다. 시각 컴포넌트를 복제하는 테스트는 추가하지 않는다. 포맷·컴파일·전체 테스트·clippy·별도 실행 파일 빌드를 수행한다. 실제 폴더 창/오버레이 화면 검증은 기존 도구 승인 제한에 따라 미검증으로 구분한다.

## 공통 Dialog 기반 계획

- 추가 분석: Alert와 RepositoryDialog는 모두 전체 부모를 덮는 모달 배경과 카드 스타일을 중복 구현한다. Workspace는 타이틀바와 모달을 같은 relative 루트 자식으로 렌더링하므로 배경이 타이틀바까지 덮는다.
- src/dialog.rs: 재사용 Dialog를 RenderOnce/IntoElement 요소로 구현한다. 생성자 ID/제목, 너비, 본문, 우측 footer, 선택적 카드 overlay, 포커스와 키 콜백을 지원한다. 업무 로직과 창 제목 영역 크기에는 의존하지 않는다.
- src/repository_dialog.rs: 공통 Dialog::new에 기존 필드 본문, 확인/취소 footer, Working일 때 ProgressOverlay를 전달한다. 기존 이벤트와 활동 상태/키 처리/포커스 동작은 유지한다.
- src/alert.rs: 같은 Dialog에 메시지 본문/확인 footer와 기존 키보드·닫기 콜백을 전달한다. 외부 Alert API는 유지한다.
- src/ui.rs: 타이틀바는 루트의 고정 행으로 유지하고 tabs~상태 표시줄과 모든 모달을 별도의 relative/flex_1/min_h_0/overflow_hidden 본문 호스트 안에 둔다. 고정 32px 보정 대신 레이아웃으로 팝업 범위를 제한해 창 크기와 플랫폼 차이를 흡수한다.
- src/main.rs: dialog 모듈 등록. ProgressOverlay 구조와 탐색 시 표시 없음은 이어서 유지한다.
- 영향은 모달 공통 껍데기와 본문 호스트이다. Git/IME/저장소 데이터와 새 탭 상태는 변경하지 않는다. 기존 UI → 표시 컴포넌트 → theme/GPUI 의존성 방향을 유지하며 미작성 헌장 검사는 생략한다.
- 검증은 포맷/컴파일/기존 전체 테스트/정적 검사/실행 파일 빌드와 구조 검토이다. 단순 시각 구현을 그대로 복제하는 테스트는 추가하지 않는다. 실제 타이틀바 노출과 크기 변경·중첩 모달 입력은 별도 시각 검증으로 기록한다.

## 깃털/물결 애니메이션 계획

- 분석: ProgressOverlay::render는 현재 고정 메시지만 child로 표시한다. Git 작업 수명은 DialogActivity::Working과 요소 마운트/제거로 이미 제어된다. GPUI AnimationExt는 요소가 배치될 때 프레임을 요청하고 repeat로 반복하므로 별도 타이머·Task는 필요 없다.
- src/progress.rs에 FeatherWaterScene 요소와 순수 프레임 계산을 추가한다. 180×140px 캔버스에서 깃털을 곡선 윤곽/중앙 줄기·깃살로 그리고, 낙하·흔들림 후 수면 접촉부터 3개 타원 물결을 시간차로 확장/투명화한다. 첫/마지막 경계는 투명하게 만들어 반복의 갑작스러운 전환을 줄인다.
- 기존 메시지 인수는 애니메이션 ElementId의 작업 키로 유지한다. 사용자 화면에서는 문구를 제거한다. 공통 배경·입력 차단·타이틀바 아래 배치를 변경하지 않는다.
- 영향은 공통 진행 표시와 관련 산출물에 한정한다. Git 서비스, 작업 수명·탐색 숨김, IME, Dialog 구조 및 의존성은 유지한다. 미작성 헌장은 검사 생략.
- 검증: 접촉 이전/접촉/확산/반복 경계의 프레임 계산을 테스트하고 기존 전체 테스트·포맷·컴파일·정적 검사·빌드를 수행한다. 실제 GPU 프레임의 시각적 자연스러움은 별도 미검증 여부를 기록한다.
