---

description: "Task list — saldo de conta, cobertura e conferência por segmento"
---

# Tasks: Saldo de conta, cobertura de dados e conferência por segmento

**Input**: Design documents from `/specs/016-account-balance-coverage/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/)

**Tests**: OBRIGATÓRIOS (Constituição I) — teste vermelho antes de cada implementação;
parser ≥ 90%.

## Format: `[ID] [P?] [Story] Description`

## Path Conventions

Backend em `src-tauri/src/`, testes inline, fixtures em `tests/fixtures/`. Front em `src/`.
Extratos reais só em `~/Documents/casa/` (CLAUDE.md).

---

## Phase 1: Setup

- [X] T001 Criar fixture `tests/fixtures/banestes_extrato_autocancela.txt`: cópia da principal com **+100,00** num lançamento e **−100,00** em outro (dias diferentes), saldos intermediários originais mantidos — total e entradas/saídas continuam fechando; só o segmento acusa
- [X] T002 [P] Registrar módulo vazio `pub mod account_position;` em `src-tauri/src/domain/mod.rs` com doc-comment; `cargo build` verde

**Checkpoint**: fixtures prontas; módulo registrado.

---

## Phase 2: Foundational — entidade + regras puras + persistência

**Purpose**: `AccountPosition`/`Coverage` e as quatro funções puras bloqueiam as três user
stories. Contrato: [contracts/positions_and_coverage.md](contracts/positions_and_coverage.md).

- [X] T003 Testes vermelhos em `src-tauri/src/domain/account_position.rs`: id determinístico (mesma tupla ⇒ mesmo id); `current_positions` (maior `as_of` vence, independe da ordem; produtos não se misturam); `month_coverage` (Full/Partial{until}/None, união de sobrepostos sem dupla contagem, mês atravessado por período que cruza meses); `coverage_gaps` (maio+julho ⇒ [2026-06]; parcial não é buraco); `chain_warning` (divergente ⇒ mensagem com os dois valores; sem posição anterior ⇒ None; posição escolhida é a corrente com `as_of < start`)
- [X] T004 Implementar `src-tauri/src/domain/account_position.rs` conforme [data-model.md](data-model.md) — T003 verde
- [X] T005 Testes vermelhos em `src-tauri/src/infrastructure/db.rs`: roundtrip save/load de posições e coberturas; `INSERT OR REPLACE` idempotente (2× save ⇒ 1 linha); `clear_bank_entries` limpa as três tabelas (FR-011)
- [X] T006 Implementar tabelas `account_positions`/`statement_coverage` + save/load + clear acoplado em `src-tauri/src/infrastructure/db.rs` — T005 verde

**Checkpoint**: domínio e persistência prontos; nada visível ainda.

---

## Phase 3: User Story 1 — Saldo da conta no painel (P1) 🎯 MVP

**Goal**: importar extrato ⇒ posição registrada ⇒ card "Saldo em conta" no painel.

**Independent Test**: fixture jul ⇒ posição 231,30/25-07 no card; reimportar não duplica;
extrato mais novo troca a corrente.

- [X] T007 [US1] Testes vermelhos em `src-tauri/src/domain/banestes_statement.rs`: `ExtratoBanestes` captura `periodo == (2026-07-01, 2026-07-25)` da fixture principal e `saldo_poupanca == 5.000,00` da consolidada; `ParsedStatement` produzido carrega `positions` (corrente 231,30 as_of 25/07; + poupança na consolidada) e `coverage`
- [X] T008 [US1] Implementar captura de período/poupança e montagem de `positions`/`coverage` em `src-tauri/src/domain/banestes_statement.rs` + campos novos (`#[serde(default)]`) em `ParsedStatement` (`src-tauri/src/domain/bank_statement.rs`) — T007 verde; ids de `bank_entries` intocados (teste de regressão `entry_id_key_format_is_frozen` verde)
- [X] T009 [P] [US1] Teste vermelho em `src-tauri/src/domain/bank_statement.rs`: fixture BTG com linha `Saldo Diário` ⇒ 1 posição (último saldo, data da linha); grid sem a linha ⇒ `positions` vazio (research R5)
- [X] T010 [US1] Implementar posição BTG best-effort em `parse_statement_rows` — T009 verde
- [X] T011 [US1] Persistir posições+cobertura no fluxo: `commands/bank.rs` (`import_bank_statement`, `save_bank_statement`) e `application/import_folder.rs::try_import_extrato` gravam junto dos entries; teste em `import_folder` (seam de texto): importar fixture ⇒ posição e cobertura no DB; reimportar ⇒ contagens estáveis
- [X] T012 [US1] Comando `list_account_positions` (posições correntes via domínio) em `src-tauri/src/commands/bank.rs` + registro no builder Tauri; DTO `AccountPositionDto`
- [X] T013 [P] [US1] Front: tipos (`AccountPosition` em `src/types/api.types.ts`), wrapper em `src/services/tauri.service.ts`, card "Saldo em conta" em `src/pages/DashboardPage.vue` (linha por posição corrente com banco/conta/produto/valor/data-base + total; card oculto sem posições) — skill `nielsen-heuristics` aplicada
- [X] T014 [US1] `cargo test && npx vue-tsc --noEmit && npm run test:run` verdes

**Checkpoint**: MVP — saldo real visível no painel.

---

## Phase 4: User Story 2 — Mês parcial, buracos e encadeamento (P1)

**Goal**: cobertura visível e encadeamento avisado.

**Independent Test**: cobertura 01–25/07 ⇒ julho parcial; maio+julho ⇒ buraco junho;
saldo anterior divergente ⇒ aviso sem bloquear.

- [X] T015 [US2] Teste vermelho em `src-tauri/src/commands/bank.rs` (ou domínio, onde a lógica morar): `coverage_summary` devolve parciais `{month, until}` e `gaps` por conta a partir das coberturas persistidas
- [X] T016 [US2] Implementar `coverage_summary` + DTO `CoverageSummary` — T015 verde
- [X] T017 [US2] Teste vermelho: `save_bank_statement`/`import_bank_statement` retornam `SaveStatementResult { saved, chain_warning }`; extrato com saldo anterior divergente da posição anterior ⇒ `chain_warning` preenchido (mensagem com os dois valores), importação completa; primeira importação ⇒ `None`
- [X] T018 [US2] Implementar `SaveStatementResult` nos dois comandos + `FolderImportSummary.warnings` (`#[serde(default)]`) na pasta automática — T017 verde
- [X] T019 [P] [US2] Front: `ExtratoPage.vue` mostra banner de cobertura (meses parciais "dados até DD/MM" + buracos, via `coverage_summary`) e o `chain_warning` no flash pós-import; `SettingsPage`/resumo da pasta exibe `warnings` uma vez; tipos/service atualizados — skill `nielsen-heuristics`
- [X] T020 [US2] Suítes verdes (`cargo test`, `vue-tsc`, Vitest)

**Checkpoint**: cobertura honesta nas telas; encadeamento vigiado.

---

## Phase 5: User Story 3 — Conferência por segmento (P2)

**Goal**: erro auto-cancelado não passa mais. Contrato:
[contracts/segment_reconciliation.md](contracts/segment_reconciliation.md).

- [X] T021 [P] [US3] Testes vermelhos em `src-tauri/src/domain/banestes_statement.rs`: fixtures íntegras ⇒ `segmentos == Fechou`; `banestes_extrato_autocancela.txt` ⇒ `Divergiu` e `exigir()` cita o dia do primeiro segmento + diferença, nada importado; texto sem saldos intermediários ⇒ `SemDados` **tolerado** (importação segue)
- [X] T022 [US3] Implementar captura de `Segmento` na varredura + checagem `segmentos` na `Conferencia` (SemDados não bloqueia; Divergiu bloqueia com dia) — T021 verde; suítes 014/015 intocadas

**Checkpoint**: rede fina ativa.

---

## Phase 6: Polish & Cross-Cutting

- [X] T023 Rodar suíte completa: `cd src-tauri && cargo test`, `npx vue-tsc --noEmit`, `npm run test:run`
- [X] T024 Validar contra o **PDF real** (example temporário via strategy, apagado no mesmo turno): posição 231,30/25-07, cobertura 01–25/07, segmentos fecham; contracheque/extrato alheio inalterados
- [X] T025 [P] Atualizar `docs/ARCHITECTURE.md` (seção 016) e `docs/MAINTENANCE.md` (invariantes: id de posição determinístico, clear acoplado, SemDados de segmento tolerado)
- [X] T026 [P] Atualizar `README.md` (saldo em conta + cobertura)
- [ ] T027 Validação manual pelo [quickstart.md](quickstart.md) no app
- [X] T028 `git status` limpo de arquivos reais/senhas

---

## Dependencies

```text
Phase 1 (T001–T002)
   └─> Phase 2 (T003–T006)  ← bloqueia tudo
          ├─> Phase 3 US1 (T007–T014) 🎯 MVP
          │      └─> Phase 4 US2 (T015–T020)   usa coverage persistida pela US1
          ├─> Phase 5 US3 (T021–T022)          independente de US1/US2 (só parser)
          └─> Phase 6 (T023–T028)
```

## Parallel Execution Examples

- T002 ∥ T001; T009 ∥ T007 (arquivos distintos); T013 ∥ T011–T012; T019 ∥ T017–T018;
  T021 pode rodar em paralelo com toda a US2; T025 ∥ T026.

## Implementation Strategy

MVP = Fases 1–3 (saldo no painel). US2 dá a honestidade temporal; US3 é rede de proteção
pura. Ordem: US1 → US2 → US3 → Polish.

**Total**: 28 tarefas — Setup 2, Foundational 4, US1 8, US2 6, US3 2, Polish 6.
