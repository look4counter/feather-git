# 데이터 모델

기존 Project·Request·Session 형식은 유지한다. Git 디렉터리는 repo.path, 작업 폴더는 repo.workdir을 사용한다. 메타데이터 수집 함수는 작업 폴더 바로 아래의 실제 Git 디렉터리 절대 경로 목록을 반환한다. 터미널 환경은 실행할 자식 프로세스에만 적용한다.
