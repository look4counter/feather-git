# 조사 결과

old/src/ui.rs의 history_panel은 uniform_list와 프로젝트별 scroll/graph 캐시를 사용한다. 현재 앱은 일반 children으로 전 행을 만들고 탭 선택마다 그래프를 다시 계산한다. 현재 고정 45px 열과 graph_x 압축은 4개 레인에서 중심 간격을 약 9.67px로 낮춰 13px 체크아웃 원과 겹칠 수 있다.
GPUI uniform_list는 높이 측정용 한 행도 요청할 수 있으므로 매 콜백마다 캐시를 range로 축소하면 불필요한 재생성이 생긴다. 현재 제한된 200개 안에서 준비한 행은 보관한다. 그래프 토폴로지와 표시 캐시는 분리한다.

## 추가 조사: GPU 경로 배치

결정: 수직선은 직접 사각형 렌더링을 사용한다. 근거: Windows DirectX 경로 배치마다 전체 크기 중간 텍스처 지우기와 MSAA resolve를 수행한다. 수직선은 정렬된 사각형으로 완전히 표현 가능하다. 곡선을 분절 사각형으로 근사하는 대안은 시각 회귀 위험으로 제외한다. 원과 곡선, 프레임워크 및 의존성은 유지한다.

## old 실제 페인트 순서 재확인

old history_view.rs는 선뿐 아니라 원도 PathBuilder::fill로 네 개 베지어를 구성한다. 현재의 rounded quad 원 및 직접 quad 수직선은 그 방식과 다르다. 단순 quad가 빠를 것이라는 가정 대신 old와 같은 경로 생성과 순서를 적용한다. DirectX 렌더러는 동일하므로 변경하지 않는다. old scroll_frame은 제한된 flex 뷰포트와 overflow_hidden을 사용한다. 실제 GPU 효과는 미측정이다.

## 진단 범위

기존 검사와 CPU 준비 벤치마크는 실제 GPUI 프레임을 검증하지 못했다. vendor/gpui Windows draw와 Workspace render를 임시 계측하여 원인을 구분한다. 자동 테스트 창은 입력을 조작하지 않고 반복 렌더만 한다.

결정: 실제 측정에서 CPU 준비가 대부분이므로 GPUI와 taffy의 개발 프로필 최적화를 적용한다. 전체 release 실행을 강제하는 대안은 사용자가 cargo run으로 개발하는 흐름을 바꾸므로 사용하지 않는다. 전체 의존성 최적화는 빌드 영향이 크므로 우선 두 모듈로 제한한다.

window.rs의 cfg(any(feature=inspector, debug_assertions))와 관련 요소 추적은 release와 dev 렌더 경로 차이다. opt-level만으로 해당 경로가 사라지지 않으므로 GPUI에 한하여 debug-assertions=false를 적용한다.
