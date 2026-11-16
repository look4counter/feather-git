# 조사 결과

최근 저장소 행과 삭제 버튼 스타일 및 GPUI Context::on_blur API를 확인했다. on_blur는 해당 FocusHandle이 포커스를 잃을 때 호출되며 Subscription을 보관해야 한다. 공통 체크박스는 w_full과 부모 flex-col의 stretch가 전체 행 클릭의 원인이다. 설정의 focus 배경은 호출부에서 추가된다. 폴더 탐색 버튼은 세 저장소 작업이 공유한다. 미해결 기술 쟁점 없음.
