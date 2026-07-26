# Contract — Posições de conta e cobertura

**Feature**: 016 · **Camadas**: `domain/account_position.rs` (puro), `infrastructure/db.rs`,
`commands/bank.rs`

## Domínio (puro)

Assinaturas em [data-model.md](../data-model.md#funções-puras-mesmo-módulo). Regras:

- `current_positions`: agrupa por (bank, account, product), maior `as_of` vence; empate de
  `as_of` (mesmo extrato reimportado) = mesma posição (id igual).
- `month_coverage`: união dos intervalos da conta interceptada com o mês civil;
  `Full` = todos os dias do mês cobertos; `Partial{until}` = prefixo coberto até `until`;
  `None` = nenhum dia. Sobreposições não contam duas vezes.
- `coverage_gaps`: meses `YYYY-MM` inteiramente sem cobertura, do mês do primeiro `start`
  ao mês do último `end` (exclusive os parciais — parcial não é buraco).
- `chain_warning`: posição corrente com `as_of < new_start` (produto Corrente, mesma
  conta); se `saldo_anterior != balance`, mensagem pt-BR:
  `"O saldo anterior deste extrato (R$ X) não bate com o saldo final do período anterior
  (R$ Y, extrato até DD/MM/YYYY). Pode haver um extrato faltando entre eles."`

## Persistência (`infrastructure/db.rs`)

```rust
pub fn save_account_positions(&mut self, items: &[AccountPosition]) -> Result<(), String>; // INSERT OR REPLACE
pub fn load_account_positions(&self) -> Result<Vec<AccountPosition>, String>;
pub fn save_statement_coverage(&mut self, items: &[Coverage]) -> Result<(), String>;
pub fn load_statement_coverage(&self) -> Result<Vec<Coverage>, String>;
// clear_bank_entries(): DELETE também em account_positions e statement_coverage (FR-011)
```

## Comandos Tauri (`commands/bank.rs`)

```rust
/// Posições correntes + total, prontas para o card do painel.
#[tauri::command] list_account_positions() -> Vec<AccountPositionDto>;

/// Meses parciais e buracos por conta (ExtratoPage + badge do painel).
#[tauri::command] coverage_summary() -> Vec<CoverageSummary>;

/// save_bank_statement / import_bank_statement passam a retornar SaveStatementResult
/// { saved, chain_warning } e a persistir positions + coverage junto dos entries.
```

Compatibilidade: o front é atualizado no mesmo PR (tipos em `api.types.ts`); nenhum
consumidor externo.

## Pasta automática (`application/import_folder.rs`)

`try_import_extrato` grava positions/coverage e acumula `chain_warning` em
`FolderImportSummary.warnings` (novo, `#[serde(default)]`) — exibido uma vez no resumo.

## UI

- **DashboardPage**: card "Saldo em conta" — uma linha por posição corrente
  (`Banestes · conta 44/2847023-5 · R$ 231,30 · extrato de 25/07`), linha de poupança
  quando existir, total no topo; skill `nielsen-heuristics` aplicada (status visível,
  reconhecimento, minimalismo — sem card quando não há posição nenhuma).
- **ExtratoPage**: banner de cobertura (meses parciais "dados até DD/MM" + buracos) e o
  `chain_warning` no flash pós-import.
