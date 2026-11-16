# 구현 계획

## 기존 구조와 분석
Rust 2024·GPUI 앱이다. Workspace에 세 패널 FocusHandle이 있고 DiffViewer는 자체 포커스와 비동기 쓰기·구독을 소유한다. 하단은 ui.rs의 기존 상태 표시줄이다. 현재 목록 패널에는 키 입력 처리와 전체 Stage 토글이 없다. 뷰어의 Space는 설정된 toggle 명령을 통해 기존 체크 로직을 사용한다. partial과 변경 줄이 있는지 확인하면 구간 선택 가능 여부를 판정할 수 있다.
DiffViewer는 포커스 변경 시 자신의 notify만 수행한다. 하단의 부모 Workspace도 갱신되도록 뷰어 observe와 목록 포커스 구독을 유지해야 한다. 반복 렌더마다 새 구독을 만들지 않는다.

## 설계와 영향 범위
src/ui.rs에 목록 패널 포커스 구독·뷰어 관찰·파일 쓰기 busy 상태를 추가한다. 파일 패널에만 Space 핸들러를 연결하고 직접 포커스·수정키 없음·실행 가능한 현재 선택을 검사한다. 백그라운드 실행기에 쓰기를 맡기고 완료 후 기존 viewer_changed 갱신 경로를 재사용한다. busy는 기존 repository_busy에 포함하고 뷰어의 병행 쓰기도 검사한다. 하단은 기존 badge 스타일로 키와 설명을 표시한다.
src/diff_viewer.rs에 포커스·구간 선택 가능 여부·실제 toggle 키를 반환하는 읽기 전용 안내 메서드와 busy 접근자를 추가한다. 내부 안내만 제거하고 기존 버튼과 입력은 유지한다.
src/diff_git.rs의 전체 파일 토글은 기존 IndexLock과 gix 필터 파이프라인을 재사용한다. 인덱스가 조회 당시와 일치하는지 잠금 안에서 검사한다. Stage는 현재 작업 파일을 clean 변환해 인덱스에 반영하고 Unstage는 HEAD 내용으로 해당 인덱스 항목만 복원한다. 최초 커밋 이전 Unstage는 인덱스에서만 제거한다. 충돌은 기존 stage_resolved 검증 경로를 재사용한다. src/backend.rs 공통 I/O 게이트웨이에 함수를 등록한다.

## 제약 및 아키텍처
UI는 직접 파일 I/O를 하지 않는다. 새 의존성·영속 설정 형식·다중 선택을 추가하지 않는다. 공개 데이터와 기존 선택·정렬·부분 Stage 경계를 유지한다. Constitution은 미작성 템플릿이므로 적용 규칙은 없다.

## 검증
안내 정책·현재 선택 요청 가드·수정키 차단을 단위 테스트한다. 실제 임시 Git 저장소에서 추가·수정·삭제·최초 커밋·다른 파일 보존·작업 파일 보존·잠금과 stale 상태를 검증한다. cargo fmt --check, cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline를 수행하고 GUI 미검증과 실행 파일 잠금을 별도 보고한다.

## 콜론 제거 보완 계획

src/ui.rs의 공통 하단 안내는 format!(": {description}")으로 구분자를 추가한다. 이를 설명 문자열 직접 렌더링으로 바꾸면 두 패널에 함께 적용된다. 기존 gap_2 배치를 유지한다. 영향은 표시 한 줄이며 새 테스트는 추가하지 않고 cargo fmt --check와 cargo check --offline로 검증한다. 기존 명세와 모순이나 아키텍처 위반은 없다.
