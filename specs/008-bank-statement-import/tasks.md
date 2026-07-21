# Tasks: Importar extrato bancário do BTG

TDD. `[P]` paralelizável.

## Phase 1 — Domínio (puro, testável)
- [ ] T001 [P] Testes para `domain/bank_statement.rs`: parse_statement_rows (ignora saldo diário/blank; acha colunas por cabeçalho; crédito/débito; data); classify_entry (fatura/salário-com-contracheque/interno → excluído com motivo; categorização + fallback BTG; dedup id).
- [ ] T002 Implementar `domain/bank_statement.rs`: `RawEntry`, `ClassifiedEntry`, `parse_statement_rows`, `classify_entry`, `entry_id`. Passar T001.

## Phase 2 — Infra + persistência
- [ ] T003 `infrastructure/btg_statement.rs`: `read_statement(path)` via calamine → linhas (Vec<Vec<String>>) → parse_statement_rows.
- [ ] T004 `infrastructure/db.rs`: tabela `bank_entries` + migração + save/load/remove.

## Phase 3 — Comandos + integração
- [ ] T005 `commands/bank.rs`: `preview_bank_statement(path)`, `import_bank_statement(path)` (salva incluídos), `list_bank_entries`, `remove_bank_entry`. Registrar em lib.rs.
- [ ] T006 Integração: mesclar bank entries (incluídos) como `ManualEntry` em get_dashboard/get_year_summary/get_inflation (sem dupla contagem).

## Phase 4 — UI
- [ ] T007 `pages/ExtratoPage.vue`: importar (dialog) → prévia (incluídos c/ categoria + excluídos c/ motivo) → confirmar → lista + remover; rota /extrato + link no menu; tipos + service.

## Phase 5 — Polish
- [ ] T008 [P] cargo test + clippy -D warnings; vue-tsc + vitest.
- [ ] T009 Validar no app com o extrato real: incluídos/excluídos corretos, sem duplicar cartão/salário, soma no painel.

## MVP
T001–T006 (importar + excluir + categorizar + somar). UI (T007) logo após.
