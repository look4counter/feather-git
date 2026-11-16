# 조사 결과

- GPUI는 open_with_system과 reveal_path를 제공하므로 OS별 셸 실행을 새로 구현하지 않는다.
- 원본 FileChange.path를 사용하고 표시용 ChangedFile.path를 실제 경로로 재해석하지 않는다.
- 커밋상세 오프셋을 행 검증에서 분리해야 다른 파일에 잘못 적용하지 않는다.
