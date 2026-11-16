# 구현 검증 결과

검증일: 2026-11-13.

## 명세·명확화·계획·일관성

명세 품질 8/8 통과. 명확화 질문 0개이며 기능·데이터·사용 흐름·성능·오류·제약·용어·완료 기준을 검토했다.
구현 전 분석에서 요구사항 5개와 작업 7개를 검토했으며 누락·중복·모순·중대 문제는 0개였다. 헌법은 미작성 템플릿이므로 선언된 원칙 검사를 생략하고 AGENTS.md의 계층·호환성·최소 변경 제약을 적용했다.

## 변경과 추적성

- FR-001, SC-001: src/virtual_list.rs의 공통 함수 한 곳에서 GPUI uniform_list를 생성하고 src/ui.rs의 세 목록이 사용한다. src/main.rs에 모듈을 등록했다.
- FR-002, SC-002: 요청 범위만 전달하고 저장소·파일도 현재 데이터의 해당 슬라이스만 순회한다. 전체 목록 복제는 없다. 10만 개 중 20개 요청에서 20회만 처리하는 자동 테스트가 통과했다.
- FR-003: 기존 행 구성의 높이 36/35, 강조, 상태 색상, 경로 말줄임과 순서를 보존했다.
- FR-004: 제목·탭·빈 상태·오류는 목록 외부에 유지했다. 커밋의 prepare_rows·track_scroll·가로 스크롤 구성도 유지했다. 기존 회귀 테스트와 새 패널 데이터 원본 전환 테스트가 통과했다.
- FR-005: branches/files/history 접두사로 목록 종류를 분리하고 프로젝트·탭을 식별자에 포함했다. 공통 범위 제한, 빈 범위 생략 및 안전한 슬라이스 조회를 사용한다.

## 실행 결과

- 테스트 선행: render_visible 구현 전 rustc --test에서 해당 함수 부재로 실패하는 것을 확인했다.
- cargo test: 56개 통과, 0개 실패, 기존 수동 성능 측정용 2개 제외.
- cargo test virtual_list: 테스트 경고 수정 후 3개 재통과.
- cargo check: 통과.
- cargo fmt --check: 통과.
- cargo clippy --all-targets: 통과. 새 코드 경고는 없으며 기존 ui.rs 테스트의 field_reassign_with_default, vendor GPUI의 실수 리터럴 경고와 proc-macro-error2 향후 호환성 경고가 남아 있다.
- cargo build: 기본 출력 target/debug/feather-git.exe의 파일 잠금으로 교체 실패. 기존 프로세스는 종료하지 않았다.
- cargo rustc --bin feather-git -- -o target/feather-git-shared-list.exe: 대체 출력으로 실행 파일 컴파일·링크 성공.

## 제한 및 미검증 사항

GUI에서 직접 스크롤·시각 배치·실제 프레임 속도는 확인하지 않았다. 범위 테스트는 지연 생성 계약을 검증하며 GPU 프레임 성능 측정은 아니다. Git 메타데이터가 없어 git diff/status를 사용할 수 없었다.

## 행 선택 확장 검증 (2026-11-13)

명세·명확화·계획·작업 목록을 갱신하고 일관성 분석을 수행했다. FR-006~008과 SC-004는 T008~T011에 대응하며 모호성·누락·모순은 없다. 기존 품질 점검 8개 항목도 확장 명세에 대해 재검토하여 모두 통과했다.

- src/virtual_list.rs: SingleSelection의 단일 선택·해제·범위 검증과 공통 행 컨테이너의 클릭 이벤트·선택 배경·호버를 제공한다. 선택 인덱스와 후속 콜백은 호출자가 전달하며 가시 범위 렌더링을 유지한다.
- src/ui.rs: 세 목록에서 공통 선택 API를 사용한다. 커밋 행의 직접 on_click과 선택 배경을 제거했다. 저장소의 현재 브랜치 글자색·배지는 사용자 선택과 독립적이다. 패널 선택은 데이터 갱신에서 해제된다. Git 변경 작업은 추가하지 않았다.
- 선택 테스트를 먼저 추가하여 SingleSelection 부재로 실패한 것을 확인한 뒤 구현했다.
- cargo test --quiet: 60개 통과, 실패 0개, 기존 수동 성능 측정용 2개 제외.
- 추가 테스트: 단일 선택 이동·반복·해제, 빈 목록·인덱스 초과·데이터 감소, 패널 선택 독립성·갱신 해제, 커밋 프로젝트·시작 탭·닫힌 탭·범위 검증과 파일 갱신.
- cargo check, cargo fmt --check, cargo clippy --all-targets: 통과. 기존 vendor 실수 리터럴 및 ui.rs 테스트 초기화 관련 경고는 남아 있다. 새 코드 경고는 없다.
- cargo rustc --bin feather-git -- -o target/feather-git-shared-selection.exe: 실행 파일 컴파일·링크 성공. 이전 실행 파일 잠금과 충돌하지 않도록 별도 출력 경로를 사용했다.
- GUI에서 직접 클릭·스크롤·선택 배경의 시각 확인은 수행하지 않았다.

## 호버 우선순위·마우스 누름 수정 검증

FR-009·FR-010 및 SC-005에 맞춰 명세·명확화·계획·작업 목록을 갱신했고 일관성 분석에서 누락·모순은 없었다. 기존 명세 품질 점검 8개도 재검토하여 통과했다.
소스 변경은 src/virtual_list.rs에 한정했다. 선택되지 않은 행에만 hover 스타일을 등록한다. on_click을 제거하고 on_mouse_down(MouseButton::Left)에서 기존 콜백을 호출한다. 선택·프로젝트·범위 검증 및 데이터 흐름은 유지된다. 해당 조건과 이벤트 연결을 직접 검토했다.

- cargo test --quiet: 60개 통과, 실패 0개, 기존 수동 성능 측정용 2개 제외.
- cargo check, cargo fmt --check, cargo clippy --all-targets: 통과. 기존 vendor 및 테스트 초기화 경고만 남아 있다.
- cargo rustc --bin feather-git -- -o target/feather-git-selection-mousedown.exe: 컴파일·링크 성공.
- 단순 조건과 프레임워크 이벤트 연결을 복제하는 새 테스트는 추가하지 않았다.
- 실제 GUI에서 버튼 누름·해제 및 호버 표시를 직접 확인하지는 않았다.

## 공통 행 높이 검증

FR-011·FR-012와 SC-006을 명세·계획·작업에 반영하고 명확화 및 일관성 분석을 수행했다. 추가 질문·누락·모순은 없으며 명세 품질 8개 항목을 재검토하여 통과했다.

- src/virtual_list.rs: ROW_HEIGHT=24 정의를 소유하고 row_height: Pixels를 받아 공통 외곽 행에 높이와 축소 방지를 적용한다.
- src/ui.rs: 커밋 24·저장소 36·파일 35를 전달한다. 각 행의 고정 높이는 제거하고 h_full로 부모 높이를 따른다.
- src/history_graph.rs: 기존 ROW_HEIGHT 이름으로 공통 상수를 재노출한다. 그래프의 연결점·노드 계산은 동일한 값과 공식을 유지한다.
- cargo test --quiet: 60개 통과, 실패 0개, 기존 수동 성능 측정용 2개 제외. 그래프의 높이·연결점·DPI·노드 배치 및 목록·선택 회귀 테스트 포함.
- cargo check, cargo fmt --check, cargo clippy --all-targets: 통과. 기존 vendor 및 테스트 초기화 경고는 남아 있고 새 코드 경고는 없다.
- cargo rustc --bin feather-git -- -o target/feather-git-shared-row-height.exe: 실행 파일 컴파일·링크 성공.
- 직접 GUI에서 행 높이·그래프 연결·선택 영역의 시각 확인은 수행하지 않았다.

## 최신 검증: 개별 높이 제거

저장소·파일의 높이 유지라는 이전 가정을 최신 요청에 맞게 변경했다. FR-011·FR-012를 정리하고 FR-013·SC-007을 추가했다. 명확화 질문은 없으며 명세 품질 재검토와 일관성 분석에서 누락·모순은 없다.

- src/virtual_list.rs: 높이 인자를 제거하고 공통 ROW_HEIGHT=24를 외곽 행에 직접 적용한다.
- src/ui.rs: 저장소 36·파일 35·커밋 24의 개별 높이 전달을 모두 제거했다. 각 내용은 기존 h_full로 공통 높이를 따른다.
- cargo test --quiet: 60개 통과, 실패 0개, 기존 수동 성능 측정용 2개 제외.
- cargo check, cargo fmt --check, cargo clippy --all-targets: 통과. 기존 vendor 및 테스트 초기화 경고만 남아 있다.
- cargo rustc --bin feather-git -- -o target/feather-git-common-height.exe: 컴파일·링크 성공.
- 직접 GUI에서 24픽셀 높이의 시각 확인은 수행하지 않았다.
