# Data Model — Saldo de conta, cobertura e conferência por segmento

**Feature**: 016-account-balance-coverage · **Date**: 2026-07-26

## Entidades novas (domínio: `domain/account_position.rs`)

### `AccountPosition`

| Campo | Tipo | Regra |
|---|---|---|
| `id` | `String` | `UUIDv5("position:{bank}:{account}:{product}:{as_of}")` — idempotência |
| `bank` | `String` | "Banestes", "BTG" (vem do `ParsedStatement.bank`) |
| `account` | `String` | mesma string congelada do `BankEntry.account` |
| `product` | `enum Product { Corrente, Poupanca }` | poupança só quando o consolidado imprimir |
| `balance` | `Decimal` (serializa string) | saldo impresso |
| `as_of` | `NaiveDate` | fim do período (Banestes) / data do último Saldo Diário (BTG) |
| `source_file` | `String` | nome do arquivo importado (rastreio + FR-011) |

### `Coverage`

| Campo | Tipo | Regra |
|---|---|---|
| `id` | `String` | `UUIDv5("coverage:{bank}:{account}:{start}:{end}")` |
| `bank`, `account` | `String` | idem posição |
| `start`, `end` | `NaiveDate` | período impresso no cabeçalho (só Banestes na v1) |
| `source_file` | `String` | idem |

### Funções puras (mesmo módulo)

```rust
/// Posição corrente por (bank, account, product): maior as_of.
pub fn current_positions(all: &[AccountPosition]) -> Vec<AccountPosition>;

/// Status de um mês ante a união das coberturas da conta.
pub enum MonthCoverage { Full, Partial { until: NaiveDate }, None }
pub fn month_coverage(covs: &[Coverage], month: &str /* YYYY-MM */) -> MonthCoverage;

/// Meses YYYY-MM sem nenhuma cobertura entre o primeiro start e o último end da conta.
pub fn coverage_gaps(covs: &[Coverage]) -> Vec<String>;

/// Aviso de encadeamento: saldo_anterior do extrato novo vs posição corrente
/// imediatamente anterior a `new_start` (mesma conta, produto Corrente).
pub fn chain_warning(
    positions: &[AccountPosition],
    new_start: NaiveDate,
    saldo_anterior: Decimal,
) -> Option<String>; // mensagem pt-BR com os dois valores
```

## Entidades alteradas

### `ExtratoBanestes` (014) — campos novos

| Campo | Origem |
|---|---|
| `periodo: Option<(NaiveDate, NaiveDate)>` | `Cliente: … Período: DD/MM/YYYY à DD/MM/YYYY` |
| `saldo_poupanca: Option<Decimal>` | `Saldo Poupança …` (consolidado; hoje dropado) |
| `segmentos: Vec<Segmento>` | cada `Saldo` intermediário fecha um trecho |

`Conferencia` ganha `segmentos: Checagem` — `SemDados` aqui é **tolerado** (extrato sem
saldos intermediários), diferente das checagens totais; `Divergiu` aborta citando o dia.

### `ParsedStatement` (compartilhado) — campos novos, `#[serde(default)]`

| Campo | Preenchido por |
|---|---|
| `positions: Vec<AccountPosition>` | Banestes (corrente + poupança); BTG (último Saldo Diário, se houver) |
| `coverage: Option<(NaiveDate, NaiveDate)>` | Banestes; BTG nunca (não imprime período) |

Ids de `bank_entries` intocados (campos novos não entram em `entry_key`).

## Persistência (SQLite)

```sql
CREATE TABLE IF NOT EXISTS account_positions (
  id          TEXT PRIMARY KEY,   -- UUIDv5 determinístico
  bank        TEXT NOT NULL,
  account     TEXT NOT NULL,
  product     TEXT NOT NULL,      -- 'corrente' | 'poupanca'
  balance     TEXT NOT NULL,      -- Decimal como string
  as_of       TEXT NOT NULL,      -- YYYY-MM-DD
  source_file TEXT NOT NULL,
  imported_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS statement_coverage (
  id          TEXT PRIMARY KEY,
  bank        TEXT NOT NULL,
  account     TEXT NOT NULL,
  start       TEXT NOT NULL,
  end         TEXT NOT NULL,
  source_file TEXT NOT NULL
);
```

Escrita: `INSERT OR REPLACE` (idempotência). `clear_bank_entries()` limpa as três tabelas
(FR-011). Sem migração de dados existentes (tabelas nascem vazias; posições aparecem na
próxima importação).

## DTOs (commands ↔ front)

| DTO | Conteúdo |
|---|---|
| `AccountPositionDto` | bank, account, product, balance (string), as_of |
| `CoverageSummary` | por conta: `partial_months: [{month, until}]`, `gaps: [month]` |
| `SaveStatementResult` | `saved: usize`, `chain_warning: Option<String>` (substitui o `usize` cru de `save_bank_statement`/`import_bank_statement`) |
| `FolderImportSummary` | ganha `warnings: Vec<String>` (`#[serde(default)]`) |

## Fluxo

```text
importar extrato ─► ParsedStatement { entries, positions, coverage }
   ├─► bank_entries (fluxos — intacto)
   ├─► account_positions (upsert)          ─► card "Saldo em conta" (posições correntes)
   ├─► statement_coverage (upsert)         ─► mês parcial / buracos (funções puras)
   └─► chain_warning(posições, saldo_anterior) ─► aviso no resultado (não bloqueia)
parser Banestes: segmentos conferidos ANTES de tudo (Divergiu ⇒ nada acima acontece)
```

## Invariantes

1. Reimportar o mesmo arquivo não altera contagem de posições nem de coberturas.
2. Posição corrente independe da ordem de importação (só de `as_of`).
3. Nenhuma posição/cobertura sem extrato correspondente após "limpar extrato".
4. `Divergiu` em segmento ⇒ zero linhas, zero posições, zero cobertura gravadas.
5. Encadeamento divergente ⇒ importação completa + aviso (nunca silêncio, nunca bloqueio).
