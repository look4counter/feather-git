# 데이터 모델

StashApplyFailure: 오류 설명과 정렬·중복 제거된 상대 PathBuf 목록. ToolbarResult는 문자열 오류와 별도 conflict_paths를 보관한다. Alert는 표시용 문자열 경로를 보존하며 빈 목록이면 기존 화면을 사용한다.
스태시 표시: 숫자 순번과 원래 설명. 실제 locator인 stash@{순번}과 예상 객체 ID는 변경하지 않는다. UI는 표시 숫자와 행 인덱스를 함께 검증한다.
