# 검증 결과: 참조 종류별 아이콘

검증일: 2026-11-13.

## 단계별 검증

명세 품질 8/8 통과. 명확화 질문 0개. 일관성 분석에서 요구사항 4개가 작업 5개에 대응하며 누락·모순·미해결 모호성은 없다. 헌법은 미작성 템플릿이므로 선언된 원칙 검사를 생략하고 AGENTS.md의 기존 계층·호환성·최소 변경 제약을 적용했다.

## 구현 및 추적성

- FR-001: src/ui.rs의 reference_icon에서 로컬 브랜치·원격 구름·태그 기호를 구분하고 커밋 배지와 저장소 목록이 재사용한다.
- FR-002: src/repository.rs의 ReferenceKind와 CommitReference.kind()가 기존 Git 원격 분류 및 실제 전체 참조 경로로 종류를 판단한다. 이름에 origin·tags가 포함되어도 오인하지 않는다.
- FR-003: 기존 이름과 원격 표시 이름, 선택·스크롤·본문색·현재 배지·공통 목록을 유지한다. 스태시 표시는 바꾸지 않았다.
- FR-004: src/history.rs의 캐시에 종류를 전달하고 미상 문자열 폴백은 아이콘 없이 보존한다. 샘플의 알려진 main·태그도 명시적 참조 메타데이터를 제공한다.

## 자동 검증

종류 분류·아이콘·샘플 테스트를 먼저 추가하여 종류·함수 부재로 실패한 것을 확인한 뒤 구현했다.
- 분류 테스트: 세 종류, 같은 표시 이름, origin·tags 포함 로컬 이름, 원격처럼 보이는 태그, 미상·빈 경로.
- 캐시 테스트: 로컬·원격·태그의 종류와 이름, Rc 공유 재사용, 기존 문자열 폴백과 빈 참조.
- UI 데이터 테스트: 세 아이콘의 구분 및 미상 미표시, 샘플 로컬·태그·빈 참조 메타데이터.
- cargo test --quiet: 69개 통과, 실패 0개, 기존 수동 성능 측정용 2개 제외. 기존 저장소 선택·오래된 태그·스크롤·원격 표시 회귀 포함.
- cargo check, cargo fmt --check, cargo clippy --all-targets: 통과. 기존 vendor 실수 리터럴·테스트 초기화·proc-macro-error2 향후 호환성 경고만 남아 있고 새 코드 경고는 없다.
- cargo rustc --bin feather-git -- -o target/feather-git-reference-kind-icons.exe: 컴파일·링크 성공.

## 미검증 사항

실제 GUI에서 브랜치·태그 글리프와 배지 배치를 직접 확인하지 않았다. Git 메타데이터가 없어 git diff/status는 사용하지 못한다.

## 브랜치 아이콘 형태 개선 검증

명세 FR-005·SC-003과 계획·작업 목록을 갱신하고 품질 점검 8개 항목을 재검토했다. 명확화 질문은 없고 일관성 분석에서 누락·모순은 없다.
소스 변경은 src/ui.rs에 한정한다. 로컬 아이콘은 16픽셀 영역 안에서 2.5픽셀 굵기의 둥근 연결선과 지름 5픽셀의 채워진 원형 노드 3개로 구성된다. 공통 reference_icon_view를 커밋 배지와 저장소 목록이 사용한다. 기존 원격·태그 기호는 유지하며 배지는 이름만 말줄임하고 아이콘은 축소하지 않는다. 현재 색상·공통 행 높이·선택 연동은 유지한다.

- 기존 아이콘 대응 테스트를 Branch/Glyph 표현에 맞게 갱신하고 로컬이 벡터 Branch 타입인지 확인했다.
- cargo test --quiet: 69개 통과, 실패 0개, 기존 수동 성능 측정용 2개 제외.
- cargo check, cargo fmt --check, cargo clippy --all-targets: 통과. 기존 vendor·테스트 초기화·향후 호환성 경고만 남아 있다.
- cargo rustc --bin feather-git -- -o target/feather-git-bold-branch-icon.exe: 컴파일·링크 성공.
- GUI에서 두께·둥근 노드와 배치의 시각 확인은 직접 수행하지 않았다.

## 최신 검증: 로컬 컴퓨터 아이콘

FR-006·SC-004에 따라 로컬 브랜치 형태 요구를 컴퓨터 모양으로 대체했다. 명세·계획·작업 목록 및 품질 점검을 재검토했으며 미해결 질문·누락·모순은 없다.
변경 파일은 src/ui.rs이다. ReferenceIcon::Computer와 computer_icon을 사용하여 16픽셀 영역에 둥근 화면 테두리·받침대·발판을 구성한다. 커밋 배지와 저장소 목록은 동일한 공통 뷰를 계속 사용한다. 원격·태그 기호와 크기·색상·선택 동작은 유지한다.

- 기존 아이콘 대응 테스트에서 로컬 타입이 Computer인지 확인한다.
- cargo test --quiet: 69개 통과, 실패 0개, 기존 수동 성능 측정용 2개 제외.
- cargo check, cargo fmt --check, cargo clippy --all-targets: 통과. 기존 vendor·테스트 초기화·향후 호환성 경고만 남아 있다.
- cargo rustc --bin feather-git -- -o target/feather-git-local-computer-icon.exe: 컴파일·링크 성공.
- 실제 GUI에서 컴퓨터 모양과 배치를 직접 확인하지 않았다.
