# STATUS

## Repository State

- Project: Lightweight Source Parser
- Repository: `peion-svg/headroom`
- Current phase: implementation preparation
- Implementation status: NOT_STARTED
- Active WorkContract: `WC-PARSER-001`

## Canonical Project Documents

- `docs/DESIGN_BASELINE.md`
- `docs/IMPLEMENTATION_ARCHITECTURE.md`
- `docs/workcontracts/WC-PARSER-001.md`

## Current Target

First vertical slice:

```text
DOCX
→ deterministic extraction
→ structural validation
→ Canonical Source
→ Stable Anchor
→ source resolve
```

## Verified So Far

- Repository reset for reuse
- README reduced to stable project information
- Design baseline created
- Implementation architecture created
- Agent work rules created
- First WorkContract created

## Not Yet Verified

- Rust project build
- DOCX extraction
- Canonical Source implementation
- Stable Anchor implementation
- source resolve roundtrip
- fixture-based acceptance tests

## Next Action

Implement `WC-PARSER-001` only.

Do not start PDF, recovery, OCR, or later WorkContracts before `WC-PARSER-001` reaches PASS or HOLD.
