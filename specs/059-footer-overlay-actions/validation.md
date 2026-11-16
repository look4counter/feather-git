# 검증 결과: 하단 동작과 커밋 기록 표시 개선

## 절차와 범위

speckit-specify → speckit-clarify → speckit-plan → speckit-tasks → speckit-analyze → speckit-implement 순서로 수행했다. 작업 중 추가된 배지/현재 행/그래프 중심/직선 분기/부모 커밋 문구를 명세·계획·작업에 반영하고 일관성을 재확인했다. 최종 범위는 FR-001~016, SC-001~008, T001~T014다. 확장 훅 파일은 없고 헌법은 미작성 템플릿으로 규범 검사를 생략했다.

## 주요 변경

- src/encoding_dialog.rs, src/main.rs: 기존 공용 Dialog로 중앙 인코딩 선택창을 제공한다. 현재값·11개 항목·위아래/Enter·취소/Escape를 처리한다.
- src/ui.rs: 대화상자 생명주기·이전 포커스·모달 보호를 연결했다. 하단 원형을 제거하고 경로 옆 터미널/탐색기와 오른쪽 인코딩 버튼에 공통 배경/패딩/호버/비활성 스타일을 적용했다. 현재 저장소 행은 민트색 아이콘/이름, 4px 간격, 이름 바로 뒤 현재 라벨을 사용한다. 커밋 배지의 로컬/태그 아이콘은 흰색이다.
- src/workspace_backend.rs: 폴더 확인과 터미널 실행을 백그라운드에서 수행한다. 시스템 PowerShell 절대 경로와 고정 인수, current_dir를 사용한다. 경로는 명령 문자열로 해석하지 않는다. 탐색기는 기존 open_with_system을 사용한다.
- src/history.rs: 표시용 배지를 태그/로컬/원격/기타 순서로 안정 정렬하고 기존 캐시를 유지한다.
- src/history_graph.rs: 원의 중심을 레인 선과 같은 DPI 보정 좌표에 맞췄다. 원의 지름·테두리는 유지한다. 비수직 분기/병합은 두 점을 잇는 직선으로 표시한다.
- src/diff_viewer.rs: 상위 항목 표기를 부모 커밋으로 바꾸고 상세 표시 테스트 기대값을 갱신했다.

## 자동화 검증

- 최종 cargo test --offline: 302개 통과, 실패 0개, 기존 명시적 무시 3개. target/final-ui-tests.log.
- cargo clippy --offline --all-targets: 종료 코드 0. 새 코드 경고는 없으며 기존 중첩 조건·미사용 코드와 GPUI/proc-macro-error2 경고는 유지된다. target/final-ui-clippy.log.
- cargo rustc --offline --bin feather-git -- -o target/footer-verified.exe: 종료 코드 0. 실행 중인 기본 앱을 교체하지 않고 최신 별도 실행 파일을 생성했다. target/final-ui-build.log.
- 변경한 7개 Rust 파일의 rustfmt --check 통과.
- 인코딩 상태 테스트: 현재값 초기화, 위아래 순환, Enter 선택, 취소 시 현재값 유지.
- 폴더/터미널 테스트: 유효 폴더, 파일/없는 경로 오류, 한글/공백/셸 특수문자 보존, 작업 폴더와 고정 인수 분리, 시스템 실행 파일 절대 경로. 실제 외부 프로그램은 실행하지 않았다.
- Workspace 테스트: 저장소 없음, 새 탭, 복원/Stage/폴더 작업 중 비활성, 프로젝트 전환/닫기/경로 변경 이후 오래된 결과 차단.
- 배지 테스트: 혼합 종류와 같은 종류 순서 보존, 증분 행에도 같은 정렬, 기존 Rc 캐시 재사용, 로컬/태그 아이콘만 흰색이고 원격/기타 색은 유지.
- 그래프 재현 테스트는 수정 전 실패했다. 100%에서 원과 선 중심 0.5px 차이를 확인했다. target/graph-center-red.log.
- 수정 후 100/125/150/175/200%와 소수/음수 좌표·8개 레인·일반/현재 원의 중심 일치 테스트가 통과했다. 기존 원/흰색 내부 크기·테두리·행 연결·레인 간격·모든 배율의 직선/대각 경로 생성 검증도 통과했다.

## 테스트 계약 변경 근거

기존 원 경계 정수 픽셀 조건은 홀수 픽셀 지름의 원과 짝수 두께 선을 같은 중심에 놓는 새 요구와 충돌한다. 원 크기를 임의 변경하지 않고 중심 일치를 보장하기 위해 원 경계의 반 픽셀 위치를 허용한다. 해당 테스트는 지름/테두리 정수 픽셀, 경계 반 픽셀 정렬 및 정확한 중심 일치를 검증하도록 변경했다. 크기·내부 지름·간격·연결 검증은 유지했다. 테스트 통과만을 위한 임의 완화가 아니다.

## 명세와 구현 대응

| 요구사항 | 근거 |
|---|---|
| FR-001~003 | 공용 Dialog, 인코딩 상태/이벤트, 기존 글로벌 읽기 전용 선택 재사용 |
| FR-004~008 | status_bar/status_button, 폴더 비동기 작업과 대상/중복/모달 가드 |
| FR-009~010 | 배지 아이콘 색 정책과 표시용 안정 정렬/캐시 테스트 |
| FR-011~012 | 기존 현재 판정 유지, 민트 행 색, 4px 간격, 이름 뒤 현재 배지 |
| FR-013~015 | 중심 재현 실패→수정 성공, 원 크기 유지, 직선 분기, 기존 그래프 회귀 |
| FR-016 | 부모 커밋 제목/행/라벨 길이와 상세 테스트 갱신 |

## 제한과 미검증

실제 GUI 배치·포커스 복구·마우스/키보드 조작, 터미널/탐색기 창의 실제 실행 위치, 화면에서 그래프의 안티앨리어싱은 직접 검증하지 않았다. 앞선 작업에서 검증용 앱의 computer-use 실행이 허용되지 않았으며 이번에는 실행 승인을 우회하거나 실행 중인 앱을 교체하지 않았다. 자동화 검증과 소스 확인이 실제 화면 확인을 대신한다고 판단하지 않는다.

기능 구현의 미해결 불일치는 없으며 GUI·외부 앱 직접 검증은 위와 같이 별도로 남긴다.

## 후속 검증: 우측 버튼 배치와 행 경계 꺾임

- src/ui.rs: 터미널·탐색기·인코딩·브랜치 표시를 우측 묶음으로 배치했다. 좌측에는 경로/상태 안내만 유지한다. status_button은 badge_base(TEXT)를 직접 재사용하여 브랜치 배지와 같은 가로 8px/세로 2px 패딩·11px 글꼴·모서리·높이 기준을 사용한다. 별도 24px 고정 높이와 테두리는 제거했다. 기존 클릭/모달/중복/오래된 결과 보호는 유지했다.
- src/history_graph.rs: 원본 Segment 시작/끝과 그래프 계산을 유지하고 로컬 좌표에서 꺾임 점을 추가한다. 상단 경계 첫 4px와 하단 경계 마지막 4px는 수직이며 행 내부는 곡선 없는 직선으로 연결한다. 한 PathBuilder로 접합하고 모든 점에 기존 DPI 보정을 적용한다.
- 경계 테스트 2개 통과: 좌/우 분기, 상/하 반쪽, 100/125/150/175/200% 배율, 소수/음수 원점에서 경계 수직성과 앞뒤 행 접점/수직 레인 일치를 검증했다. 기존 그래프 중심·크기·간격·연결 테스트도 통과했다.
- 최종 cargo test --offline: 304개 통과, 실패 0개, 기존 무시 3개. target/footer-boundary-tests.log.
- cargo clippy --offline --all-targets: 종료 코드 0. 기존 미사용 코드·공급 라이브러리·비관련 중첩 조건 경고만 유지된다. target/footer-boundary-clippy.log.
- 변경 두 파일 rustfmt --check 통과.
- cargo rustc --offline --bin feather-git -- -o target/footer-boundary-verified.exe: 종료 코드 0. target/footer-boundary-build.log.
- 실제 GUI의 버튼 외곽 크기와 그래프 접합 안티앨리어싱은 직접 검증하지 않았다. 기존 실행 앱을 교체하지 않았으며 새 별도 실행 파일을 제공한다.

후속 명세 FR-004/007/015와 SC-009/010은 구현과 자동화 가능한 검증에 대응하며, 실제 GUI 확인은 위 제한대로 별도 미검증이다.

## 최신 후속 검증: 셸 구분·곡선/레인 간격·그리드 헤더

- src/workspace_backend.rs: DirectoryAction을 GitBash/PowerShell/Explorer로 구분했다. Git Bash는 PATH의 직접 런처, git.exe 근처 설치 루트와 ProgramFiles/ProgramFiles(x86)/LOCALAPPDATA 표준 설치 위치에서 찾는다. 없으면 명확한 오류를 표시한다. 런처는 인수 없이 current_dir에 저장소를 전달한다. PowerShell은 기존 시스템 실행 파일과 고정 인수/독립 콘솔을 유지한다.
- src/ui.rs: Git Bash 옆에 Windows 전용 PowerShell 버튼을 추가했다. 기존 우측 위치·공용 배지 크기·비활성 및 비동기 오류 가드는 유지했다.
- src/history_graph.rs: 레인 간격을 19→18px로 줄였다. 행 경계의 수직 4px 구간은 유지하고 내부 연결을 수직 접선의 베지어 곡선으로 바꿨다. 원 크기·중심·색·두께·캐시는 유지했다.
- src/ui.rs, src/file_history_dialog.rs: 공용 커밋 헤더와 두 목록의 헤더 인수가 같은 ROW_HEIGHT=24px를 사용한다. 공용 헤더 아래에 1px LINE 구분선을 추가했다.
- src/history_grid.rs, src/ui.rs: 5개 레인 기본 폭은 96→92px로 변한다. 두 고정 테스트 기대값만 새 간격에 맞추고 접기/확장·총 폭 보존 검증은 유지했다.

검증 결과:

- 최종 cargo test --offline: 307개 통과, 실패 0개, 기존 무시 3개. target/shell-curve-tests-final.log.
- Git Bash 탐색/명령 테스트 3개 통과: 사용자 지정 Git PATH, 직접 런처 PATH, 표준 사용자 설치 fallback, 미설치/잘못된 후보 거절, 특수문자 경로의 작업 디렉터리 분리. 실제 셸은 실행하지 않았다.
- 기존 PowerShell의 절대 실행 파일·고정 인수·작업 디렉터리 테스트와 폴더 오류/상태 보호 테스트 통과.
- 그래프 테스트 9개 통과: 여러 배율의 곡선 경로 생성, 원 중심/크기/레인 여백, 상하 경계 수직성, 앞뒤 행 경계 접점 일치.
- 최소 원 여백은 그대로 4 논리 px 이상이다. 150%에서 6 물리 픽셀을 나타내는 f32 결과 5.9999943 때문에 발생한 비교 오차를 물리 픽셀 정렬 오차(0.001px 미만)와 정수 최소 픽셀 조건으로 검증하도록 수정했다. 실제 여백 요구를 낮추지 않았다. 진단 기록은 target/lane-gap-diagnostic.log.
- cargo clippy --offline --all-targets: 종료 코드 0. target/shell-curve-clippy-final.log. 기존 경고만 유지된다.
- 변경한 5개 Rust 파일 rustfmt --check 통과.
- cargo rustc --offline --bin feather-git -- -o target/shell-curve-verified.exe: 종료 코드 0. target/shell-curve-build-final.log.

FR-004/005/015/017~020 및 SC-008~013의 최신 동작에 대응한다. UI의 Windows 조건부 버튼, 공유 헤더 높이 인수와 하단 구분선은 소스 확인했으며 실제 GUI 표시와 Git Bash/PowerShell 창 시작 위치는 직접 검증하지 않았다. 앞선 GUI 실행 승인 제한을 우회하지 않았고 실행 중인 앱도 교체하지 않았다.

## 최신 버그 검증: 바로 건너뛰기 후 페이지 커서 동기화

원인: finish_reference는 확장된 커밋 목록과 more_commits를 교체하면서 history_cursor를 이전 스냅샷 값으로 남겼다. queue_history_page가 새 목록 길이를 이전 커서에 요청하여 조회 위치 불일치 오류가 발생했다.

src/ui.rs에서 성공 결과의 history_cursor도 같은 Project에서 함께 반영하도록 수정했다. 기존 서버 오프셋 검증·늦은 결과 거절·200개 추가 정책은 유지했다. 새 조회/오류 숨기기/전체 재조회 우회는 추가하지 않았다.

- 수정 전 reference_jump_continues_pagination_from_replacement_cursor가 사용자와 같은 문구로 실패했다: 커밋을 더 읽을 수 없습니다: 커밋 조회 위치가 변경되었습니다. 기록은 target/reference-cursor-red.log.
- 수정 후 초기 1,500개 밖 중간 참조 이동 → 이전 진행 페이지의 늦은 결과 거절 → 새 커서로 200개/나머지 추가 → 종료가 통과했다. 전체 ID 순서·개수·선택·스크롤 핸들/가로 오프셋·작업 파일 바이트를 확인했다.
- 기존 가장 오래된 태그 이동 테스트에 more_commits=false와 history_cursor=None 조건을 추가했고 통과했다.
- 우측 하단 브랜치명 배지를 제거했다. branch_label은 다른 패널에서도 사용하므로 유지했고 저장소/커밋 참조 표시는 변경하지 않았다.
- cargo test --offline: 308개 통과, 실패 0개, 기존 무시 3개. target/reference-cursor-tests.log.
- cargo clippy --offline --all-targets: 종료 코드 0. 기존 경고만 유지된다. target/reference-cursor-clippy.log.
- src/ui.rs rustfmt --check 통과.
- cargo rustc --offline --bin feather-git -- -o target/reference-jump-verified.exe: 종료 코드 0. target/reference-cursor-build.log.

최신 FR-021~023, SC-014~015 및 T023~026의 구현과 자동화 가능한 검증은 완료했다. 실제 GUI의 바로 건너뛰기·스크롤 조작과 하단 표시 확인은 직접 수행하지 않았다. 새 실행 파일은 생성했으며 실행 중인 앱을 교체하지 않았다.
