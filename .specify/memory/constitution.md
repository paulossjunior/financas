<!--
SYNC IMPACT REPORT
==================
Version change: (none) → 1.0.0
Status: Initial ratification — all content is new

Principles added:
  - I. Test-Driven Development (TDD)
  - II. Clean Architecture & Design Patterns
  - III. Code Optimization & Simplicity
  - IV. Data Integrity
  - V. Local-First & Privacy

Templates reviewed:
  - .specify/templates/plan-template.md    ✅ aligned (Constitution Check section present)
  - .specify/templates/spec-template.md   ✅ aligned (no implementation details required)
  - .specify/templates/tasks-template.md  ✅ aligned (TDD task ordering matches Principle I)

Follow-up TODOs:
  - None. All placeholders resolved.
-->

# Financas Constitution

## Core Principles

### I. Test-Driven Development (NON-NEGOTIABLE)

TDD is mandatory for all implementation work. The cycle is strict and cannot be skipped:

- Tests MUST be written before implementation code.
- Tests MUST fail (red) before any production code is written.
- Implementation MUST be the minimum code to make the test pass (green).
- Refactoring MUST happen after tests pass — never before.
- Unit tests cover domain logic; integration tests cover data parsing and aggregation flows.
- A feature is NOT considered done unless tests exist, pass, and were written first.
- Test coverage on core business logic (parsers, categorizers, aggregators) MUST be ≥ 90%.

### II. Clean Architecture & Design Patterns

Code MUST follow separation of concerns with distinct, independently testable layers:

- **Domain layer**: pure business rules (categories, expense aggregation, transaction models) — zero external dependencies.
- **Application layer**: use-cases and orchestration — depends only on domain.
- **Infrastructure layer**: file I/O, XLSX parsing — depends on application layer contracts, never on domain directly.
- **Presentation layer**: dashboard rendering — consumes application output only.
- Design patterns MUST be applied where they reduce coupling or improve testability. Overuse is a violation: every pattern must be justified by a concrete problem it solves.
- SOLID principles apply: Single Responsibility, Open/Closed, Liskov Substitution, Interface Segregation, Dependency Inversion.
- Interfaces/protocols MUST define boundaries between layers so each layer can be tested in isolation.

### III. Code Optimization & Simplicity

- YAGNI: build only what the current spec requires. No speculative abstractions.
- DRY: duplication is allowed exactly twice; the third occurrence triggers extraction.
- Premature optimization is a violation. Optimize only after profiling identifies a real bottleneck.
- Every function MUST have a single, clear responsibility.
- Naming MUST be self-documenting; comments are reserved for non-obvious invariants.
- Cyclomatic complexity per function MUST stay ≤ 10. Refactor above this threshold.
- Dead code MUST be deleted — not commented out.

### IV. Data Integrity

Financial data is sensitive and must be processed with zero tolerance for silent errors:

- All numeric operations on monetary values MUST use exact decimal arithmetic (not floating point).
- Every XLSX parse operation MUST validate expected column structure before processing rows.
- Invalid, malformed, or duplicate records MUST be rejected explicitly with a clear error message — never silently ignored or defaulted.
- Aggregation results (category totals, percentage breakdowns) MUST be deterministic given the same input files.
- Estornos (negative transactions) MUST be handled explicitly and documented in the data model.

### V. Local-First & Privacy

- All user financial data MUST remain on the local machine. No network calls for data processing.
- No telemetry, analytics, or external API calls that transmit transaction data.
- The `faturas/` directory is the sole data entry point; no direct bank API integration in this project scope.
- Dependencies MUST be audited before adoption: no dependency may phone home or require authentication to external services.

## Development Workflow

- Red → Green → Refactor. No exceptions.
- Every layer boundary has a contract test before integration tests are written.
- Commits MUST be atomic: one logical change per commit, tests included.
- A pull request or merge MUST NOT be approved if any tests are failing or if TDD cycle was bypassed.
- Architecture violations (e.g., domain layer importing infrastructure) are blocking defects.

## Governance

- This constitution supersedes all other coding conventions in this repository.
- Any amendment requires: (a) description of the change, (b) rationale, (c) version bump per semantic rules, (d) update to this file.
- MAJOR bump: removal or redefinition of an existing principle.
- MINOR bump: new principle or section added.
- PATCH bump: wording clarification, typo fix, non-semantic refinement.
- All implementation plans (`/speckit-plan`) MUST include a Constitution Check gate before Phase 0 research begins.
- Complexity exceptions MUST be logged in the plan's Complexity Tracking table with explicit justification.

**Version**: 1.0.0 | **Ratified**: 2026-06-07 | **Last Amended**: 2026-06-07
