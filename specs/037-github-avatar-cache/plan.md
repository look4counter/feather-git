# 구현 계획

## 실제 구조
Rust 2024·GPUI 앱이다. 상세 조회는 백엔드 작업에서 DetailIdentity의 Gravatar URL을 만든다. avatar.rs는 GPUI HTTP 어댑터·전용 스레드 1개·대기열 8개·3초 제한·256KiB 응답 제한을 제공한다. GPUI의 이미지 캐시만으로는 커밋 API나 이메일별 매핑의 세션 수명을 보장하지 못한다.

## 설계
repository.rs에 원격 조회 게이트웨이를 추가해 origin 우선으로 GitHub 저장소를 찾는다. 외부 서비스별 원격 파싱은 avatar.rs의 순수 함수로 분리한다. diff_viewer.rs의 상세 데이터 생성에서 원격을 읽고 인물별 내부 사진 URI를 만든다. 내부 URI에는 저장소·전체 SHA·역할·이메일 해시만 포함한다.

사진 작업 스레드가 내부 URI를 받아 GitHub 커밋 API에서 역할별 avatar_url을 얻고 사진을 조회한다. 실패하면 Gravatar로 넘어간다. UI·Git 작업에서는 네트워크를 실행하지 않는다. 상세와 원격 확인은 하나의 백엔드 요청 경계로 묶어 설정된 지연이 두 번 적용되지 않게 한다. 작업 스레드 소유 SessionCache에 커밋 API 결과·이메일 해시별 프로필·실제 사진 바이트와 실패를 저장한다. 재시작 때만 초기화하며 GPUI 캐시가 이미지를 다시 요구해도 외부 요청 없이 반환한다. GitHub 미연결 이메일은 음성 캐시로 보관한다. 이메일이 비면 프로필 매핑을 공유하지 않고 커밋별 결과만 사용한다.

원격은 HTTPS·ssh://·git@github.com: 형태를 처리하고 정확한 호스트와 owner/repo 경로를 검증한다. 외부 사진 URL은 GitHub 아바타 호스트와 기존 Gravatar에 한정한다. API 응답은 필요한 author/committer.avatar_url만 보관한다. 비인증 API 실패도 캐시해 반복 호출하지 않는다.

## 영향·헌법
변경: src/avatar.rs, src/repository.rs, src/backend.rs, src/diff_viewer.rs. 새 의존성·공개 API·세션 디스크 형식 변경은 없다. 헌법은 미작성 템플릿이므로 규범 검사는 건너뛰며 AGENTS.md의 계층 분리·최소 변경을 따른다.

## 내부 서버 조회 설계

공개 GitHub를 기존대로 우선한다. 발견하지 못하면 `src/repository.rs`와 `src/backend.rs`를 통해 사용 가능한 우선 원격의 웹 주소를 전달한다. `src/avatar.rs`의 기존 작업 스레드에서 Gitea와 Bitbucket Data Center 커밋 응답을 순서대로 시도하고 같은 출처의 이미지만 조회한다. 두 역할이 커밋 응답과 이미지 캐시를 공유한다. Gitea 커밋의 최상위 계정에 사진 주소가 없으면 연결된 계정명으로 같은 설치 경로의 사용자 API를 조회하고 해당 결과를 세션에 캐시한다. 원격 주소의 인증정보는 제거하며 같은 출처의 리다이렉트만 허용한다.

## 검증
외부 요청 함수를 주입한 테스트로 원격 파싱, 역할별 URL, 다른 커밋의 이메일 캐시, 같은 커밋 API 공유, 이미지 캐시, 실패 대체와 음성 캐시를 검증한다. 임시 Git 저장소의 origin 우선순위도 검증한다. fmt/check/test/clippy/build를 수행한다. 실제 GUI·외부 API는 미검증이면 명시한다.
