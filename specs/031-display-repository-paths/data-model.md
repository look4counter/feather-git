# 데이터 모델

기존 PathBuf·RepositoryEntry·Session·Request는 변경하지 않는다. 표시 문자열은 원본 경로를 읽어 만든 일회성 값이다. 표시 결과는 다시 저장하거나 I/O 입력으로 사용하지 않는다.
