# 데이터 모델

## 저장소 탭의 조회 상태

- generation: 현재 화면 조회 결과를 적용할 요청 번호.
- history_generation: 히스토리 차단을 소유하는 새로고침 요청 번호. 영속 저장하지 않는다.
- history_loading: 소유한 새로고침의 완료 전까지 유지한다.
- loading: 현재 화면 조회의 진행 상태. 오래된 완료는 바꾸지 않는다.

## 상태 전이

1. 새로고침 시작: generation 증가, history_generation에 복사, 두 로딩 상태 설정.
2. 커밋 선택: 기존대로 generation 증가와 loading 설정. 히스토리 소유권 보존.
3. 새로고침 완료: history_generation 일치 시 history_loading 해제. generation도 일치할 때만 loading 해제 및 결과 적용 허용.
4. 닫힌 탭: 콜백에서 조회 대상이 없으므로 상태 적용 없음.

성공과 실패의 정리 규칙은 동일하다.
