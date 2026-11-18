# 경량 범용 Source Parser 설계 기준서 v0.1

## 문서 메타데이터

- **Title:** Lightweight General-Purpose Source Parser Design Baseline
- **Document ID:** `SOURCE-PARSER-DESIGN-BASELINE`
- **Subject:** 구조 보존·위치 추적·선택적 지능 복구 기반 범용 문서 Source Parser
- **Description:** DOCX 및 digital PDF를 가볍게 deterministic하게 추출하고, 구조 이상이 확인된 영역에만 지능 모델을 적용하여 Canonical Source와 Stable Anchor를 생성하는 파서 설계 기준
- **Scope:** DOCX, digital PDF, 구조 추출, 무결성 판정, targeted recovery, Canonical Source, Stable Anchor
- **Out of Scope:** 문서 의미판단, 논증 검수, 도메인 추론, 법률·의학 등 개별 도메인 extension 구현
- **Version:** 0.1
- **Schema Version:** 0.1
- **Status:** draft_baseline
- **Owner:** Project Owner
- **Created:** 2026-11-18
- **Updated:** 2026-11-18

## 0. 정책

### change_policy

**allowed_changes**
- 구현 라이브러리 교체
- 내부 parser adapter 변경
- 무결성 검사 규칙 보강
- recovery provider 교체
- 성능 최적화

**forbidden_changes**
- Parser가 의미판단 책임을 갖도록 확장
- extraction 실패를 내용 부재로 변환
- 모델 출력을 검증 없이 Canonical Source로 승격
- Source 위치 결속 제거
- 특정 parser library를 제품 의미 계약으로 고정

**approval_required_for**
- 지원 입력 형식 확대
- Canonical Source 의미 변경
- Stable Anchor identity 변경
- Core와 Domain Extension의 책임 경계 변경

### compatibility_policy

- Canonical Source와 Stable Anchor의 의미 호환성을 우선한다.
- Parser 또는 recovery engine 교체로 결과 표현이 달라질 경우 새로운 parser revision으로 취급할 수 있어야 한다.
- 외부 라이브러리 내부 ID를 Source identity로 사용하지 않는다.

### validation_policy

필수 검증 대상:

- 입력 형식 식별
- 구조 보존
- 위치 결속
- table 구조
- reading order
- integrity failure 표현
- targeted recovery 범위
- recovery 후 재검증
- deterministic path에서 모델 미호출

# 1. 목적

이 Parser의 목적은 문서를 이해하거나 해석하는 것이 아니다.

목적은 다음이다.

> 다양한 문서 입력을 후속 시스템이 안정적으로 조회·재조회·인용·검증할 수 있는 읽기 전용 Source 표현으로 변환한다.

Parser는 최소한 다음을 가능하게 해야 한다.

- 원문 구조 보존
- 원문 위치 재확인
- 부분 Source 재조회
- 표·각주·캡션 등 관계 보존
- 입력 형식과 무관한 공통 Source 표현
- 후속 의미 객체의 원문 결속

# 2. 핵심 설계 기조

## 2.1 Deterministic First, Intelligence on Failure

기본 경로는 deterministic processing이다.

```text
Input
→ Lightweight Extraction
→ Structure Integrity Check
→ PASS
→ Canonicalization
```

지능 모델은 기본 parser가 아니다.

다음 조건이 확인된 영역에만 사용한다.

```text
FAIL
or
AMBIGUOUS
→ Targeted Intelligence Recovery
```

전체 문서를 관성적으로 모델에 전달하지 않는다.

## 2.2 가벼움과 품질의 분리

가벼움은 구조 정보를 버리는 방식으로 확보하지 않는다.

```text
가벼움
= 불필요한 작업을 하지 않는 것

품질
= 필요한 Source 구조와 위치를 정확하게 보존하는 것
```

정상적으로 deterministic extraction이 가능한 영역에는 추가 모델 호출·OCR·고급 layout inference를 수행하지 않는다.

## 2.3 Parser와 Meaning Layer 분리

Parser는 다음을 판단하지 않는다.

- 이 문장이 Claim인지
- 이 표가 주장에 충분한 근거인지
- 수치가 과학적으로 타당한지
- 법률 조항이 어떤 의미인지
- 검사값이 임상적으로 중요한지

Parser는 Source를 보존한다.

Meaning Layer가 그 Source를 해석한다.

# 3. 입력 범위

## 3.1 MVP 지원 입력

### DOCX

기본 deterministic 구조 추출 대상으로 한다.

우선 보존 대상:

- section
- heading
- paragraph
- list
- table
- row
- cell
- merged cell 관계
- caption
- footnote
- document order
- source location

### Digital PDF

기본적으로 다음을 추출한다.

- text
- bounding region / position
- page
- font/layout metadata 중 구조 판정에 필요한 최소 정보
- image/figure region
- table candidate region

PDF 자체에서 의미 구조를 완전히 복원할 수 있다고 가정하지 않는다.

## 3.2 보류 입력

다음은 MVP 지원 깊이를 별도로 결정한다.

- scanned PDF
- image-only document
- handwriting
- 복잡한 mathematical document
- figure-heavy scientific document

# 4. 공통 출력 모델

입력 형식과 관계없이 최종 Source 표현은 동일한 논리 모델로 정규화한다.

```text
Input Document
→ Format Adapter
→ Extracted Structure
→ Integrity Check
→ Optional Recovery
→ Canonical Source
→ Stable Anchor
```

# 5. Canonical Source

Canonical Source는 후속 시스템이 사용하는 읽기 전용 Source 표현이다.

최소 계층:

```text
Document
├─ Section
├─ Paragraph
├─ List
├─ Table
│  ├─ Row
│  └─ Cell
├─ Caption
├─ Footnote
├─ Figure Region
└─ Equation Region
```

모든 node는 가능한 경우 다음 정보를 가진다.

```text
source_node_id
node_type
content
parent_id
document_order
source_location
extraction_status
```

## 5.1 Source identity

Source identity는 외부 parser의 임시 ID에 의존하지 않는다.

최소 개념:

```text
document_id
document_revision_id
source_node_id
```

Parser 변경으로 Source 표현이 달라질 경우 revision을 분리할 수 있어야 한다.

# 6. Stable Anchor

Stable Anchor의 목적은 후속 의미 객체를 실제 원문에 다시 결속하는 것이다.

최소 개념:

```text
anchor_id
document_revision_id
source_node_id
local_start
local_end
quote
quote_hash
source_location
```

구조형 Source에는 추가 위치를 둘 수 있다.

예:

```text
page
bounding_region
table_id
row_index
column_index
figure_id
footnote_id
```

## 6.1 Anchor 불변

다음을 Anchor로 사용하지 않는다.

- retrieval chunk ID
- 모델 출력 순번
- 외부 parser의 임시 session ID
- 현재 실행에만 존재하는 transient object ID

# 7. Deterministic Extraction

## 7.1 DOCX

DOCX는 내부 구조가 명시되어 있으므로 가능한 한 직접 구조를 보존한다.

우선순위:

```text
XML structure
→ normalized internal representation
→ Canonical Source
```

저수준 XML parsing 자체는 검증된 라이브러리를 사용할 수 있다.

라이브러리는 교체 가능한 adapter다.

## 7.2 Digital PDF

PDF에서는 우선 다음만 확보한다.

```text
text
+
position
+
page
+
layout primitives
```

이 정보로 구조 후보를 생성한다.

PDF의 시각적 배치가 곧 논리 구조라고 가정하지 않는다.

# 8. Structure Integrity Check

Integrity Check는 extraction 결과가 Canonical Source로 승격 가능한지 판정한다.

기본 상태:

```text
VALID
AMBIGUOUS
PARTIAL
FAILED
```

## 8.1 대표 이상 신호

### Reading order

- 텍스트 순서가 공간적 배치와 반복 충돌
- 다단 레이아웃이 단일 열로 잘못 결합
- header/footer가 본문 사이에 삽입
- paragraph 순서가 비정상적으로 교차

### Table

- 행/열 alignment 불일치
- 셀 영역 중첩
- header와 data 관계 불명
- 병합 셀 관계 손실
- 표 내부 텍스트가 본문으로 분산
- 표 candidate인데 구조화 셀이 생성되지 않음

### Caption / Footnote

- caption marker 존재 + 대상 region 없음
- footnote marker 존재 + 대응 footnote 없음
- 표/그림 번호와 대상 구조 불일치

### Text

- 비정상적 fragmentation
- 과도한 duplication
- 누락 의심
- source position 불일치

### Equation

- 문자 순서 붕괴
- 첨자/위첨자 관계 손실
- symbol ordering 비정상

# 9. Targeted Intelligence Recovery

Integrity Check가 `VALID`이면 모델을 호출하지 않는다.

다음에만 recovery를 허용한다.

```text
AMBIGUOUS
PARTIAL
FAILED
```

그리고 가능하면 전체 문서가 아니라 문제 영역만 전달한다.

## 9.1 Recovery 역할

지능 모델은 다음을 보조할 수 있다.

- 복잡한 table relation 복구
- 병합/다단 header 복원
- reading order 복구
- caption ↔ figure/table 관계
- equation structure 복구
- OCR 결과 보정

## 9.2 Recovery 금지

지능 모델은 다음을 해서는 안 된다.

- 원문에 없는 셀 생성
- 보이지 않는 값을 추정
- 의미상 그럴듯하다는 이유로 구조 창작
- 누락된 문장을 생성
- 불확실한 값을 확정값으로 승격

# 10. Recovery 재검증

모델 출력은 바로 Canonical Source가 아니다.

```text
Recovery Result
→ Source Rebinding
→ Structural Validation
→ Consistency Check
→ Canonical Source
```

재검증할 수 없는 결과는 unresolved 상태로 남긴다.

예:

```text
RECOVERY_VERIFIED
RECOVERY_PARTIAL
SOURCE_UNRESOLVED
```

# 11. Source Binding

모든 material 구조는 가능한 한 원본 Source 위치로 다시 확인 가능해야 한다.

```text
Canonical Node
↕
Stable Anchor
↕
Original Document Region
```

후속 시스템은 parser가 생성한 설명만 신뢰해서는 안 된다.

필요한 경우 항상 원본 Source로 돌아갈 수 있어야 한다.

# 12. 실패 상태

최소 실패 유형:

```text
UNSUPPORTED_FORMAT
EXTRACTION_FAILED
EXTRACTION_PARTIAL
STRUCTURE_AMBIGUOUS
UNREADABLE_REGION
RECOVERY_FAILED
SOURCE_UNRESOLVED
```

## 12.1 핵심 불변

```text
Extraction Failure
≠
Content Absence
```

```text
Retrieval Failure
≠
Content Absence
```

```text
Recovery Failure
≠
Content Absence
```

기술적 실패를 의미적 부재로 변환하지 않는다.

# 13. Core와 Domain Extension

Core는 여러 도메인에서 공통으로 사용하는 구조만 담당한다.

## Core

```text
section
paragraph
list
table
cell
caption
footnote
figure region
equation region
source location
anchor
```

## Domain Extension 예시

### Scientific

```text
figure/equation specialization
table statistical structure
```

### Legal

```text
article
paragraph
item
sub-item
```

### Medical

```text
measurement
value
unit
reference range
```

### Corporate

```text
appendix
financial table
document hierarchy
```

Extension은 Core Source identity와 Anchor 체계를 깨지 않는다.

# 14. 구현 책임 분리

## 직접 구현할 책임

- Canonical Source schema
- Stable Anchor
- Structure Integrity Check
- Recovery Router
- Recovery validation
- Source rebinding
- failure state
- parser adapter contract

## 외부 라이브러리에 맡길 수 있는 책임

- DOCX XML decoding
- PDF text extraction
- PDF position extraction
- image decoding
- 기본 OCR
- low-level file format handling

외부 라이브러리는 언제든 교체 가능한 부품으로 취급한다.

# 15. MVP 범위

## 포함

- DOCX deterministic extraction
- digital PDF lightweight extraction
- Canonical Source
- Stable Anchor
- 기본 structure integrity validation
- targeted recovery routing
- recovery re-validation
- 명확한 failure state

## 보류

- scan/OCR 전체 지원
- figure 상세 의미 구조화
- equation 상세 구조화
- 고급 scientific layout
- 도메인별 extension

## 제외

- 문서 의미 이해
- 논증 판단
- 도메인 inference
- 자체 PDF rendering engine
- 전체 문서 기본 Gemini 처리
- 범용 Agent parser
- 모든 문서 형식 지원

# 16. 성능 기조

정상 문서는 가능한 한:

```text
model calls = 0
```

을 기본 경로로 한다.

성능 평가는 다음을 분리한다.

```text
extraction latency
integrity-check latency
recovery latency
memory
model cost
recovery rate
```

# 17. 주요 불변

1. Parser는 의미판단을 수행하지 않는다.
2. Canonical Source는 원문에 다시 결속 가능해야 한다.
3. Stable Anchor는 transient chunk에 의존하지 않는다.
4. 정상 영역에는 불필요한 모델 호출을 하지 않는다.
5. 구조 이상은 가능한 한 deterministic하게 감지한다.
6. 모델 recovery는 문제 영역에만 제한한다.
7. 모델 결과를 검증 없이 정본으로 승격하지 않는다.
8. extraction/recovery 실패를 내용 부재로 해석하지 않는다.
9. 외부 parser library는 교체 가능해야 한다.
10. Domain Extension이 Core Source identity를 변경하지 않는다.

# 18. 수용 테스트

최소 fixture:

### DOCX

- 일반 section/paragraph
- list
- simple table
- merged cell
- multi-level header
- unit
- footnote
- caption

### Digital PDF

- single column
- multi-column
- table
- table + caption
- footnote
- figure + caption
- equation
- deliberately difficult reading order

### Failure

- malformed document
- unsupported format
- partial extraction
- ambiguous table
- unreadable region

### Recovery

각 이상 fixture에 대해:

```text
deterministic extraction
→ integrity failure detection
→ targeted recovery
→ re-validation
→ source rebinding
```

을 검증한다.

# 19. 완료 기준

파서 MVP는 다음이 관찰 가능하게 성립할 때 완료된다.

```text
supported input
→ lightweight extraction
→ integrity decision
→ targeted recovery when necessary
→ validated Canonical Source
→ Stable Anchor
→ original source re-resolution
```

그리고 다음 조건을 만족해야 한다.

- 정상 입력은 모델 없이 처리 가능
- 이상 영역만 recovery 대상으로 선택
- recovery 후 원본 결속 검증 가능
- 기술적 실패와 내용 부재 구분
- 외부 parser implementation 교체 가능
- 후속 의미 시스템이 parser 내부 구현에 의존하지 않음
