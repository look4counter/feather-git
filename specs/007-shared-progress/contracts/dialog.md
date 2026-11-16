# 공통 Dialog 표시 계약

Dialog::new(ID, 제목)에 width, body, footer, overlay, track_focus, on_key_down을 조합한다. 본문과 버튼은 업무 호출부가 제공하고 footer는 공통 기반이 우측에 배치한다. 배경 클릭은 하위 입력으로 전달하지 않으며 닫기 정책은 호출부 키/버튼 이벤트가 결정한다.
팝업은 타이틀바와 분리된 본문 호스트의 자식으로 넣는다. Dialog는 부모 영역만 덮으며 고정 타이틀바 높이를 계산하지 않는다. 중첩 Alert는 RepositoryDialog 다음에 렌더링한다. ProgressOverlay는 해당 카드 내부에 삽입한다.
