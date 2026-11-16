# 구현 계획: 환경설정 오버레이

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

## 기술 환경과 구조
Rust, GPUI 0.2.2, git2 0.21, gix 0.90, 기존 Windows Credential API를 사용한다. 의존성을 추가하지 않는다. Workspace의 하단 상태바에서 SettingsPanel을 열고 공용 Dialog로 표시한다. 설정 및 인증 UI, 설정 접근, 비밀 보관을 분리한다.

## 기존 구현 분석과 영향 범위
현재 Dialog는 모달 차단·크기·포커스·키 처리를 제공한다. 하단 버튼은 status_button을 재사용한다. 기존 TextInput은 일반 한 줄 입력으로 비밀번호·읽기 전용·비활성 처리가 없어 필요한 최소 상태와 표시 좌표 매핑을 추가한다. 기존 저장소 접속은 repository::credentials의 helper callback을 사용한다. 등록한 비밀번호 프로필을 이 경로에 연결하고 기존 helper fallback은 유지한다.
참조 프로젝트 SettingsPanel/AuthPanel 및 settings_git/auth_profiles/credentials의 검증된 로직과 테스트를 가져온다. 전체 native_git나 CommitEditor는 복제하지 않는다. 설정용 어댑터는 필요한 config/remote 명령만 인프로세스로 수행한다. 설정 오류와 값 없음의 종료코드를 구분한다.

## 설계 및 불변 조건 확인
헌법은 미작성 템플릿이므로 추가 규칙이 없다. AGENTS.md의 계층 분리·호환성·최소 변경을 따른다. UI는 비동기로 설정 접근을 실행한다. Git CLI 프로세스로 설정을 변경하지 않는다. 인증 helper만 별도 프로세스로 실행하며 취소·시간제한을 보존한다. 비밀번호는 운영체제 보관소에만 저장하고 입력 이벤트/복사/표시에 노출하지 않는다. 전역 설정 테스트는 격리 파일만 사용한다.

## 구현 단계
1. 참조 UI/backend와 필요한 소규모 어댑터 이식.
2. TextInput의 읽기 전용·비밀번호·비활성 모드와 Unicode 좌표 테스트.
3. Workspace 모달 수명주기, 하단 버튼, 키보드·포커스·하위 창 연결.
4. 저장소 인증 callback 연결과 회귀 검증.

## 검증
격리된 설정/원격 테스트, 모의 비밀 보관소와 인증 helper 테스트, 입력 마스크 좌표 및 입력 정책 테스트, 전체 cargo test --offline, cargo clippy, cargo fmt 확인, 실행 파일 별도 이름 빌드. 실제 전역 설정과 자격증명을 개발 과정에서 변경하지 않는다. 실행 중인 앱을 종료하지 않으며 화면 확인 제한은 결과에 명시한다.


## 확인 동작 변경 계획

인코딩의 임시 커서와 확정 이벤트를 분리하고 선택/확인/취소 상태 테스트를 갱신한다. 환경설정은 성공 이벤트를 Workspace가 구독하여 저장 성공 때만 닫는다. 저장 실패는 기존 오류 표시를 유지한다. 창별 키 핸들러와 단축키 힌트를 제거하고 모든 하단을 확인·취소로 통일한다. 공용 Dialog, 기존 저장 및 인증 계층과 비밀 보호를 유지한다. 영향 파일은 encoding_dialog.rs, settings.rs, auth_settings.rs, ui.rs, form_controls.rs이며 기능 검증 후 전체 테스트·정적 분석·빌드를 수행한다.
