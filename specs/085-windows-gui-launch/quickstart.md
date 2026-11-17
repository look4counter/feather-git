# 검증 안내

1. cargo build --release --offline을 실행한다.
2. target/release/feather-git.exe의 PE 서브시스템이 GUI(2)인지 확인한다.
3. cargo check --all-targets --offline을 실행한다.
4. 실제 환경에서 실행 파일을 더블클릭하여 앱 창만 표시되는지 확인한다. 개발 빌드는 콘솔 진단을 유지한다.

## 검증 결과

- cargo build --release --offline: 통과. 기존 링커 안내 경고 1건이 남아 있다.
- 실행 파일 PE 헤더의 서브시스템: 2(Windows GUI). 콘솔 자동 생성 방식이 아님을 확인했다.
- cargo check --all-targets --offline: 통과. 개발 및 테스트 타깃이 컴파일된다.
- git diff --check: 통과.
- 실제 GUI 실행과 더블클릭 화면 관찰은 미검증이다. 다른 운영체제의 빌드는 미실행이며 조건부 설정은 Windows 릴리스에만 적용된다.
- 수렴 검토: 요구사항 3개·성공 기준 2개·사용자 시나리오 2개 및 계획의 조건부 설정·기존 시작 흐름 보존을 확인했다. 소스 변경은 시작점 속성 한 줄이며 미해결 구현 불일치는 없다. 헌장은 미작성 템플릿으로 검사를 생략했다.
