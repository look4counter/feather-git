# 구현 계획: Diff 인코딩 선택·하단 표시 정리·커밋 증분 조회

## 요약

하단 라벨에서 기존 ContextMenu로 11개 보기 인코딩을 선택한다. 원본 읽기 이후 메모리의 디코딩에만 적용한다. 우측 제품명과 좌측 프로젝트명 접두사를 제거한다.

## 기술 맥락

Rust 2024, GPUI 0.2.2, gix/git2, similar 기반 Windows 데스크톱 앱이다. Cargo.lock에 이미 있는 encoding_rs 0.8.44를 직접 의존성으로 선언한다. 저장 형식 변경이나 외부 서비스는 없다. 기존 1 MiB/5,000행/비교 500ms 제한을 유지한다.

## 기존 구조와 영향 분석

- src/ui.rs: Workspace 하단의 UTF-8은 고정 문구이며 update_tab_notice가 프로젝트명과 경로를 합친다. ContextMenu는 선택·Escape·바깥 클릭·화면 경계 보정을 제공한다.
- src/diff_viewer.rs: ViewerTarget::load → diff_git::diff 호출. Files는 하위 DiffViewer를 만들며 ReadSlot과 generation으로 오래된 요청을 거절한다. 실행 중 공유 인코딩을 조회 인자로 캡처하고 대상 비교에 인코딩을 포함한다. 글로벌 변경 구독으로 현재/과거/복수 파일을 재조회한다.
- src/diff_git.rs: 원문 바이트 읽기 → NUL 검사 → UTF-8 해석 → LF/CRLF 정규화 → similar 비교. UTF-16은 원문 NUL 대신 해석 후 NUL을 검사해야 한다. 기본 diff 공개 경로는 UTF-8로 유지하고 별도 인자를 받는 내부 조회 경로를 추가한다.
- src/backend.rs: 매크로가 백엔드 공개 함수를 감싸므로 새 조회 함수도 같은 경계에 선언한다.
- partial_stage 및 conflict는 String::as_bytes를 저장하므로 비 UTF-8 보기에서 미리보기를 만들지 않는다. UTF-8 원문과 BOM은 기존대로 보존한다. 전체 Stage는 기존 원본 바이트 경로를 쓴다.
- src/main.rs와 새 src/text_encoding.rs: 실행 중 공유 선택과 디코더를 등록한다. 파일 변환·자동 감지·영구 저장은 추가하지 않는다.

## 헌법 검사

헌법은 미작성 템플릿이므로 규범 검사는 생략한다. AGENTS.md의 최소 변경, 계층 분리, 원문/공개 API 호환성, 실제 동작 분석, 검증 요구를 준수한다. 핵심 의존성 교체나 아키텍처 변경은 없다. 설계 후에도 충돌 없음.

## 구현 단계

1. 선택 모델과 엄격 디코딩을 추가한다. UTF-16 엔디언과 잘못된 코드 단위를 검증한다.
2. 읽기 전용 인코딩 비교를 백엔드 및 뷰어에 연결하고 비 UTF-8 쓰기 경로를 보호한다.
3. 팝업과 현재 선택 표시를 연결하고 하단 불필요 문구를 제거한다.
4. 실제 임시 저장소로 바이트 불변·과거/작업 Diff·오류·UTF-8 복귀를 검증한다.

## 검증

디코더의 언어별 고정 바이트, UTF-16 BOM/엔디언/불완전 문자, 빈 파일·NUL·잘못된 UTF-8 테스트를 수행한다. 임시 저장소에서 인덱스/작업 파일의 바이트를 비교하고 기존 부분 Stage 회귀를 확인한다. cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline 및 변경 파일 포맷 검사를 실행한다. GUI 검증 가능 여부는 별도로 기록한다.

## 구조와 복잡도

기능 문서는 specs/058-diff-encoding-picker 아래에 둔다. 애플리케이션은 src의 기존 UI/백엔드 구조를 유지하며 디코더 모듈 하나만 추가한다. 공통 메뉴는 중복 구현하지 않는다.

## 추가 설계: 커밋 기록 증분 조회

repository::snapshot_with_target은 현재 revwalk를 200개에서 중단한다. 이를 1,500개 초기 페이지와 살아 있는 탐색 커서로 변경한다. git2::Revwalk가 Repository를 빌리므로 저장소와 커서를 함께 소유하는 작업 스레드에 채널로 다음 페이지를 요청한다. 마지막 페이지를 오프셋으로 재전달할 수 있게 보관하여 취소된 요청 재시도에서 누락을 막는다. 참조/HEAD 시작점과 정렬은 최초 시점으로 고정한다. 모든 커서 소유자가 해제되면 채널이 닫히고 스레드가 종료된다.

GraphBuilder::append가 이미 있으므로 새 알고리즘을 만들지 않는다. PreparedHistory/HistoryState에 기존 빌더를 유지하고 다음 페이지의 그래프만 백그라운드에서 생성한다. UI는 기존 layouts/rows 뒤에 추가하고 스크롤 객체를 교체하지 않는다.

workspace_backend에 페이지 요청/결과를 추가하고 Workspace에서 보이는 범위의 끝과 현재 개수 차이 50행으로 요청한다. 단일 진행 요청, 커서 식별자와 오프셋으로 중복·오래된 결과를 거절한다. 실패는 자동 반복하지 않고 기존 그리드 아래 오류/재시도 동작을 제공한다. 같은 저장소의 탭 이동 중 완료는 해당 캐시에 반영할 수 있지만 교체된 커서/닫힌 탭에는 반영하지 않는다.

변경 파일: src/repository.rs, src/backend.rs, src/graph.rs, src/history.rs, src/workspace_backend.rs, src/ui.rs. 초기 경계(0/1,500/1,501), 연속 페이지(1,905), 중복 요청, 끝/오류, 참조 선택과 병합 그래프, 스크롤/캐시/선택 및 늦은 결과를 검증한다. 핵심 의존성·공개 저장 형식 변경 없음. 헌법/계층 검토 결과 충돌 없음.
