# 조사

src/ui.rs의 Workspace::branches는 Project.branches 이후 원격 저장소 제목과 Project.remotes를 무조건 추가한다. src/repository.rs는 로컬과 원격 참조를 이미 분리한다. 새로운 필터나 데이터 변환 없이 잘못된 렌더 항목을 제거하면 해결된다.
