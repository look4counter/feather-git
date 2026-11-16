# 데이터 모델

RepositoryEntry의 이름·PathBuf를 그대로 사용한다. 확인 팝업과 실행 작업은 복사된 항목을 보관한다. 목록 위치 변경으로 다른 항목을 삭제하지 않는다. projects 벡터 인덱스를 보존하고 open_projects·캐시에서만 성공한 삭제 대상을 정리한다.
