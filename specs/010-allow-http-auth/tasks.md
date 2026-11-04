# 작업 목록: HTTP 인증 주소 지원

입력: [명세](spec.md), [계획](plan.md), 조사 및 계약 문서.

## 1단계: 준비

- [x] T001 specs/010-allow-http-auth/spec.md와 설계 문서의 요구사항 및 영향 범위를 확인한다.

## 2단계: 기반 검증

- [x] T002 src/auth_profiles.rs와 src/credentials.rs에 HTTP 요청·등록·가져오기·프로토콜 분리 회귀 테스트를 추가하고 기존 HTTP 거부 사례를 미지원 프로토콜로 바꾼다. FR-001~004.

## 3단계: 사용자 이야기 1 — HTTP 인증

- [x] T003 [US1] src/auth_profiles.rs에서 HTTP 주소 검증·요청·가져오기·원격 연계를 지원한다. FR-001~004.
- [x] T004 [US1] src/credentials.rs에 로그인 대상 프로토콜을 보존하고 src/native_remote.rs의 계정 탐색에 HTTP를 포함한다. FR-001~003.
- [x] T005 [US1] src/auth_settings.rs와 인증 검증 오류에서 HTTP/HTTPS 지원을 안내한다. FR-005.

독립 검증: HTTP 프로필 저장, 도우미 입력 및 원격 연결 테스트가 통과한다.

## 4단계: 사용자 이야기 2 — 기존 동작 유지

- [x] T006 [US2] src/auth_profiles.rs와 src/credentials.rs의 HTTPS·프로토콜 격리·잘못된 입력 회귀 테스트를 실행한다. FR-004, SC-001~003.

독립 검증: HTTPS 기존 테스트와 미지원 주소 거부가 통과한다.

## 5단계: 마무리

- [x] T007 cargo test와 cargo fmt --all -- --check를 실행하고 specs/010-allow-http-auth/quickstart.md 기준 결과를 확인한다.

## 의존성과 실행 전략

T001 → T002 → T003 → T004 → T005 → T006 → T007 순서로 진행한다. 사용자 이야기 1을 최소 기능으로 구현하고 사용자 이야기 2에서 회귀를 확인한다. 서로 다른 파일의 T003과 T004는 테스트 준비 후 병행 가능하지만 이번 실행은 순차 처리한다.
