# 작업 목록: 깃털 아이콘

## 1단계: 준비

- [x] T001 specs/006-feather-icon/spec.md와 checklists/requirements.md의 요구사항 및 명확화 결과를 확인한다.

## 2단계: 기반

- [x] T002 specs/006-feather-icon/contracts/icon.md에 따라 자산 경로와 좌표계를 확정한다.

## 3단계: 사용자 이야기 1

- [x] T003 [US1] assets/branding/feather-git-icon.svg에 단색 깃털과 투명 중심 여백을 구현한다. FR-001~004.
- [x] T004 [US1] 밝고 어두운 배경의 24, 32, 128픽셀 미리보기를 만들고 확인했다. SC-001~003. 검증용 파일은 사용자 요청에 따라 삭제했다.

## 4단계: 마무리

- [x] T005 specs/006-feather-icon/quickstart.md의 사용 계약과 SVG 구조를 대조하고 완료 상태를 기록한다.

## 의존성과 실행 전략

T001 → T002 → T003 → T004 → T005 순서로 진행한다.
단일 사용자 이야기이므로 전체가 최소 납품 범위다.
병렬 실행 예시는 T003 완료 후 T004의 렌더링과 T005의 문서 대조를 각각 진행하는 것이다.


