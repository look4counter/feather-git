# 버전 목록 형식

UTF-8 XML의 루트는 `versions`이며 자식 `version`의 `id`, `version`, `url` 속성을 사용한다. Feather Git ID에 대해서만 파싱하며 중복은 오류다. 다운로드 URL은 공식 GitHub 저장소의 버전별 릴리스 자산이다.
