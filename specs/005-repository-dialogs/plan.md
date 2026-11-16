# 구현 계획: 저장소 설정 팝업과 실제 프로젝트 열기

## 기존 구조와 영향 분석

현재 main → ui → sample/graph 구조이며 Workspace는 정적 PROJECTS 인덱스와 COMMITS/FILES를 직접 사용한다. Alert는 독립 모달이며 theme를 공유한다. Create는 테스트 Alert, Open/Clone은 미연결 안내만 제공한다. 입력 요소와 저장소 서비스가 없어 실제 프로젝트를 열려면 데이터 접근을 별도 계층으로 추가하고 정적 렌더링을 프로젝트 데이터로 바꿔야 한다.
GPUI 0.2.2 로컬 공식 input 예제는 EntityInputHandler의 UTF-16/IME 및 TextElement 렌더링을 제공한다. old의 start_repository는 prompt_for_paths와 백그라운드 작업을 사용한다. 기존 git2, gix, url, tempfile로 처리 가능해 새 의존성이 필요 없다.

## 설계와 파일

- src/text_input.rs: 공식 GPUI 예제의 단일행 편집/IME/선택/클립보드 요소를 재사용하고 앱 전용 예제 코드는 제외한다. 값 변경 이벤트와 외부 set_text, 한 줄 정규화, 포커스를 제공한다. 기존 의존성 범위에서 문자 경계를 안전하게 처리한다.
- src/repository.rs: Operation/Create/Open/Clone, Request, Commit/ChangedFile/Project 데이터와 순수 경로→이름 함수를 정의한다. git2로 소스 접근/대상 검사·초기화/복제 및 스냅샷 로딩을 수행한다. NO_SEARCH로 부모 저장소 오인과 no_reinit로 재초기화를 막는다. 유효성 및 실패 메시지에 소스 인증정보를 포함하지 않는다.
- src/repository_dialog.rs: 모드별 필드 Entity, 변경 구독, busy 상태를 가진 공통 폼을 제공한다. 경로 변경 이벤트는 이름을 갱신하고 사용자 이름 수정은 독립적이다. 경로 탐색/확인/취소는 Workspace 콜백으로 위임한다. Tab/Shift+Tab, Enter/Esc 및 모달 차단을 처리한다.
- src/ui.rs: 동적 Project 목록으로 기존 데모를 유지한다. Create/Open/Clone에서 공통 폼을 열고 폴더 탐색, 백그라운드 Git 작업, 오류 Alert, 성공 시 새 탭 종료/중복 프로젝트 선택을 연결한다. Alert는 폼보다 뒤에 렌더링하여 오류를 위에 표시한다. 선택된 프로젝트 데이터로 패널·그래프·상세와 빈 상태를 렌더링한다.
- src/main.rs: 새 모듈과 입력 키 바인딩 등록. Cargo.toml은 변경하지 않는다.

## 흐름과 경계

입력 → 확인 → 단일 busy 작업 → 소스 검증(Clone) → 대상 검증 → 작업 → 실제 스냅샷 → 프로젝트 선택. 오류는 busy 해제 후 폼 위 Alert로 전달한다. 탐색 취소는 값 보존, 지연된 결과는 같은 폼이 여전히 열려 있을 때만 반영한다. 작업 중 폼을 닫지 못하게 한다. 저장소 생성/복제 실패 시 재귀 삭제·정리 작업은 하지 않는다.
초기 커밋은 최근 200개와 추가 이력 표시를 제공한다. 아직 커밋이 없으면 빈 상세를 표시한다. 기존 샘플 데이터는 같은 표시 모델로 변환하며 실제 탭에 샘플이 나타나지 않게 한다.

## 헌장 및 제약

헌장은 미작성 템플릿이므로 검사 생략. UI → 저장소 서비스/데이터 모델, UI → 입력/폼/Alert, 서비스 → git2 방향을 지키며 UI에서 파일 작업을 직접 수행하지 않는다. 기존 의존성/공개 외부 API/영속 데이터 형식은 변경하지 않는다. 실제 Git 연동은 이번 명시적 사용자 요청에 따른 확장이다.

## 검증 전략

서비스 재현 테스트부터 작성한다. tempfile에 초기화/열기/복제, 비어 있지 않은 대상과 기존 파일 보존, 부모 저장소와 가짜 .git, 소스 오류 우선, 유니코드 경로·이름과 초기 커밋/변경 상태를 검증한다. 입력 UTF-16·한 줄·문자 경계 및 폼 모드/값 의미를 검증하고 프로젝트 전환·중복·기존 탭 테스트를 유지한다. cargo fmt --check, cargo check --offline, cargo test --offline, 별도 실행 파일 빌드. 실제 네이티브 폴더 선택·IME·네트워크/인증은 미검증 범위를 명시한다.

## Open 성공 안내 제거 계획

- 분석: Workspace::submit_repository의 Ok(project)는 설정 팝업 종료/프로젝트 활성화 뒤 Open일 때만 show_alert를 호출한다. Err(error)는 작업 잠금을 해제하고 별도 오류 Alert를 표시한다.
- src/ui.rs의 성공 분기에서 Open 전용 show_alert 조건문만 제거한다. operation 변수는 오류 타이틀에 필요하므로 유지한다. 서비스·폼·포커스·전환 및 공통 Dialog 구조는 변경하지 않는다.
- 영향 범위는 성공 UI 분기와 관련 명세/계약/검증 문서이다. 기존 의존성 방향에 변화가 없으며 미작성 헌장 검사는 생략한다.
- 검증: 소스 분기 검토, cargo fmt --check, cargo check/test/clippy --offline 및 별도 실행 파일 빌드. 단순 안내 제거를 복제하는 테스트는 추가하지 않고 기존 프로젝트 전환·저장소 검증 테스트를 회귀 실행한다.
