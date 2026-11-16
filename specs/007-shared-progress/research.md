# 조사 결과

진행 표시는 별도 팝업 Entity가 아닌 폼 내부 absolute 오버레이이다. 공통 표시를 RenderOnce 요소로 분리하면 호출부의 영역과 상태를 그대로 유지할 수 있다. bool 두 개보다 세 가지 활동 상태를 사용해 탐색 중 잠금을 유지하면서 표시를 구분한다. 기존 Alert 및 RepositoryDialog는 합치지 않는다.

## Dialog 호스트 조사

두 팝업의 배경·카드·타이틀·footer 구현이 중복된다. 상위 루트에 타이틀바와 함께 팝업을 넣어 전체 창을 덮고 있으므로, 공통 껍데기 추출과 함께 팝업 호스트를 타이틀바 아래 본문으로 옮긴다. Dialog는 호스트 전체를 채우므로 다른 화면에서도 같은 API를 재사용한다.

## 진행 애니메이션 조사

GPUI examples/animation.rs와 elements/animation.rs를 확인했다. Animation::new(...).repeat()와 AnimationExt::with_animation으로 프레임별 값이 0~1로 전달되고 요소가 사라지면 프레임 요청도 사라진다. canvas와 PathBuilder의 곡선/타원 호를 사용해 외부 이미지 없이 장면을 그린다.
