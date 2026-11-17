# 구현 계획: Windows 배포 실행 시 콘솔 제거

## 기술 맥락 및 기존 구현 분석

Rust 2024·GPUI 데스크톱 앱이다. src/main.rs에 실행 서브시스템 지정이 없어 Windows 콘솔 실행 파일이 생성된다. main은 인수 해석 → 백엔드 초기화 → GPUI 창 생성 → 종료 정리로 이어진다. repository.rs와 credentials.rs의 백그라운드 명령에는 이미 숨김 실행 설정이 있다. workspace_backend.rs의 명시적 터미널은 CREATE_NEW_CONSOLE을 의도적으로 사용한다.

## 설계 및 영향 범위

src/main.rs 최상단에 Windows이면서 debug_assertions가 꺼진 경우에만 windows_subsystem = "windows"를 적용한다. 기존 main과 자식 프로세스는 수정하지 않는다. 신규 의존성·API·데이터 변경은 없다. 실행 중 콘솔 숨김 API보다 빌드 시 GUI 서브시스템 선택이 작고 직접적인 해결이다.

## 헌장 점검

헌장은 미작성 템플릿이므로 해당 검사는 생략한다. 사용자 지침의 계층 책임·기존 API·의존성을 보존한다. 설계 전후 아키텍처 충돌은 없다.

## 파일 구조

소스 변경은 src/main.rs 한 곳이다. 문서는 specs/085-windows-gui-launch 아래에 둔다.

## 검증

cargo build --release --offline으로 실제 Windows 실행 파일을 생성하고 PE 헤더의 Subsystem 값이 2(GUI)인지 검사한다. cargo check --all-targets --offline으로 개발·테스트 컴파일을 확인한다. 실행 조건과 기존 초기화 코드의 diff를 검토한다. 실제 GUI 시각 검증은 별도로 미검증 상태를 명시한다. 단순 속성을 복제하는 테스트는 추가하지 않는다.
