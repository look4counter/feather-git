# 구현 계획: Git 디렉터리 직접 지정

**작성일**: 2026-11-17  
**명세**: [기능 명세](spec.md)

## 요약

폴더 선택은 UI에서 기본 경로만 구성하고 최종 경로 검증과 생성·복제·열기는 저장소 계층에서 처리한다. Project.path 및 기존 세션 path는 Git 디렉터리를 식별한다. 파일 작업은 별도의 workdir을 사용한다.

## 기술적 맥락

Rust 2024, GPUI 0.2.2, git2 0.21.0, gix 0.90.0 기반 Windows 데스크톱 앱이다. 신규 의존성 없이 기존 JSON 버전과 필드를 유지한다. cargo test, cargo check, cargo clippy로 검증한다. UI 스레드에 파일시스템 조회를 추가하지 않는다.

## 헌장 점검

constitution.md는 미작성 템플릿이므로 이를 변경하지 않고 기존 구조와 AGENTS.md를 따른다. UI·저장소·세션 책임과 의존성 방향을 유지하며 설계 전후 충돌이 없다.

## 기존 구조 및 영향 분석

- repository_dialog.rs: 경로 변경 구독은 마지막 이름을 자동 채운다. 작업별 이름 계산으로 변경한다.
- ui.rs: browse_repository_path는 선택 경로를 그대로 입력한다. 생성·복제만 .git을 추가한다. 파일 경로 복사는 미리 조회한 Project.workdir을 사용한다.
- repository.rs: run은 작업 폴더를 초기화·복제하고 snapshot은 workdir을 path로 저장한다. 이를 직접 Git 디렉터리로 바꾼다.
- 초기화 workdir_path는 .git 연결 파일을 생성할 수 있다. 사용자 지정 경로는 bare 생성 후 set_workdir(false) 및 core.bare=false로 연결하여 기존 .git을 보존한다.
- RepoBuilder는 저장소 생성 콜백을 노출하지 않는다. bare 복제 후 원격 추적 참조·기본 브랜치·upstream을 일반 복제 형태로 만들고 부모에 안전하게 체크아웃한다.
- diff_git과 변경 작업은 대부분 repo.workdir을 이미 사용한다. 사용자 지정 메타데이터가 상태 조회나 스테이지 대상이 되지 않도록 검증한다.
- session.rs: 경로 정규화 시 실제 Git 디렉터리로 변환하여 open/active/recent에 적용한다. 누락된 경로는 유지한다.
- workspace_backend.rs 및 file_history.rs: 파일 열기·디렉터리 열기·이력 내보내기는 workdir을 사용한다.
- 저장소 삭제는 기본 저장소의 기존 동작을 유지하되 공유 작업 폴더나 사용자 지정 메타데이터를 전체 삭제 대상으로 해석하지 않도록 보호한다.

## 프로젝트 구조

문서는 specs/081-explicit-git-directory/ 아래에 둔다. 구현 대상은 src/repository.rs, src/repository_dialog.rs, src/ui.rs, src/session.rs, src/workspace_backend.rs, src/file_history.rs, src/file_history_dialog.rs, src/backend.rs, src/partial_stage.rs이다. backend.rs의 기존 I/O 등록을 유지하고 부분 스테이지의 메타데이터 보호도 같은 규칙을 적용한다. 필요 시 src/diff_git.rs의 메타데이터 보호를 보완한다. 테스트는 기존 각 모듈 테스트에 추가한다.

## 단계 및 검증

1. 경로·이름·생성 테스트와 구현을 수행한다.
2. 기본·사용자 지정·빈 소스·다중 브랜치 복제 및 기존 파일 보존을 검증한다.
3. 경로 식별·파일 기능·이전 세션 변환과 다중 저장소 복원을 검증한다.
4. 관련 및 전체 테스트, 빌드·정적 분석 후 수렴 검사한다.

## 복잡성 추적

새 의존성이나 주요 아키텍처 변경 없이 단일 경로에 섞인 저장소 식별과 작업 폴더 의미를 분리한다.

## 추가 계획: Windows 숨김 속성

- 기존 run은 생성·복제 완료 후 공통으로 snapshot을 호출한다. 이 경계에서 Open을 제외하고 숨김 속성을 적용한다.
- src/repository.rs에 플랫폼별 작은 함수를 두고 기존 windows 의존성의 GetFileAttributesW 및 SetFileAttributesW를 사용한다. UTF-16 경로를 전달하며 기존 속성에 FILE_ATTRIBUTE_HIDDEN만 추가한다.
- 기존 속성이 이미 숨김이면 쓰기를 생략한다. 다른 플랫폼에서는 성공으로 반환하여 동작을 유지한다.
- 오류는 만든 Git 디렉터리가 남아 있다는 문맥을 포함한다. 자동 삭제나 재시도는 도입하지 않는다.
- 검증: 기본·사용자 지정 이름의 Create·Clone, 부모 및 작업 파일 비숨김, 기존 속성 보존·멱등성, 누락 경로 오류, Open 시 속성 보존을 Windows 테스트로 확인한다. 관련 저장소·대화상자·세션 테스트와 cargo check·clippy를 실행한다.
- 헌장은 미작성 템플릿이며 기존 계층과 의존성을 유지하므로 충돌이 없다.
