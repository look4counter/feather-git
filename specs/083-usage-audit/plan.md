# 구현 계획: 사용 흐름 점검 도구

## 기술 맥락과 영향 분석

기존 Rust 앱의 테스트를 실행하고 소스 호출 관계를 분석한다. 자동 승인 검토가 feather-git 화면 접근을 거부했으므로 컴퓨터 사용은 중단한다.

commit_history_actions.rs의 command는 libgit2로 검증한 저장소의 workdir만 -C로 전달하고 GIT_DIR 등을 제거한다. 따라서 .git2와 .git의 경로 식별 불일치를 직접 확인한다.

## 설계

- target/qa-run 아래 임시 Rust 도구가 기존 commit_history_actions.rs를 include하여 실제 Reset·Revert 함수를 호출한다. 필요한 상위 함수는 원본 open_exact와 인덱스 충돌 경로 함수를 그대로 사용한다.
- 같은 도구에서 src/repository_search.rs도 include하여 실제 검색 결과를 확인한다. 취소가 없는 정상 실행에 한해 checkpoint를 빈 함수로 둔다.
- 기존 빌드 의존성을 재사용하고 앱 소스·공개 API·의존성은 변경하지 않는다.
- 폐기 가능한 로컬 복제본을 만들고 Git CLI로 전후 HEAD를 비교한다. 재현 프로세스에는 GIT_CEILING_DIRECTORIES를 지정해 주 프로젝트로 탐색하지 못하게 한다.
- 별도 코드 검토로 병합·리베이스 중단, 터미널, 폴더 선택, 상태·검색·복원을 점검한다.
- docs/qa-2026-11-17.md에 결과와 미검증 범위를 기록한다.

## 헌장 점검

헌장은 미작성 템플릿이다. 앱 구조를 변경하지 않고 기존 사용자 변경을 보존한다.

## 검증

전체 테스트, 원본 함수를 호출하는 격리된 재현, cargo check·clippy 및 diff 검사를 사용한다. 재현 도구가 실제 앱 함수와 동일한 파일을 include하는지 확인한다.
