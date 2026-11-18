# WC-PARSER-001 — DOCX Deterministic Source Slice

## Purpose

DOCX 입력 하나를 deterministic하게 처리하여 검증 가능한 `Canonical Source + Stable Anchor`를 생성하고 원문 위치로 다시 resolve할 수 있게 한다.

## Required Result

```text
DOCX fixture
→ deterministic extraction
→ structural validation
→ Canonical Source
→ Stable Anchor
→ source resolve
```

하나의 실제 DOCX fixture에서 이 경로가 끝까지 동작해야 한다.

## Work World

이번 작업에서 유효한 기준:

- `SOURCE-PARSER-DESIGN-BASELINE v0.1`
- `SOURCE-PARSER-IMPLEMENTATION-ARCHITECTURE v0.1`
- 새 저장소의 `AGENTS.md`
- 현재 DOCX fixture
- Rust toolchain
- DOCX의 실제 XML/relationship 구조
- 이번 계약에서 작성한 코드와 테스트 결과

외부 parser의 설계나 과거 SCI 구현은 현재 구현 권위가 아니다.

## Boundary

### Allowed

- Rust 기반 parser core 작성
- DOCX 저수준 decoding library 도입
- Canonical Source 최소 모델 구현
- Stable Anchor 최소 모델 구현
- DOCX adapter 구현
- DOCX 구조 검증
- source resolve 구현
- 필요한 fixture/test 작성

### Forbidden

- PDF 구현
- Gemini/recovery 구현
- OCR
- 의미판단
- 논증 구조
- domain extension
- DB/persistence
- Agent orchestration
- 미래 format용 speculative abstraction
- 자체 DOCX ZIP/XML 엔진 제작이 기존 검증 라이브러리보다 명확한 이점 없이 필요한 것으로 가정
- 다음 WorkContract 기능 선행 구현

Boundary 밖 변경이 필요하면 임의 확장하지 말고 `HOLD`로 반환한다.

## Required State Transition

```text
DOCX bytes
→ extracted structure
→ validated structure
→ canonical source
→ stable anchors
→ resolvable source
```

## Minimum Canonical Source

이번 계약에서 필요한 node type만 구현한다.

```text
Document
Section / Heading
Paragraph
List
Table
Row
Cell
Caption
Footnote
```

fixture가 특정 node type을 실제로 포함하지 않는다면 존재하지 않는 기능을 억지로 구현하지 않는다.

공통 최소 필드:

```text
source_node_id
node_type
content
parent_id
document_order
source_location
extraction_origin
extraction_status
```

## Minimum Stable Anchor

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

표 셀처럼 구조 위치가 필요한 경우 최소한의 row/column 정보를 추가할 수 있다.

## Required Invariants

1. DOCX 구조를 의미판단으로 재작성하지 않는다.
2. 외부 DOCX library의 transient ID를 Canonical identity로 사용하지 않는다.
3. Stable Anchor는 retrieval chunk에 의존하지 않는다.
4. 추출 실패를 내용 부재로 변환하지 않는다.
5. Canonical node에서 원문 Source를 다시 resolve할 수 있어야 한다.
6. 이번 계약에서는 모델/API 호출이 없어야 한다.

## Required Tests

최소 fixture는 다음 구조를 포함한다.

- heading/section
- paragraph
- list
- simple table
- merged cell 또는 multi-level header 중 최소 하나
- caption 또는 footnote 중 최소 하나

검증:

- document order 보존
- parent/child 관계
- table row/cell 관계
- 포함된 merged/header 관계 보존
- source node identity 안정성
- anchor 생성
- anchor → source resolve roundtrip
- quote/hash 일치
- malformed/unsupported DOCX의 명시적 실패
- `cargo fmt --check`
- `cargo test`

## PASS

다음이 모두 참일 때만 PASS:

- 실제 fixture가 deterministic path로 처리됨
- 필요한 구조가 Canonical Source에 보존됨
- Stable Anchor가 생성됨
- anchor roundtrip으로 동일 source를 재확인 가능
- 기술 실패와 내용 부재가 구분됨
- 모든 필수 테스트 통과
- PDF/Recovery 등 다음 단계 기능을 구현하지 않음

## HOLD

- DOCX 구조 보존에 필요한 핵심 결정을 설계문서만으로 결정할 수 없음
- 선택한 library가 필수 source information을 제공하지 않아 Boundary 밖 구조 변경이 필요함
- fixture가 계약 수용조건을 검증하기에 실제로 부족함

## FAIL

- 구조 손실을 정상 Source로 승인
- Anchor가 원문으로 resolve되지 않음
- 필수 fixture/test 실패
- 계약 밖 기능을 포함시켜야만 결과가 동작함

## Minimum Sufficient / Stop

필수 fixture에서:

```text
DOCX
→ Canonical Source
→ Stable Anchor
→ original source resolve
```

가 검증되고 필수 테스트가 모두 통과하면 종료한다.

더 일반적인 parser abstraction, PDF 대비 설계, recovery 준비, 성능 최적화는 작업 지속 사유가 아니다.
