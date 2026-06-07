# Quickstart & Validation Guide: Gestor Financeiro — Dashboard de Faturas BTG

**Branch**: `001-credit-card-dashboard` | **Date**: 2026-06-07

## Prerequisites

- Rust toolchain (1.75+): `rustup show`
- Node.js 20+: `node --version`
- Tauri CLI v2: `cargo tauri --version`
- A BTG invoice XLSX file with password removed (see Decryption note below)

### Decryption Note

BTG sends invoices as password-protected files. Before importing, open the file in Excel, LibreOffice Calc, or Numbers → enter the password → File → Save As → unprotected XLSX → place in `faturas/`.

---

## Setup

```bash
# Clone and install
git clone <repo>
cd financas
npm install          # frontend deps
cargo build          # Rust backend (first run downloads crates)
```

---

## Run in Development

```bash
cargo tauri dev
```

App opens. Frontend at `http://localhost:1420` (proxied through Tauri webview).

---

## Validation Scenarios

### Scenario 1: Import a BTG Invoice (User Story 1 — P1) ✅

1. Launch the app (`cargo tauri dev`).
2. Click "Selecionar Pasta" → choose `faturas/` directory.
3. Click "Importar Faturas" → select the decrypted XLSX file.
4. **Expected**: Confirmation banner: "Fatura importada: X transações".
5. **Expected**: No errors in the warnings list.
6. **Verify**: Open DevTools → check Pinia store has `invoices[0].transactions.length > 0`.

**Edge case — encrypted file**:
1. Select the original (encrypted) BTG XLSX.
2. **Expected**: Error message: "Arquivo protegido por senha. Remova a proteção antes de importar."

**Edge case — duplicate import**:
1. Import same file twice.
2. **Expected**: Warning "Fatura já importada — substituída."
3. **Expected**: Transaction count unchanged.

**Validated via**: 6 Rust integration tests against `tests/fixtures/sample_fatura.xlsx` (multi-section BTG XLSX) — all pass. `test_encrypted_file_returns_error`, `test_parse_btg_fixture_returns_transactions`, `test_fixture_categories_inferred`, `test_fixture_reversal_detected`, `test_fixture_installment_parsed_from_description`, `test_fixture_transactions_have_valid_dates`. ImportButton and invoice store Vitest tests pass.

---

### Scenario 2: View Expense Dashboard (User Story 2 — P2) ✅

After importing at least one invoice:

1. Navigate to "Dashboard" tab.
2. **Expected**: Donut chart displays expense categories with Portuguese labels.
3. **Expected**: Horizontal bar chart shows categories ordered largest → smallest.
4. **Expected**: Category table shows `nome | total R$ | % do total | nº transações`.
5. **Verify**: Sum of all category `net_total` values equals `DashboardData.net_total`.

**Validated via**: CategoryChart.test.ts (series data length = categories.length), CategoryRanking.test.ts (ascending sort verified, accent color on max), aggregate_by_category unit tests confirm percentages and net totals correct.

---

### Scenario 3: Identify Biggest Expense (User Story 3 — P2) ✅

1. On Dashboard, confirm the first bar in the ranking chart is highlighted (different color).
2. **Expected**: Banner "Maior gasto: [Category] — R$ X.XX (XX%)" visible above charts.
3. **Expected**: "Top 5 Transações" list shows 5 transactions ordered by amount desc.
4. **Verify**: Manually sum 3 random category totals from the XLSX — should match chart.

**Validated via**: BiggestSpendBanner.test.ts (name, percentage, R$ amount rendered), TopTransactions.test.ts (5 rows ordered by caller, date format dd/mm/yyyy), Rust unit test `test_top_transactions_returns_5_largest`.

---

### Scenario 4: Monthly Trend (User Story 4 — P3) ✅

Requires 2+ invoices from different months:

1. Import two invoices from different months.
2. Navigate to "Histórico" tab.
3. **Expected**: Line chart shows 2+ data points, one per month.
4. **Expected**: Hovering a point shows month, total, and top categories for that month.

**Validated via**: MonthlyTrend.test.ts (2 snapshots → 2 xAxis points, month labels "Mai/2026", missing-category months fill with 0), Rust unit tests `test_monthly_trend_empty_when_single_invoice` and `test_monthly_trend_two_months`.

---

## Running Tests

### Rust unit tests

```bash
cd src-tauri
cargo test
```

All domain logic tests (categorization, aggregation, decimal math) must pass.

### Vue component tests

```bash
npm run test          # Vitest (watch mode)
npm run test:run      # Vitest (single run)
```

### E2E tests (Playwright)

```bash
npm run test:e2e
```

Launches Tauri in test mode and runs full UI scenarios.

---

## Expected Test Output (pre-implementation RED phase)

When running tests before implementation, all tests should **fail** — this confirms TDD discipline:

```
FAIL src-tauri/src/domain/categorizer.rs - 3 tests, 3 failures
FAIL src-tauri/src/domain/aggregator.rs  - 5 tests, 5 failures
FAIL src/components/Dashboard.test.ts    - 8 tests, 8 failures
```

Only after implementing each module should the corresponding tests turn green.
