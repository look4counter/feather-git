# 검증 결과: 커밋 원격 참조 아이콘

검증일: 2026-11-13.

## 단계별 검증

명세 품질 8/8 통과. 명확화 질문 0개. 명세·계획·작업 목록의 일관성 분석에서 요구사항 4개가 모두 작업 6개에 대응하며 누락·모순·미해결 모호성은 없다. 헌법은 미작성 템플릿이므로 선언된 원칙 검사는 생략하고 AGENTS.md의 계층·호환성·최소 변경 제약을 적용했다.

## 구현 및 요구사항 추적

- FR-001: src/ui.rs의 커밋 배지는 참조마다 표시되며 원격 참조에 구름 기호를 붙인다. origin/main → ☁ main, upstream/feature/login → ☁ feature/login.
- FR-002: src/repository.rs에서 reference.is_remote() 값을 참조 메타데이터에 보존한다. origin 문자열로 원격을 판단하지 않는다.
- FR-003: 참조별 이름을 별도로 전달하므로 로컬·태그·슬래시·쉼표가 있는 브랜치 이름도 유지한다. 등록 원격 중 가장 긴 접두사를 제거하며 등록되지 않은 원격 추적 참조도 처리한다.
- FR-004: 기존 Commit.reference 결합 문자열과 Project.remotes를 유지한다. src/history.rs가 메타데이터를 Rc 공유 캐시에 저장하며 기존 문자열만 있는 데이터는 원래 이름으로 폴백한다. 샘플 데이터도 호환한다. 공통 목록·그래프·선택 로직은 변경하지 않았다.

## 자동 검증

- 테스트를 먼저 추가하여 Commit.references 및 PreparedHistoryRow.references 부재로 실패한 것을 확인하고 구현했다.
- 실제 저장소 테스트: origin·upstream·team/upstream 등 여러 원격, 등록 정보가 없는 원격 추적 참조, 중첩·쉼표 이름, 로컬 origin/topic·remotes/test, 태그 origin/release, 원래 문자열과 원격 패널 목록 보존.
- 캐시 테스트: 참조 종류·표시 이름, Rc 재사용, 기존 문자열 폴백 및 빈 참조.
- cargo test --quiet: 63개 통과, 실패 0개, 기존 수동 성능 측정용 2개 제외.
- cargo check, cargo fmt --check, cargo clippy --all-targets: 통과. 기존 vendor 실수 리터럴·테스트 초기화·proc-macro-error2 향후 호환성 경고가 남아 있고 새 코드 경고는 없다.
- cargo rustc --bin feather-git -- -o target/feather-git-remote-reference-icon.exe: 실행 파일 컴파일·링크 성공.

## 미검증 사항

GUI에서 구름 글리프 및 배지 배치를 직접 확인하지 않았다. 실제 저장소·캐시·회귀 테스트와 소스 검토로 검증했으며 시각 검증과 구분한다. Git 메타데이터가 없어 git diff/status는 사용하지 못한다.
