# 검증 안내

로컬·원격·태그 및 커밋 기록의 행을 우클릭하여 선택과 체크아웃 메뉴를 확인한다. 실행 후 HEAD·기록·파일 갱신, 단일클릭과 참조 더블클릭 유지, 충돌 안내를 확인한다. 자동 검증은 fmt, test, check, 엄격한 clippy를 사용한다.

## 자동 검증 결과

- 체크아웃 관련 테스트: 11개 통과.
- 전체 테스트: 407개 통과, 실패 0개, 수동 성능 테스트 3개 제외.
- cargo check, cargo fmt --check, cargo clippy -- -D warnings 통과.
- 새 UI 테스트의 최초 실행은 빈 테스트 저장소에 커밋을 넣지 않은 준비 오류로 실패했다. 명시적인 커밋 데이터를 준비한 후 재실행과 전체 테스트가 통과했다.
- 실제 GUI 우클릭·메뉴 선택 조작은 미검증이다. 실제 사용자 저장소에 체크아웃은 실행하지 않았다.

## 수렴 점검

요구사항 5개, 성공 기준 3개와 두 사용자 이야기 및 계획의 기존 경로 재사용·정확한 대상·선택 재검사·안전한 체크아웃을 코드와 테스트에 대조했다. 구현 누락이나 모순 및 추가 작업 없음. 헌법은 미작성 템플릿으로 유효 규범 검사는 생략했다. 화면 조작 검증의 제한은 위와 같다.

## 브랜치 우선 체크아웃 검증 결과

- 구현 전 후보·확인 소비·로컬 참조 검증 테스트의 미구현 API 실패를 확인했다.
- 전체 cargo test --offline: 474개 통과, 실패 0개, 기존 제외 3개.
- 최종 공통 복귀 코어 연결 후 cargo test --offline history_checkout_: 3개 통과. 로컬 우선·단일/다중 후보·원격만 존재·원격 HEAD 별칭·태그 제외·확인/취소·중복 소비·후보 변경·소유 경로·작업 상태·잘못된 선택 및 실제 attached HEAD·실행 직전 참조 이동 거절 포함.
- 역방향 복귀를 포함한 최종 cargo test --offline rebase: 15개 통과.
- 최종 cargo check --offline 및 cargo clippy --offline --all-targets -- -D warnings: 통과.
- cargo rustc --offline --bin feather-git -- -o target/branch-aware-checkout-verified.exe: 별도 실행 파일 빌드 성공. 출력 경로와 MSVC 라이브러리 생성 안내 경고만 발생했다.
- 실제 팝업의 시각적 배치·클릭·Tab/Enter/Escape는 수동 검증하지 않았다. 공용 Dialog·가상 목록·기존 입력 보호를 재사용한다. 기본 실행 중 앱을 교체하지 않았고 실제 사용자 저장소는 수정하지 않았다.

### 일관성과 수렴

추가 요구사항 FR-006~007·수용 기준 5개·SC-004를 T007~T010에 대응시켰고 품질 점검 5개를 통과했다. 구현 후 원래 FR-001~007과 단일·다중·로컬 우선·원격 기존 흐름·브랜치 없음·오래된 UI/실제 참조 보호를 대조했다. 헌장은 미작성 템플릿으로 별도 원칙 검사 제외. 누락·부분 구현·모순·요청 외 추가 0건으로 수렴 완료다.

## 목록 표시 오류 보완 검증 (2026-11-16)
- ui.rs 목록 래퍼에 세로 flex 및 축소 방지를 추가하여 고정 높이를 공용 가상 목록에 전달한다.
- cargo test --offline history_checkout_: 3개 통과. 로컬 우선 후보·원격 대안·팝업 소비·취소·오래된 상태·실제 attached 체크아웃 검증 유지.
- cargo check --offline, cargo clippy --offline --all-targets -- -D warnings 통과.
- cargo rustc --offline --bin feather-git -- -o target/checkout-list-verified.exe 빌드 통과. 실행 중인 기본 실행 파일은 교체하지 않았다.
- 수렴: 표시 영역의 구조적 원인을 수정했고 후보 선택 및 확인·취소 요구와 코드가 일치한다. 새 후보 정책이나 백엔드 변경은 없다.
- 미검증: 실제 앱 화면은 computer-use의 앱 접근 거부로 확인하지 못했다. 시각적인 목록 표시 및 긴 목록 스크롤은 수동 확인이 필요하다. 기존 전체 테스트 결과를 이번 수정 후 재실행 결과로 간주하지 않는다.

## 인코딩 선택 스타일 통일 검증 (2026-11-16)
- 창 폭 400, 행 높이 28, 항목 간격 4, 체크 표시 영역 16, px_3·gap_2·rounded_sm·SELECTED·RAISED를 EncodingDialog와 대조했다.
- cargo test --offline history_checkout_: 3개 통과. cargo test --offline encoding_dialog: 1개 통과.
- cargo check --offline 및 cargo clippy --offline --all-targets -- -D warnings 통과.
- cargo rustc --offline --bin feather-git -- -o target/branch-choice-style-verified.exe 성공. 실행 중 앱은 교체하지 않았다.
- 수동 확인 절차: 같은 커밋의 복수 브랜치 선택 창을 열고 인코딩 선택과 행 스타일을 비교한다. 클릭과 위아래 키가 체크 표시를 이동시키며 확인 전에는 체크아웃하지 않는지 확인한다. 8개 초과 후보에서 스크롤·긴 이름 생략·취소 포커스 복원을 확인한다.
- 미검증: 실제 창 조작과 긴 목록의 시각 확인은 기존 앱 접근 제한으로 실행하지 못했다. 전체 회귀 테스트는 이번 스타일 변경 후 재실행하지 않았다.
- 수렴 결과: FR-008·FR-009·SC-005, 구현 계획의 스타일 및 기존 상태 재사용 결정을 확인했다. 코드상 미구현·부분 구현·모순·범위 외 변경 0건. 헌법은 템플릿으로 구체 규범 검사를 생략한다. 시각 검증 제한은 위에 별도 기록했다.
