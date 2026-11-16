# 구현 계획

## 분석과 기술 맥락
Rust 2024·GPUI 앱이다. diff_git는 CommitDetail에 이름·이메일·시간을 담고 ViewerTarget::load가 detail_rows로 고정 높이 24px 텍스트 행을 만든다. VirtualList의 줄 인덱스가 선택·복사·스크롤에 연결되어 있어 가변 높이 카드로 교체하지 않는다. 현재 GPUI는 NullHttpClient를 사용하므로 원격 img를 넣기만 하면 사진이 나오지 않는다.

## 설계
Diff 모델에 인물 사진의 시작 행과 URL만 추가한다. 상세 변환에서 부모 행 수를 반영해 작성자·커미터의 두 줄을 매핑한다. 각 24px 행에 같은 40px 사진의 위·아래 부분을 클리핑해 한 사진이 두 행을 차지하게 한다. 원래 텍스트 인덱스는 보존하며 인물 행의 드래그·커서 가로 위치에 사진 폭 48px를 반영한다. 기본 아이콘을 바탕에 항상 그려 로딩 중과 실패 시에도 보이게 한다.

avatar.rs는 이메일 trim·소문자·SHA256으로 URL을 만든다. GPUI 전역 이미지 캐시와 img를 재사용한다. NullHttpClient 대신 사진용 HTTP 어댑터를 설치한다. 별도 작업 스레드 1개·대기열 8개·요청 제한시간 3초·응답 256KiB 제한으로 Git 작업 풀과 UI를 막지 않는다. 큐 포화나 실패는 기본 아이콘으로 귀결된다. GET과 Gravatar 호스트만 허용한다. 원본 이메일은 요청이나 로그에 기록하지 않는다.

## 의존성과 영향
변경: src/diff_model.rs, src/diff_viewer.rs, src/main.rs, Cargo.toml, Cargo.lock. 추가: src/avatar.rs. 이미 잠금파일에 있는 sha2 0.10.9, gpui_http_client 0.2.2, futures 0.3, zed-reqwest 0.12.15-zed를 직접 의존성으로 사용하고 blocking 기능을 활성화하고 기본 TLS 대신 기존 GPUI와 같은 rustls-tls-native-roots를 사용한다. 새 HTTP 라이브러리 대안으로 WinHTTP 직접 호출을 검토했으나 OS 전용 코드 중복이 커서 기존 의존성 그래프의 HTTP 구현을 재사용한다. 핵심 의존성 교체는 없다.

## 헌법과 검증
헌법은 미작성 템플릿이므로 규범 검사를 건너뛴다. UI·통신·데이터 책임을 분리한다. 해시 정규화·빈 이메일·허용 URL·응답 크기·부모별 행 매핑·두 줄 시간 형식과 기존 선택 테스트를 검증한다. fmt/check/test/clippy/build를 실행한다. 실제 사용자 이메일로 네트워크 테스트하지 않으며 GUI와 실서비스 미검증은 명시한다.
