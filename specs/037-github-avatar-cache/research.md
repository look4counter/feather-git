# 조사 결과

[GitHub Get a commit](https://docs.github.com/en/rest/commits/commits#get-a-commit)은 최상위 author와 committer의 avatar_url을 제공하며 공개 저장소는 인증 없이 조회할 수 있다. 계정이 연결되지 않으면 null일 수 있다. 기존 이름·이메일 문자열을 계정 이름으로 추측하지 않고 저장소와 SHA로 연결한다.

현재 사진 작업 스레드에 캐시를 소유시키면 락 경쟁이나 새 UI 상태를 만들지 않고 재시작 전 캐시 수명을 보장한다. 같은 이메일은 다음 커밋에서도 프로필을 재사용하고 사진 바이트는 URL로 공유한다. GPUI 캐시와 별개로 작업 스레드 캐시를 두어 외부 재요청을 막는다.
