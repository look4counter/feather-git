# 내부 계약

팝업 준비는 저장소를 변경하지 않는다. 확인만 쓰기를 시작한다. CommitRequest의 기대 HEAD/참조가 다르면 실패한다. 인덱스 변경만 커밋하며 작업 파일은 변경하지 않는다. commit 실패는 Err, 생성 후 push 실패는 id가 있는 Outcome으로 반환한다. UI는 Outcome이 있으면 팝업을 닫고 error를 따로 안내한다. TextInput의 기존 단일 줄 API와 IME 테스트 계약은 유지한다.
