# 검증 결과

## 워크플로우와 일관성

명세·명확화·계획·작업·일관성 분석·구현·수렴을 수행했다. 추가 질문 0개, 명세 품질 점검 4/4 통과다. FR-001~005는 T003~005에 대응한다. 실제 재현 후 인덱스 stat 가설을 기각하고 설치 설정 누락과 FastEq 크기 차이 경계를 반영해 명세·계획·계약·작업의 일관성을 다시 점검했다. 미해결 누락·모순은 없다. Constitution은 미작성 템플릿이며 확장 훅은 없다.

## 실제 저장소 재현

- D:/Projects/feather-git-old의 Git --no-optional-locks status --porcelain 결과는 종료 코드 0, 변경 0개였다.
- Git config --show-origin은 core.autocrlf=true가 Git 설치 설정에서 오고 core.filemode=false는 저장소 설정에 있음을 확인했다. LF·CRLF·혼합 줄바꿈 파일이 있다.
- 구현 전 gix는 core.autocrlf가 없었고 libgit2는 true를 읽었다. 앱 조회는 85개를 잘못 반환했다. 상태 iterator의 유지보수 항목은 0개여서 최초 stat 가설은 직접 원인이 아니었다.
- 수정 후 명시적 외부 저장소 테스트에서 meaningful=0, listed=0을 확인했다. 조회 전후 인덱스 바이트도 동일했다. 작업 파일이나 저장소 설정을 바꾸지 않았다.

## 구현 결과

src/diff_git.rs의 open에서 기존 libgit2 기본 설정의 core.autocrlf·core.eol·core.safecrlf만 gix에 누락된 경우 메모리 snapshot으로 보완한다. 이미 있는 저장소·사용자 값을 덮어쓰지 않으며 외부 필터·명령 설정은 적용하지 않는다.

추가 회귀에서 gix-status FastEq가 원문 크기만 다른 LF/CRLF 파일을 clean 변환 전에 수정으로 반환함을 확인했다. 정규화가 true/input인 일반 파일의 내용 변경 중 크기가 다른 경우만 기존 clean 파이프라인의 출력을 작은 버퍼로 인덱스 blob과 비교해 동일하면 제외한다. 실행 권한·타입·삭제·충돌·Staged 상태는 이 검사로 제외하지 않는다. 실제 내용 차이는 계속 표시한다.

## 자동 검증

- cargo test --offline: 241개 통과, 실패 0개, 3개 제외. 제외 항목은 기존 성능 측정용 2개와 이번에 추가한 명시적 외부 저장소 검증 1개다.
- 외부 검증은 FEATHER_VERIFY_CLEAN_REPO=D:/Projects/feather-git-old 및 cargo test --offline external_clean_repository -- --ignored --nocapture로 별도 실행해 통과했다.
- 새 자동 회귀 2개는 기본 줄바꿈 상속·로컬 우선순위·LF/CRLF 동등성·실제 수정 보존·설정 파일 미변경, stat 갱신만 있는 인덱스와 실제 변경의 구분·인덱스 미변경을 검증한다. 기존 Stage·충돌·추가·삭제·정렬 테스트도 통과했다.
- cargo fmt --check: 통과.
- cargo clippy --offline --all-targets: 통과. 기존 미사용 코드·중첩 조건·의존성 경고는 남아 있다.
- cargo build --offline: 컴파일·링크 후 target/debug/feather-git.exe 교체에서 액세스 거부로 실패했다. 실행 파일 잠금으로 보이며 기존 앱을 종료하지 않았다. 테스트 실행 파일은 컴파일·링크·실행에 성공했다.

## 수렴 및 미검증 범위

FR-001~005, SC-001~002, 실제 저장소 증상과 계획의 설정 우선순위·정규화·읽기 전용 범위를 코드와 테스트로 대조했다. 미구현·부분 구현·상충·범위 밖 추가 항목은 0건이며 추가 구현 작업은 없다. 실행 파일 교체는 미완료이고 현재 실행 중인 앱 화면에서의 결과는 미검증이다. 이 제한을 실제 조회 엔진과 테스트의 성공 결과와 구분한다.
