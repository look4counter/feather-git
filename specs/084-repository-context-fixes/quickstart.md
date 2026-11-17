# 검증 안내

프로젝트 루트에서 TEMP·TMP를 target/test-tmp로 지정하고 cargo test, cargo check --all-targets, cargo clippy --all-targets를 실행한다. 각 회귀 테스트는 폐기 가능한 저장소만 사용한다. 실제 UI·외부 원격 인증은 별도의 실사용 검증 범위이다.

## 실행 결과 — 2026-11-17

- 수정 전 새 회귀 테스트에서 .git2 이력 작업 실패·잘못된 저장소 선택, 검색 메타데이터 노출, 터미널 환경 누락을 먼저 재현했다.
- 전체 테스트: **524개 통과, 실패 0개, 기존 성능 측정 3개 제외**. 로그: target/084-test-results.log.
- Reset 3모드×3배치, Revert 2옵션×3배치, Revert 계속·중단 4조합×3배치, Merge/Rebase 중단 각 3배치를 검사했다. 비대상 HEAD·인덱스·작업 상태를 보존했다.
- Git 명령의 명시적 디렉터리 및 외부 경로 환경 제거, 한글·공백·특수문자 경로의 두 터미널 환경, 베어 저장소의 작업 폴더 제거를 검증했다.
- 검색은 실제 .git2·사용자 지정 메타데이터를 제외하고 정상 .git2-notes·.gitignore와 커밋 검색을 유지했다.
- 최초 점검 도구를 현재 원본 코드로 다시 빌드하여 동위 Reset·Revert, 단독 Reset, 상위 저장소가 있는 Reset, 메타데이터 검색을 재실행했다. 다섯 사례 모두 기대 결과와 일치했다. 결과: target/qa-run/084-probe-results.json.
- cargo check --all-targets와 cargo clippy --all-targets가 종료 코드 0으로 통과했다. Clippy 경고는 없었다. git diff --check도 통과했다.
- Windows 네이티브 폴더 선택의 숨김 플래그와 기존 폴더·파일·다중 선택 조건을 코드로 확인하고 Windows 종속 코드 빌드를 통과했다.
- 이전 자동 승인 검토가 feather-git 화면 접근을 거부했으므로 실제 폴더 선택창과 터미널 창 조작은 수행하지 않았다. 외부 원격 인증과 다른 OS 빌드도 미검증이다.
