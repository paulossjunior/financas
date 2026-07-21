# Implementation Plan: Importar extrato bancário do BTG

**Branch**: `008-bank-statement-import` | **Date**: 2026-07-21 | **Spec**: [spec.md](spec.md)

## Summary
Ler o extrato .xls do BTG, classificar cada lançamento (crédito/débito), **excluir** o que o app já conta (fatura do cartão, salário com contracheque, transferência interna), **categorizar** (regras do app + fallback BTG), **salvar local** com dedup, e somar no painel como despesa avulsa / renda extra por mês. Prévia antes de confirmar.

## Technical Context
- **Rust** (Tauri) + **Vue 3**. calamine (já dep) lê .xls. `rust_decimal`, `rusqlite`, `chrono`, `uuid` (já).
- **Storage**: nova tabela `bank_entries` (dedup por UUIDv5 de data+desc+valor+conta).
- **Testing**: `cargo test` — parser (linhas sintéticas) + classificação (pura), TDD ≥90%.
- **Integração**: lançamentos incluídos viram `ManualEntry` (avulso/renda) e entram no pipeline de dashboard/ano/inflação sem dupla contagem.
- **Local-first**: leitura de arquivo local; nenhuma rede.

## Constitution Check
- **I. TDD**: `parse_statement_rows` + `classify_entry` puros → testes primeiro (saldo diário/blank ignorados; fatura/salário/interno excluídos; crédito/débito; categorização + fallback; dedup id).
- **II. Clean arch**: `infrastructure/btg_statement.rs` (I/O calamine) → linhas; `domain/bank_statement.rs` (puro: parse linhas + classificação); `commands/bank.rs` fino; `db.rs` persiste.
- **III. Simplicidade**: incluídos reusam o pipeline de ManualEntry — sem nova agregação no dashboard/ano.
- **IV. Integridade**: `Decimal`; dedup determinístico; exclusão evita dupla contagem.
- **V. Local-first**: só arquivo local.

## Project Structure
```
src-tauri/src/
  domain/bank_statement.rs     # NOVO — RawEntry, ClassifiedEntry, parse_statement_rows, classify_entry (+ testes)
  infrastructure/btg_statement.rs  # NOVO — read_statement(path) via calamine → linhas → parse
  infrastructure/db.rs         # + bank_entries (save/load/remove) + migração
  commands/bank.rs             # NOVO — preview_bank_statement, import_bank_statement, list_bank_entries, remove_bank_entry
  application/get_dashboard.rs  # + merge bank entries (como ManualEntry)
  commands/dashboard.rs        # get_year_summary_cmd + get_dashboard_cmd: incluir bank entries
src/
  pages/ExtratoPage.vue        # NOVO — importar + prévia (incluídos/excluídos) + lista + remover
  router + nav                 # rota /extrato
  services/tauri.service.ts + types/api.types.ts
```

## Complexity Tracking
Sem desvios. Reuso do pipeline de ManualEntry evita nova lógica de agregação.
