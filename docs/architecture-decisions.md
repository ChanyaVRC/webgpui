# Architecture Decision Log Policy

## 1. Purpose
Track major technical decisions and their rationale for future maintainability.

## 2. When to Write an ADR
- Crate boundary changes
- Public API compatibility policy changes
- Significant CI threshold changes
- Scope changes between Compat and FastPath

## 3. Minimal Template
```markdown
# ADR-YYYYMMDD-<title>

## Context
## Decision
## Alternatives
## Consequences
## Metrics Impact
```

## 4. Operation Rules
- Make ADR mandatory for breaking changes
- Attach related PRs and metrics in the ADR body
- Update corresponding docs pages after the change
