# 조사 결과

[Gravatar 해시 규칙](https://docs.gravatar.com/rest/hash/)은 이메일의 양끝 공백 제거와 소문자 처리 후 SHA256을 사용한다. [사진 API](https://docs.gravatar.com/sdk/images/)의 d=404로 기본 사진 대신 없음 응답을 받아 앱의 기본 아이콘을 유지한다.

GPUI img는 동일 URL의 로딩과 결과를 캐시하며 loading·fallback을 지원한다. 현재 앱은 HTTP 구현이 없으므로 어댑터가 필요하다. 사진을 별도 스레드에서 조회하면 Git의 두 작업 스레드를 점유하지 않는다. 선택·복사 인덱스를 유지하려고 두 개의 고정 높이 행에 같은 사진을 클리핑한다.
