# 검증 안내

cargo check --offline, cargo test --offline, cargo clippy --offline --all-targets, cargo fmt --all -- --check 및 별도 출력 빌드를 실행한다. 새 빌드 출력에서 대상 경고와 향후 호환성 보고서 경고가 사라졌는지 확인한다.


## 실행 결과 (2026-11-15)

- cargo build --offline: 종료코드 0, 일반 빌드 경고 없음.
- cargo check --offline --future-incompat-report: 종료코드 0, 향후 호환성 경고가 있는 의존성 0개.
- cargo clippy --offline --all-targets -- -D warnings: 종료코드 0.
- cargo fmt --all -- --check: 종료코드 0.
- cargo test --offline: 348 통과, 실패 0, 수동 성능 측정 3 무시.
- 별도 출력 실행 파일 target/warnings-clean-verified.exe 생성 성공. 이 방식의 -o 출력 경로 안내 2건과 MSVC import library 생성 안내는 일반 소스 경고와 구분한다. 테스트 링크 시에도 import library 생성 안내 1건이 있었다.
- 사용자 프로세스와 전역 설정을 변경하지 않았다. 화면 동작은 미검증이며 이번 변경은 UI 동작 변경을 포함하지 않는다.

## 수렴 결과

FR-001은 함수별 cfg(test) 전달 및 호출 관계, FR-002는 미사용 필드·메서드 제거와 그래프 테스트 유지, FR-003은 f32 리터럴, FR-004는 같은 버전의 로컬 가시성 패치, FR-005는 전체 검사 결과로 확인했다. 미구현 요구사항과 명세·구현 불일치 없음. 로컬 패치의 라이선스 및 원본 테스트를 보존했다.

## Windows 링커 안내 정리 검증

- 변경 전 cargo build --release --offline에서 임포트 라이브러리·내보내기 보조 파일 생성 안내 경고 1건을 재현했다.
- build.rs가 Windows MSVC의 feather-git 실행 파일에만 /NOIMPLIB·/NOEXP를 전달한다. 경고 억제 속성이나 전역 설정은 추가하지 않았다.
- 변경 후 실제 재링크한 cargo build --release --offline: 종료코드 0, 경고 0건.
- PE 헤더 전후 비교: 내보내기 이름 950개 전부 동일, GUI 서브시스템 2 유지.
- cargo check --all-targets --offline: 종료코드 0, 경고 없음. 개발·테스트 타깃 컴파일 통과.
- rustfmt --check build.rs 및 git diff --check: 통과.
- 이번 추가 변경에서는 전체 테스트 실행·실제 GUI 실행·다른 플랫폼 빌드를 수행하지 않았다. 이전 테스트 실행 기록은 재검증으로 취급하지 않는다.
- 수렴 점검: 추가 요구사항 FR-006~FR-008·성공 기준 SC-002·사용자 이야기 2의 수용 시나리오와 계획의 대상 제한·보조 파일 생략·내보내기 보존을 확인했다. 미해결 불일치 0건이다. 기존 기능 구현은 수정하지 않았다. 헌장은 미작성 템플릿으로 검사를 생략했다.
