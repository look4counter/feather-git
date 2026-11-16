# 검증 결과: 공통 진행 표시와 Dialog 기반

## 구현과 명세 일치

- ProgressOverlay를 src/progress.rs로 분리했다. 메시지를 지정할 수 있고 theme/GPUI에만 의존한다. 기존 RepositoryDialog의 전용 진행 UI는 제거했다.
- Idle/Browsing/Working으로 폼 상태를 구분한다. 탐색과 Git 작업 모두 중복 입력을 막고 Working에만 진행 표시를 붙인다. 탐색 취소/오류, 작업 실패·성공 처리와 포커스/Alert 순서는 유지했다.
- 공통 Dialog를 src/dialog.rs에 추가했다. 배경·카드·제목·본문·우측 footer·선택적 overlay·포커스/키 콜백 연결을 한 곳에서 구현한다.
- RepositoryDialog는 Dialog에 저장소 필드·확인/취소 및 ProgressOverlay를 조합한다. Alert도 Dialog에 메시지·확인 및 기존 닫기 키 콜백을 조합한다. Alert 생성/렌더링 API와 업무 책임은 유지한다.
- Workspace는 타이틀바를 루트의 첫 고정 행으로 두고, 그 아래 relative/flex_1/min_h_0/overflow_hidden 본문 호스트에 탭·본문·상태 표시줄과 모든 모달을 배치한다. Dialog는 이 호스트의 크기만 덮으므로 고정 높이 보정 없이 타이틀바 영역을 제외한다.
- Alert는 RepositoryDialog 다음에 렌더링되어 오류 중첩 순서가 유지된다. Git/IME/데이터 로직과 의존성은 변경하지 않았다. FR-001~008의 코드 대응을 확인했다.

## 자동화 및 빌드

- 탐색 잠금/진행 숨김, Git 작업 잠금/진행 표시와 대기 해제의 상태 계약 테스트를 추가했다.
- cargo fmt --check: 통과.
- cargo check --offline: 통과.
- cargo test --offline: 40개 통과. 기존 저장소·입력·IME·탭·그래프·Alert 및 진행 상태 테스트 포함.
- cargo clippy --offline --all-targets: 통과. 기존 탭 테스트 field_reassign_with_default 및 배포 GPUI의 부동소수점 미래 호환성 경고는 남아 있다.
- cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-progress.exe: 공통 진행 표시 단계에서 통과.
- cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-dialogs.exe: 최종 공통 Dialog/본문 호스트 구조에서 통과.
- 기존 proc-macro-error2 미래 호환성 경고와 링크/별도 출력 파일 경로 안내가 있다.

## 미검증 항목

이전 화면 도구의 실행 파일 사용 승인 제한으로 실제 탐색 창 뒤 진행 문구 미표시, 진행 중 입력 차단, Alert/저장소 팝업의 타이틀바 비침범 및 창 크기 변경·제어, 중첩 Alert의 키보드/포커스는 직접 조작 검증하지 못했다. 상태 테스트와 구조 검토가 실제 시각 검증을 대신한다고 간주하지 않는다.

## 깃털/물결 애니메이션 검증

- 기존 고정 진행 문구는 깃털 낙하 → 수면 접촉 → 3겹 물결 확산/소멸의 2.8초 반복 벡터 장면으로 대체했다. new(message) 호출 형태는 유지하며 문자열은 애니메이션 작업 키로 사용한다.
- GPUI Animation::repeat/with_animation은 표시 요소의 수명 안에서만 프레임을 요청한다. 작업 종료를 지연시키는 타이머/Task는 없다. 기존 Working 표시/탐색 숨김과 입력 차단·타이틀바 비침범 구조는 유지했다.
- 접촉 이전 물결 없음, 깃털 끝과 수면 높이 일치, 물결 확대/투명화, 반복 처음/끝 투명 상태 및 전체 프레임 좌표/불투명도 범위를 자동화로 검증했다.
- 최종 cargo fmt --check, cargo check --offline, cargo test --offline, cargo clippy --offline --all-targets 통과. 전체 테스트 42개 통과.
- cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-loading.exe 통과. Open 성공 Alert 제거도 포함한 빌드이다.
- FR-009~010 및 SC-006과 코드/수학적 프레임 조건이 일치한다. 기존 의존성/탭 테스트의 경고는 유지된다.
- 기존 도구 승인 제한으로 실제 GPU 화면의 움직임과 시각적 자연스러움은 직접 검증하지 못했다. 프레임 계산 테스트를 실제 화면 검증으로 간주하지 않는다.
