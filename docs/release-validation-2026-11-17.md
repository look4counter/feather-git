# Release 배포 후보 검증 결과

**검증일**: 2026-11-17  
**판정**: 최신 Release 빌드·자동화·실행 파일의 비화면 검증은 통과했다. 실제 GUI 및 실제 원격 인증 검증은 미완료이므로 정식 배포 검증 완료로 판정하지 않는다.

## 검증한 배포 파일

- 파일: [feather-git.exe](../target/release/feather-git.exe)
- 빌드: `cargo build --release --locked`, 현재 작업 트리의 미커밋 수정 포함
- 버전: Cargo.toml 기준 1.0.0
- 아키텍처: Windows x64
- 크기: 27,922,944바이트
- SHA-256: `fa62669c9e05790aaf9e18050aa0c6142f70d37f19fca760f8e149736a678b4a`
- 테스트용 빈 폴더에 실행 파일만 복사한 뒤 동일 해시임을 확인했다.
- [실행 파일 검사 결과](../target/release-validation/20261117-195323/artifact-check.json), [빌드 소스 상태](../target/release-validation/20261117-195323/build-state.json)

기존 Release 파일은 마지막 수정 이전 파일이었으므로 이번 검증에서 최신 소스로 다시 빌드했다. 실제 배포 시에는 위 해시와 일치하는 파일을 기준으로 해야 한다.

## 실행한 검증

| 항목 | 결과 | 검증 범위 |
| --- | --- | --- |
| 잠금 파일 기준 Release 빌드 | 통과 | 최적화 프로필 및 Windows 네이티브 코드 컴파일·링크 |
| Release 전체 자동화 테스트 | 524개 통과, 0개 실패, 기존 성능 측정 3개 제외 | `cargo test --release --locked -- --test-threads=4` |
| 배포 파일 CLI 실행 | 9개 통과 | 도움말, 유효한 지연 경계값, 잘못된 옵션·범위·중복·값 누락의 종료 코드 |
| 실행 파일 단독 복사 | 통과 | 소스 디렉터리 밖의 별도 빈 작업 폴더에서 CLI 실행 |
| CLI의 설정 부작용 | 없음 | 격리한 LOCALAPPDATA 경로에 설정이 생성되지 않음 |
| Git 작업 및 충돌 회귀 | 통과 | 자동화의 로컬 원격 Clone·Fetch·Pull·Push, 브랜치·커밋·충돌·계속·중단 |
| .git2 비대상 저장소 보호 | 통과 | 단독·동위·상위 저장소, Reset/Revert 및 Merge/Rebase 중단 |
| 터미널 저장소 환경 | 통과 | 실제 명령 구성의 환경을 적용한 자식 Git이 선택 저장소를 반환함 |
| 세션·최근 목록 복원 | 통과 | 저장·읽기·복원 상태 및 백그라운드 처리 자동화. 실제 앱 재실행은 아님 |
| 실제 GUI 시작·화면 조작 | 미검증 | 도구의 자동 승인 검토가 앱 접근을 거부함 |
| 실제 GitHub HTTPS 인증과 Push | 연결 실패·인증 미도달 | 사용자 지정 nonmac의 origin에서 GitHub 443 연결 실패 |
| nonmac 이력 기반 로컬 원격 흐름 | 7단계 통과 | 별도 복제본의 Git CLI로 Clone·Commit·Push·Fetch·Pull 확인 |
| 새 Windows PC·가상 머신 설치 | 미검증 | 현재 개발 PC에서만 실행 |

`--help`와 오류 옵션은 GUI 초기화 전에 종료하므로 위 CLI 검증을 GUI 시작 검증으로 간주하지 않는다. 터미널 환경 테스트도 실제 Git Bash·PowerShell 창을 조작한 검증과 구분한다.

## 배포 환경에서 확인할 사항

- PE 의존성에 `VCRUNTIME140.dll`, `icuuc.dll` 및 Windows 그래픽·시스템 DLL이 있다. 이 PC에는 해당 런타임이 존재한다. 런타임이 없는 새 PC에서 실행 가능한지는 검증하지 않았다.
- 앱의 일부 이력 변경·중단 동작은 외부 Git 명령을 실행하므로 대상 PC의 Git 설치와 PATH 확인이 필요하다. 이번 환경의 Git은 `2.40.1.windows.1`이다.
- 최신 실행 파일의 Authenticode 상태는 `NotSigned`이다. 디지털 서명은 적용하지 않았다.
- PE 서브시스템은 `IMAGE_SUBSYSTEM_WINDOWS_CUI`이다. 소스와 Windows 플랫폼에서 콘솔을 숨기는 처리는 발견하지 못했다. 일반 실행 시 콘솔 창이 동반되는 배포 형태인지 확인하고 제품의 의도와 맞는지 결정해야 한다. 실제 화면 관찰은 하지 못했다.
- 설치 프로그램·자동 업데이트·배포용 서명 설정은 이번에 찾지 못했다. 이번 산출물은 실행 파일 기준이다.

## 실제 화면 검증의 차단 원인

사용자 요청 후 다시 정식 컴퓨터 사용 도구로 접근을 시도했으나 `Computer Use was not approved to use feather-git`으로 거부됐다. 다른 조작 도구나 우회 수단은 사용하지 않았다. 따라서 숨김 Git 디렉터리 선택, 실제 Commit/Push 버튼, 화면 복원 및 배포 파일의 GUI 동작을 확인했다고 보고할 수 없다.

사용자가 `D:\Projects\nonmac`을 지정했다. 해당 저장소의 origin은 GitHub HTTPS 원격이다. 인증 창을 띄우지 않는 읽기 명령 `ls-remote origin HEAD`를 실행했으나, 443번 포트 연결에 실패하여 인증까지 도달하지 못했다. 따라서 인증정보가 잘못됐거나 Push 권한이 없다는 판정은 할 수 없다. 실제 GitHub에는 테스트 커밋이나 브랜치를 올리지 않았다.

## 사용자 지정 nonmac의 격리 검증

원본에는 미커밋 변경 25개가 있어 읽기만 수행했다. 커밋 이력을 프로젝트의 target 아래 bare 원격으로 복사하고 독립된 두 복제본을 만들었다. 하드링크 없이 복제하고 테스트 명령의 훅 실행과 서명을 비활성화했다.

1. 원본 커밋 이력을 테스트용 bare 원격으로 복제했다.
2. 독립된 두 작업 복제본을 만들었다.
3. 첫 복제본에서 테스트 브랜치와 커밋을 만들고 Push했다.
4. 다른 복제본에서 Fetch·체크아웃하여 파일 내용을 확인했다.
5. 다른 복제본의 후속 커밋을 Push했다.
6. 첫 복제본에서 Fetch가 원격 참조만 갱신하고 로컬 HEAD를 보존함을 확인했다.
7. Pull fast-forward 후 HEAD·작업 파일이 후속 커밋과 일치하고 작업 트리가 깨끗함을 확인했다.

모든 단계가 통과했다. 원본 HEAD와 상태 출력이 검증 전후 완전히 동일함을 비교했다. 원본의 미커밋 변경 25개는 보존됐다. 이 흐름은 **Git CLI의 실제 파일·원격 동작 검증이며 앱 GUI 조작 검증은 아니다**.

- [nonmac 원격 연결 결과](../target/release-validation/20261117-195323/nonmac-remote-check.json)
- [nonmac 로컬 원격 흐름 결과](../target/release-validation/20261117-195323/nonmac-smoke/results.json)

## 근거

- [Release 빌드 로그](../target/release-validation-build.log)
- [Release 테스트 로그](../target/release-validation-tests.log)
- [앞선 결함 수정·재현 검증](qa-2026-11-17.md)

앱 기능 소스는 이번 검증에서 변경하지 않았다. Release 실행 파일과 검증 기록만 새로 만들었다.
