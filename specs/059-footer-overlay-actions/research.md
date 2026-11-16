# 조사 결과

결정: 공용 Dialog를 그대로 사용한다. 현재 ContextMenu는 마우스 위치 메뉴이며 요청한 중앙 오버레이가 아니다. RepositoryDialog의 엔티티 이벤트/포커스 복구 패턴을 사용하면 모달 책임이 분리된다.

결정: 탐색기는 기존 GPUI open_with_system을 사용한다. 별도 explorer 셸 명령문을 만들 필요가 없다.

결정: Windows PowerShell에 고정 인수를 주고 검증된 폴더를 current_dir로 전달한다. 경로가 포함된 명령 문자열이나 cmd /c start는 사용하지 않는다. CREATE_NEW_CONSOLE은 사용자가 요청한 대화형 터미널 표시를 위한 것이다.

검토 대안: Windows Terminal 의존성은 설치 여부와 별도 fallback이 필요하므로 도입하지 않는다. 기존 공용 Dialog를 확장하거나 팝업 프레임을 새로 구현할 필요가 없다. 미해결 조사 항목 없음.

## 추가 배지 조사

커밋 배지는 PreparedHistoryRow.references의 캐시된 순서대로 렌더링한다. 표시용 준비 시 안정 정렬하면 초기/증분/파일 히스토리 모두 일관되고 렌더 정렬 비용이 없다. 저장소의 현재 행 판정은 이미 로컬 탭의 project.branch 이름 일치를 사용하므로 판정 로직은 유지하고 색·배치만 바꾼다.

## 그래프 좌표 조사

100% 배율·레인 중심 10·일반 지름 9에서 현재 왼쪽 경계 round(10-4.5)=6이므로 원 중심은 10.5다. 선 중심은 10이다. 원 크기 변경 없이 같은 중심 보정을 적용하고 원 경계를 반올림하지 않는 최소 수정을 선택한다. 지름의 짝수 보정 대안은 원/현재 흰색 내부 크기를 바꾸므로 제외한다.

## 후속 조사: 경계 꺾임

Raw Segment의 y는 상단 절반 0→12, 하단 절반 12→24로 구분된다. 꺾임 점은 로컬 좌표에서 만들고 기존 snap_graph_coordinate를 모든 점에 적용하면 소수 DPI에서도 행 경계에서 같은 x의 수직 접속이 유지된다. 선을 별도 두 경로로 나누는 대안은 꺾임에 틈이 생길 수 있어 한 polyline 경로를 선택한다. 버튼은 새 치수 상수 대신 기존 badge_base를 재사용한다.

## Git Bash 실행 조사

[Git for Windows 공식 포터블 안내](https://github.com/git-for-windows/build-extra/blob/main/portable/root/README.portable)는 기본 런처가 시작된 작업 디렉터리를 유지한다고 설명한다. 따라서 경로를 --command나 셸 문자열로 만들 필요 없이 current_dir만 전달한다. 로컬 Git 위치는 C:/Program Files/Git/cmd/git.exe이며 같은 설치 루트에 git-bash.exe가 있음을 확인했다. PATH 사용자 지정 설치와 표준 시스템/사용자 설치를 유한 후보로 탐색하며 새 라이브러리나 레지스트리 접근은 추가하지 않는다.

그래프의 기존 현재 노드 외곽은 150%에서 14px이므로 최소 간격 4px를 유지하려면 레인 간격은 18px 이상이어야 한다. 19→18px을 선택한다. 헤더는 현재 두 사용처가 공용 함수를 쓰므로 높이 상수를 공통 ROW_HEIGHT로 연결한다.

참조 이동 조사: backend ReferenceRequest가 반환한 Project에는 대상까지 읽은 새 커서가 이미 포함된다. 커서를 새로 만들거나 오류를 숨기는 대신 성공 결과 전체의 조회 상태를 함께 반영하는 최소 수정으로 해결한다. 이전 커서 결과는 기존 same_session/offset 검사로 격리할 수 있다.
