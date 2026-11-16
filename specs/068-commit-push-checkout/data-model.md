# 상태 및 데이터

ToolbarAction에 Checkout {reference:String}을 추가한다. 정확한 refs 접두어로 로컬/원격/태그를 구분하고 detached 의미를 전달한다. 기존 ToolbarResult의 성공/오류·재조회 형식은 유지한다. CommitDialog는 확인 버튼의 가용 상태와 실행 준비 검증을 분리한다. Push 무시는 Ok(())이며 별도 알림을 만들지 않는다.
