# 조사 결과

- 기존 SingleSelection.select는 동일 인덱스일 때 false를 반환하므로 새 선택 모델은 필요 없다.
- GPUI 콜백의 notify는 이미 bool로 제어하므로 선택 메서드의 조기 반환으로 후속 갱신을 차단할 수 있다.
