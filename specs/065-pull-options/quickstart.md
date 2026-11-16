# 검증 안내

임시 로컬 원격으로 네 옵션과 분기·충돌·오류를 검증한다. UI에서 Pull 메뉴의 순서와 실행 중 중복 방지, 완료 후 기록/파일 갱신을 확인한다. 실제 사용자 원격이나 계정으로 개발 중 작업을 실행하지 않는다.


## 구현 및 검증 결과 (2026-11-15)

- 상단 샘플 작업 버튼 7개를 제거하고 Pull 드롭다운에 지정된 네 옵션을 순서대로 제공한다. 메뉴는 버튼 아래에 열리고 선택 시만 실행한다.
- 기존 인증 콜백 및 백그라운드 Git 계층을 사용한다. Fetch와 세 Pull 모드의 그래프 분석, 안전한 checkout, 병합 커밋 및 rebase를 연결했다. 설정된 pull.ff/pull.rebase와 별개로 사용자가 고른 모드를 따른다.
- upstream 없음·detached/unborn HEAD·삭제된 원격 upstream·네트워크 오류·기존 충돌 상태를 처리한다. 네트워크 오류에 주소 비밀값을 포함하지 않는다.
- 인덱스 잠금 회귀에서 checkout이 작업 파일을 먼저 갱신하는 문제를 확인하고, 배타적 인덱스 잠금 및 임시 인덱스 연결로 작업 파일 변경 전 잠금을 확인하도록 수정했다. 외부 잠금과 관련 없는 로컬 변경이 보존됨을 검증했다.
- merge/rebase 충돌은 인덱스·파일·작업 상태를 보존한다. 계속/취소 전용 UI는 이번 범위에 없으며 기존 Git 도구로 완료/취소할 수 있다.
- 성공/실패 모두 Project와 PreparedHistory를 재조회하고 경로/요청 세대가 같은 결과만 적용한다. 기존 기록 커서를 새 스냅샷의 커서로 교체하며 실제 HEAD가 목록 첫 행이 아니어도 선택을 유지한다.
- Git 작업 통합 테스트 12개와 작업 가드·오래된 결과·HEAD 선택·실패 후 재조회 테스트가 통과했다. 임시 저장소는 target 하위에 만들고 줄바꿈 규칙은 fixture에 격리했다.
- 최종 cargo test --offline: 368 통과, 실패 0, 수동 성능 측정 3 무시.
- cargo clippy --offline --all-targets -- -D warnings: 종료코드 0.
- cargo fmt --all -- --check 및 rustfmt --edition 2024 --check src/synchronization.rs: 통과.
- cargo rustc --offline --bin feather-git -- -o target/pull-options-verified.exe: 종료코드 0. 별도 출력 경로 및 MSVC 라이브러리 생성 안내는 소스 경고와 구분한다.
- 실제 원격 서버·사용자 계정 인증과 GUI 조작은 미검증이다. 개발 중 사용자 저장소에 실제 Fetch/Pull을 실행하지 않았고 실행 중인 앱을 종료하지 않았다.

## 수렴 점검

FR-001은 toolbar의 단일 메뉴와 샘플 제거, FR-002/003은 SyncMode 및 격리 통합 테스트, FR-004는 기존 repository 인증/백엔드 계층 재사용, FR-005는 잠금·안전 checkout·충돌 유지·분기 거부, FR-006은 SyncRequest/SyncResult와 UI 비동기 가드 및 스냅샷 적용 테스트에 대응한다. 명세·계획·코드를 대조한 결과 미구현 요구사항이나 불일치를 발견하지 못했다. 외부 의존성 및 저장 형식 변경은 없다.


## 공통 분할 버튼 변경 검증

- SplitButton은 아이콘/하단 문구와 실행 본체, 별도 옵션 화살표를 제공한다. 실행과 선택 콜백이 분리되어 옵션 선택만으로 Git 요청이 시작되지 않는다. 공통 컴포넌트는 아이콘과 옵션·콜백을 외부에서 받으므로 다른 작업에도 재사용할 수 있다.
- Pull은 20 크기 아이콘 아래 Pull 문구를 표시한다. 툴바 높이60·가로 여백12·세로 여백3으로 줄이고 우측 브랜치와 Git 저장소 라벨을 제거했다.
- 선택된 옵션은 메뉴 체크 및 초기 커서로 표시하며 Session.pull_mode에 저장한다. 이전 JSON은 기본 merge로 읽고 알 수 없는 키도 일반 Pull로 복원한다. 앱 재시작 후 선택 복원 테스트를 통과했다.
- 일반 문맥 메뉴는 체크 옵션이 없으면 기존 표시를 유지한다. 비활성/작업 중에는 본체·화살표가 실행되지 않으며 UI에서도 현재 가드를 재확인한다.
- 최종 전체 cargo test --offline: 372 통과, 실패 0, 수동 성능 측정 3 무시.
- cargo fmt --all -- --check, rustfmt --edition 2024 --check src/synchronization.rs 및 cargo clippy --offline --all-targets -- -D warnings: 종료코드 0.
- cargo rustc --offline --bin feather-git -- -o target/pull-split-button-verified.exe: 종료코드 0. 별도 출력 경로 및 MSVC 생성 안내는 소스 경고와 구분한다.
- 실제 GUI 배치·마우스 조작과 외부 원격 인증은 미검증이다. 실제 사용자 저장소에서 Fetch/Pull을 실행하지 않았다.

## 분할 버튼 수렴 결과

FR-007은 공통 SplitButton과 두 콜백, FR-008은 메뉴 체크 및 안정 키·세션 저장·복원·기본값 테스트, FR-009는 툴바 축소와 우측 라벨 제거에 대응한다. 변경된 선택/실행 명세와 구현이 일치하며 미완료 구현이나 불일치를 발견하지 못했다. 기존 Git 실행 계층은 그대로 유지된다.

## Hover 배경 및 라벨 변경 검증

SplitButton의 기본 배경을 제거하고 활성 본체·화살표의 hover 배경만 유지했다. toolbar에서 Fetch 선택은 Fetch, 나머지 세 모드는 Pull 라벨로 전달한다. 옵션 저장·복원·실행 콜백은 변경하지 않았다. 관련 footer_action_tests 5개, fmt 검사, 엄격한 clippy 및 target/pull-hover-verified.exe 별도 빌드가 통과했다. 전체 테스트는 이번 단순 표시 변경에서 재실행하지 않았다. 실제 GUI hover·클릭 조작은 미검증이다. FR-010/011과 코드 대조 결과 미구현이나 명세 불일치 없음.

## 전역 프로그레스 변경 검증

Fetch와 세 Pull 모드의 공통 start_sync가 전역 진행 포커스를 설정하고 sync_busy 동안 기존 ProgressOverlay를 표시한다. Git 처리와 상태 재조회 완료 후 요청 세대가 같은 경우에만 포커스를 복원하며 오류는 이후 공용 알림으로 표시한다. 하단 notice의 원격 작업 실행 중·완료·결과 안내 쓰기를 제거했다.

관련 작업 상태 회귀 테스트 5개와 프로그레스 테스트 4개가 통과했다. 성공·실패 후 표시 해제, 오래된 결과의 표시 유지, 기존 하단 문구 보존을 확인했다. fmt 검사·엄격한 clippy 및 target/pull-global-progress-verified.exe 별도 빌드가 성공했다. 전체 테스트는 이번 UI 연결 변경에서 재실행하지 않았다. 실제 GUI 포커스/오버레이 및 실제 원격 작업은 미검증이다. FR-012/013과 구현 대조에서 미구현이나 불일치를 발견하지 못했다.

## 작은 진행 카드 및 타이틀바 제외 검증

ProgressOverlay의 내용은 너비280 PANEL 카드에 애니메이션과 문구로 배치했다. 루트는 반투명 검정 막과 기존 입력 차단을 유지하며 top_inset/left/right/bottom으로 작업 영역만 덮는다. Windows 타이틀바와 오버레이가 같은 CUSTOM_TITLEBAR_HEIGHT(32)를 사용하므로 타이틀바를 덮지 않는다. 다른 플랫폼은 클라이언트 밖의 네이티브 타이틀바를 사용하여 inset0이다.

프로그레스 테스트4개와 작업 상태 회귀5개, fmt 검사·엄격한 clippy 및 target/progress-card-verified.exe 별도 빌드가 통과했다. 전체 테스트는 이번 표시 변경에서 재실행하지 않았다. 실제 GUI 크기·반투명도·타이틀바 조작은 미검증이며 코드 배치와 차단 영역을 대조했다. FR-014/015와 구현은 일치하고 남은 구현 작업이나 불일치를 발견하지 못했다.

## Pull rebase 옵션 제거 검증

최종 UI는 Fetch·일반 Pull·빠른 전진 전용 Pull 세 가지 OPTIONS만 사용한다. 이전 rebase 저장값은 일반 Pull로 정규화하며 선택·실행도 UI 목록으로 보호한다. 내부 리베이스는 기록 양방향 기능에서 공유한다.
footer_action_tests 6개와 전체 회귀 468개가 통과했다. 최종 컴파일·엄격한 Clippy·별도 history-rebase-verified.exe 빌드가 성공했다. 추가 리베이스 사전 검사 보완 후 관련 테스트 9개도 통과했다. 실제 GUI 메뉴 클릭은 수동 검증하지 않았다.
명세·계획·작업·코드에서 세 옵션·복원·선택 거절·기존 실행 보호를 대조했으며 미해결 불일치 없이 수렴했다. 새 기록 기능의 상세 검증은 ../079-history-rebase/quickstart.md에 기록했다.
