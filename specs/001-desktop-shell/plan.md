# 구현 계획: 데스크톱 기본 화면

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

## 요약

Rust 2024와 기존 GPUI 의존성을 유지하여 네이티브 화면을 새로 만든다. 소스와 기존 호출 관계는 없으며 Cargo.toml만 존재한다. Git 데이터 접근과 UI를 연결하지 않는다.

## 기술적 맥락

- 대상: Windows 데스크톱, 기본 1440×900, 최소 1100×700.
- 의존성: 기존 gpui 0.2.2. 로컬 캐시의 공식 예제를 확인했다.
- 제약: vendor/gpui 및 vendor/proc-macro-error2가 없어 전체 Cargo 빌드가 차단될 수 있다. 설정은 임의 교체하지 않는다.
- 상태 흐름: 샘플 데이터 → 화면 렌더링 → 클릭 → 선택 인덱스 갱신 → 상세 렌더링.
- 범위: src/main.rs 진입점, src/ui.rs 화면과 색상, src/sample.rs 예시 데이터.
- 검증: cargo fmt, cargo check, cargo test 시도. 의존성 없는 예시 데이터 테스트는 rustc로 별도 실행한다.

## 헌장 확인

헌장은 미작성 템플릿이다. 기존 의존성 유지, UI와 데이터 분리, Git 작업 미실행을 이번 화면의 설계 제약으로 적용한다. 설계 전후 충돌 없음.

## 구조와 영향 범위

기존 소스가 없으므로 공개 API나 기존 동작에 영향이 없다. 모듈 방향은 main → ui → sample이다. 테마 상수와 재사용 가능한 패널/라벨 도우미를 UI 내부에 둔다.

## 구현 단계

1. 예시 데이터와 선택 검증을 작성한다.
2. GPUI 진입점과 4분할 화면을 작성한다.
3. 프로젝트/커밋 선택과 미연결 안내를 연결한다.
4. 포맷, 데이터 테스트, 전체 빌드 시도 및 수렴 검토를 수행한다.

## 실행 오류 수정 계획

- 현재 동작 분석: main의 cx.new 호출은 AppContext 트레이트가 범위에 없어 실패한다. 본문 row는 items_center를 적용한 뒤 존재하지 않는 items_stretch를 호출하여 실패한다.
- 수정: main에 AppContext를 가져오고 본문의 컨테이너만 div().flex()로 변경하여 기본 stretch 정렬을 사용한다. 하위 행과 데이터 흐름은 유지한다.
- 영향 범위: src/main.rs와 src/ui.rs의 진입점 및 본문 정렬. 공개 API, 데이터, 의존성을 변경하지 않는다.
- 검증: cargo fmt --check, cargo check --offline, cargo test --offline, 실행 후 창 생성 확인.
- 기존 vendor 부재 제약은 현재 Cargo.toml에서 해소되어 해당 검증 작업을 현재 환경 기준으로 진행한다.

## 프로젝트 바 개선 계획

- 분석: tabs()는 PROJECTS 전체를 정적으로 순회하고 project 인덱스를 상세에서 사용한다. 닫기 후 이름 인덱스가 이동하면 상세와 선택이 어긋나므로 원본 프로젝트 인덱스의 열린 목록을 별도로 유지한다.
- src/ui.rs에 열린 프로젝트 목록과 닫기 처리를 추가한다. 선택은 원본 인덱스를 유지하고 선택 탭을 닫은 경우에만 이웃으로 이동한다.
- 탭 로고를 제거하고 border_t_2, 닫기 자식 버튼, 스크롤 탭 영역, 오른쪽 고정 + 버튼을 적용한다. 자식 닫기 이벤트는 stop_propagation으로 부모 선택을 막는다.
- 탭이 없으면 기존 본문 대신 빈 상태를 표시한다. +는 미연결 안내만 제공한다.
- 영향: src/ui.rs와 프로젝트 바 계약 및 검증 문서. 데이터/서비스/의존성은 변경하지 않는다.
- 검증: 선택/비선택/마지막 탭 닫기 단위 테스트, cargo fmt --check, cargo check --offline, cargo test --offline, cargo build --offline. 실제 클릭 검증은 별도 명시한다.
- 헌장은 미작성 템플릿이다. 기존 모듈 책임과 의존성 방향에 변화가 없다.

## Windows 커스텀 제목 표시줄 계획

- 분석: main.rs의 TitlebarOptions는 기본 Windows 제목 표시줄을 사용한다. ui.rs의 tabs() 배경은 BASE이다. GPUI 0.2.2의 appears_transparent는 Windows 기본 제목 표시줄 숨김을 지원한다.
- 공식 로컬 GPUI 소스의 WindowControlArea는 Drag/Min/Max/Close를 Windows 비클라이언트 영역으로 매핑하므로 직접 Win32 의존성이나 별도 창 처리 계층을 추가할 필요가 없다.
- main.rs에서 Windows에서만 appears_transparent를 켠다. ui.rs에 Windows 전용 32px 제목 행을 추가하고 BASE를 공유한다.
- 제목 영역은 Drag, 제어 버튼은 Min/Max/Close로 지정해 네이티브 드래그, 더블클릭, 시스템 창 제어를 사용한다. 최대화 상태에 따라 복원 기호를 표시한다.
- 검증: 포맷, 컴파일 검사, 기존 테스트, 실행 파일 빌드. 실제 Windows 조작 검증 여부를 별도 기록한다. 현재 실행 중인 앱 파일이 잠긴 경우 별도 파일명으로 빌드한다.
- 영향은 src/main.rs, src/ui.rs 및 화면 계약에 한정한다. 데이터 흐름과 의존성을 유지하고 미작성 헌장은 검사 생략한다.

## 제목 버튼 및 프로젝트 바 밀도 개선 계획

- 분석: titlebar()는 Segoe UI Symbol의 일반 문자를 16px로 사용하여 자형/기준선이 일정하지 않다. 버튼은 이미 46px 너비와 Windows 네이티브 제어 영역을 사용하므로 이를 재사용한다.
- src/ui.rs에서 Windows 10/11에서 제공하는 Segoe MDL2 Assets 글꼴의 창 제어 코드포인트를 사용한다. 아이콘은 10px 및 명시적 줄 높이로 중앙 정렬하며, 닫기 호버는 기존 Windows 빨강을 유지한다.
- 프로젝트 바 높이는 36px, 닫기 영역은 20px, + 영역 너비는 40px로 줄인다. 제목 행은 32px로 유지한다.
- 영향 범위: src/ui.rs의 스타일/아이콘만 변경한다. 선택 상태, 데이터, 의존성, 네이티브 제어 영역은 변경하지 않는다.
- 검증: cargo fmt --check, cargo check --offline, 기존 cargo test --offline, cargo build --offline. 시각 검증은 실제 실행 여부와 구분하여 기록한다.
- 헌장은 미작성 템플릿이며 기존 책임과 의존성 방향에 변화가 없다.
- 추가 지시 반영: 프로젝트 바 컨테이너 위쪽에 LINE 색상의 1px 테두리를 추가한다. 제목 표시줄과의 경계만 추가하며 다른 레이아웃은 유지한다.


## 프로젝트 탭바 정렬 보정 계획

- 분석: 모든 탭에 border_t_2가 적용되어 비선택 상태에서도 BASE 테두리가 2px 공간을 차지한다. +는 flex 중앙 정렬이지만 텍스트 글꼴의 기준선 때문에 기호 자체가 시각 중심과 다를 수 있다.
- src/ui.rs의 tabs() 높이를 42px로 변경한다. 탭 테두리는 제거하고 relative 탭의 선택 상태에만 absolute 2px 강조선을 겹쳐 배치한다. 내용 높이에 영향을 주지 않는다.
- +는 텍스트 대신 12×12px 컨테이너 안에 가로/세로 1px 선을 교차시켜 그리고 버튼에서 중앙 정렬한다. 글꼴 기준선 의존성을 제거한다.
- 영향: tabs() 스타일만 변경한다. 기존 선택/닫기/추가 안내 및 모듈 의존성은 유지한다.
- 검증: 포맷, 컴파일 검사, 기존 테스트, 별도 파일명 빌드. 실제 화면 정렬 검증 여부는 별도 기록한다.
- 기존 36px 계약은 이번 42px 계약으로 대체한다. 미작성 헌장이며 구조 변경은 없다.

## 프로젝트 탭 닫기 버튼 호버 표시 계획

- 분석: Workspace::tabs()가 열린 프로젝트를 순회하고 닫기 버튼을 항상 렌더링한다. 닫기는 close_project()로 전달되며 stop_propagation으로 부모 선택을 막는다.
- GPUI 0.2.2 로컬 소스의 group/group_hover를 재사용한다. 탭에 project-tab 그룹을 지정하고 닫기 버튼에 invisible과 group_hover의 visible을 적용한다. 그룹은 각 탭의 자식 범위에 적용된다.
- 숨김은 레이아웃 공간을 유지한다. 상태 필드, 데이터, 의존성, 공개 API 및 기존 이벤트 처리를 변경하지 않는다. main → ui → sample 의존성 방향을 유지한다.
- 영향: src/ui.rs의 탭/닫기 스타일과 기존 명세·검증 문서. 헌장은 미작성 템플릿이므로 검사 생략하며 기존 설계와 충돌하지 않는다.
- 검증: cargo fmt --check, cargo check --offline, cargo test --offline, 별도 이름으로 cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-hover.exe. 실제 마우스 조작은 별도 확인 대상으로 기록한다.

## 프로젝트 탭 경계와 활성 닫기 표시 계획

- 모든 탭 행의 border_r_1·LINE 경계를 유지한다. 닫기는 비활성 탭에만 invisible을 적용하고 부모 그룹 호버에서 visible로 표시한다.
- 기존 close_project, 선택 처리, stop_propagation과 숨김 전후 크기를 유지한다. 영향 파일은 src/ui.rs이다.
- 이 내용은 현재 소스와 T032에서 확인하여 복원했다. 과거 실행 결과는 재생성하지 않는다. 이전 샘플 의존성은 020에서 제거되었다.

## 패널 헤더 높이 축소 계획

- 분석: src/ui.rs의 heading()이 48px 높이를 지정하고 row()의 items_center로 중앙 정렬한다. 저장소, 변경 파일, 커밋 기록, 커밋 상세의 네 호출부가 같은 함수를 사용한다.
- heading()의 높이 한 곳만 36px로 변경한다. flex_shrink_0, 배지와 제목 스타일, 아래 경계선 및 호출부를 유지한다. 추가 상태, 의존성 및 데이터 모델 변경은 없다.
- 영향 범위는 공통 패널 헤더 스타일이다. 기존 main → ui → sample 의존성 방향과 UI/데이터 책임을 유지한다. 헌장은 미작성 템플릿으로 검사 생략하며 구조 충돌은 없다.
- 검증: cargo fmt --check, cargo check --offline, cargo test --offline, 별도 파일명 실행 파일 빌드. 실제 창의 잘림 및 정렬 여부는 별도 확인 대상으로 기록한다.
