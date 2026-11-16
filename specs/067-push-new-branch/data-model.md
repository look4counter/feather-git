# 데이터 및 상태

ToolbarAction은 Push와 CreateBranch {name, checkout}이다. ToolbarRequest는 세대·프로젝트·경로·프로젝트명·작업을 가지며 Result는 실행 성공 또는 오류와 별도 재조회 결과를 가진다. BranchOutcome은 생성 여부를 나타내는 성공 값과 선택적 후속 체크아웃 오류다. BranchDialog는 입력·옵션·busy·error·포커스를 보관한다. 원격 성공 뒤 추적 설정 실패는 원격 전송 성공 사실을 명시하는 오류다.
