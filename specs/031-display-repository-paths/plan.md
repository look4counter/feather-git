# 구현 계획: 표시용 저장소 경로

## 기존 구조와 호출·데이터 흐름

src/session.rs는 canonicalize한 PathBuf를 최근 목록과 세션에 보관하고 ui.rs의 entry_request는 이 원본을 저장소 열기에 사용한다. 화면에서는 최근 카드와 update_tab_notice가 path.display를 직접 사용한다. diff_viewer.rs의 파일 변경 실패 안내도 저장소 루트를 직접 표시한다. 폴더 선택 입력과 백엔드 Request의 to_string_lossy는 실제 작업 입력이므로 그대로 유지해야 한다.

## 설계 및 영향 범위

- src/path_display.rs: std::path::Path를 받아 별도 String을 반환하는 순수 표시 함수를 추가한다. 확장 드라이브의 ASCII 문자·콜론·역슬래시를 확인하여 접두사를 제거하고, 확장 UNC는 일반 UNC의 두 역슬래시를 복원한다. UNC 표기는 대소문자 차이를 허용한다. 지원하지 않는 장치/불완전한 형태와 다른 경로는 원문을 유지한다. 비 UTF-8 표현은 기존 display와 같은 손실 변환 정책을 사용한다.
- src/main.rs: 공통 모듈을 등록한다.
- src/ui.rs: 최근 목록 라벨과 현재 저장소 안내에서 공통 함수를 사용한다. 원본 PathBuf, entry_request, session_snapshot, 폴더 선택 입력은 변경하지 않는다.
- src/diff_viewer.rs: 앱이 직접 포맷하는 파일 변경 실패 안내의 저장소 경로를 같은 함수로 표시한다. 라이브러리 오류 문자열 전체는 임의 치환하지 않는다.

## 아키텍처·헌법 점검

헌법은 미작성 템플릿으로 규범 검사를 생략한다. AGENTS.md의 계층 책임·최소 변경·기존 데이터/공개 API 호환성을 유지한다. 화면 포맷팅 모듈만 추가하고 데이터 계층·저장 형식·백엔드 경계는 바꾸지 않는다. 새 외부 의존성은 없다.

## 검증

먼저 기존 display 동작을 그대로 반환하는 함수에 기대 표시 테스트를 연결해 드라이브/UNC 실패를 확인한다. 이어 정상 드라이브·UNC 대소문자·공백/한국어·일반/상대/비 Windows·불완전/장치 경로와 원본 PathBuf 보존을 검증한다. UI 상태 테스트에서 화면 안내는 정리되고 entry_request와 session_snapshot은 원본 경로를 유지하는지 확인한다. 전체 test/fmt/check/clippy/build를 실행하고 GUI 직접 검증과 실행 파일 교체 제한을 구분한다.
