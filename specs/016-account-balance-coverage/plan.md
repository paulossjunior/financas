# Implementation Plan: Saldo de conta, cobertura de dados e conferência por segmento

**Branch**: `016-account-balance-coverage` | **Date**: 2026-07-26 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/016-account-balance-coverage/spec.md`

## Summary

Primeira entidade de **estoque** do app: cada importação de extrato passa a produzir, além
dos lançamentos, **posições de conta** (saldo final; poupança no consolidado; último "Saldo
Diário" no BTG quando existir) e **cobertura** (período impresso no cabeçalho). Dashboard
ganha o card "Saldo em conta"; a tela de extrato ganha mês parcial/buracos/aviso de
encadeamento. No parser Banestes, os saldos intermediários viram **conferência por
segmento** (pega erro que se auto-cancela). Fluxos existentes intactos (FR-009).

## Technical Context

**Language/Version**: Rust 1.75+ (Tauri v2) + TypeScript 5/Vue 3.

**Primary Dependencies**: nenhuma nova — `rust_decimal`, `chrono`, `regex`, `rusqlite`,
`uuid` v5 já presentes.

**Storage**: SQLite — 2 tabelas novas (`account_positions`, `statement_coverage`), upsert
por id determinístico (UUIDv5); remoção acoplada ao "limpar extrato" (FR-011).

**Testing**: `cargo test` inline + fixtures de texto existentes (já contêm período e
saldos diários) + 1 fixture nova de erro auto-cancelado; Vitest/vue-tsc no front.

**Target Platform / Project Type**: desktop Tauri local (como 014/015).

**Performance Goals**: irrelevante em escala (≤ dezenas de posições/coberturas); consultas
diretas sem índice extra além dos PKs.

**Constraints**: `Decimal` em dinheiro; nenhum extrato real no repo; aviso de encadeamento
NÃO bloqueia importação (spec); conferência por segmento bloqueia (mesma política estrita
das conferências existentes).

**Scale/Scope**: 1 usuário, 2 bancos, ~12 extratos/ano por conta.

## Constitution Check

| Princípio | Cumprimento |
|---|---|
| I. TDD | Testes vermelhos antes de cada implementação (tasks.md); parser ≥90% mantido — a conferência por segmento nasce de teste com fixture adulterada. |
| II. Clean Architecture | Entidades e regras puras em `domain/account_position.rs` (posição, cobertura, parciais/buracos/encadeamento — zero I/O); extração dos dados no parser de domínio existente; persistência em `infrastructure/db.rs`; orquestração nos commands. |
| III. Simplicidade | Sem CRUD de contas, sem projeção — só o que o spec pede. Posição BTG = melhor esforço do dado já presente (linha "Saldo Diário" hoje descartada); nada inventado quando o arquivo não traz (`Option`). |
| IV. Data Integrity | Segmentos com `Decimal`; segmento divergente aborta com dia + diferença; posições idempotentes por id determinístico; encadeamento divergente é AVISO explícito (nunca silêncio, nunca bloqueio indevido). |
| V. Local-First | Tudo no SQLite local; fixtures anonimizadas (as existentes já servem). |

**Gate**: PASS (inicial e pós-design re-check).

## Project Structure

### Documentation (this feature)

```text
specs/016-account-balance-coverage/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/
│   ├── positions_and_coverage.md     # domínio + persistência + comandos
│   └── segment_reconciliation.md     # conferência por segmento no parser
└── tasks.md                          # (/speckit-tasks)
```

### Source Code (repository root)

```text
src-tauri/src/
├── domain/
│   ├── account_position.rs      # NOVO: AccountPosition, Coverage + regras puras
│   │                            #   (corrente, parciais, buracos, encadeamento, união)
│   ├── banestes_statement.rs    # captura período, saldo poupança, segmentos diários;
│   │                            #   Conferencia ganha a checagem de segmentos
│   ├── bank_statement.rs        # ParsedStatement ganha positions/coverage (default);
│   │                            #   BTG: última linha "Saldo Diário" → posição (best effort)
│   └── mod.rs
├── infrastructure/db.rs         # tabelas account_positions/statement_coverage +
│                                #   save/load/clear acoplado ao clear_bank_entries
├── commands/bank.rs             # persistir posição+cobertura no save/import; aviso de
│                                #   encadeamento na resposta; list_positions/coverage_summary
└── application/import_folder.rs # extrato da pasta também grava posição+cobertura

src/
├── pages/DashboardPage.vue      # card "Saldo em conta" (por conta + total + data-base)
├── pages/ExtratoPage.vue        # mês parcial, buracos, aviso de encadeamento pós-import
├── services/tauri.service.ts    # wrappers novos
└── types/api.types.ts           # AccountPosition, CoverageSummary, SaveStatementResult

tests/fixtures/
└── banestes_extrato_autocancela.txt  # NOVO: +100/−100 (total fecha, segmento não)
```

**Structure Decision**: mesma arquitetura das 014/015 — regra pura no domínio, I/O na
infraestrutura, orquestração fina nos commands; nenhum diretório novo.

## Complexity Tracking

Sem violações — tabela vazia.
