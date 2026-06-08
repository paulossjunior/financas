# Quickstart: Listagem Mensal de Faturas

**Feature**: specs/002-monthly-invoice-list
**Date**: 2026-06-07

---

## Prerequisites

- Tauri dev environment running: `npx tauri dev`
- At least 2 XLSX invoices from different months available in `faturas/`
- OR: use Playwright mock for automated validation (see below)

---

## Scenario 1 — US1: Listar faturas agrupadas por mês

### Manual validation

1. Start app: `npx tauri dev`
2. Import 3 invoices from different months via Dashboard → "Importar Faturas"
3. Navigate to **"Histórico"** tab
4. Expected:
   - 3 month groups visible, sorted most-recent-first
   - Each group shows: month label (e.g., "Maio 2026"), N faturas, total líquido (R$)
   - Individual invoice files listed inside each group

### Automated (Playwright)

```ts
// tests/history.spec.ts
// Mock list_invoices with 3 invoices across 2 months
// Mock get_dashboard_cmd with monthly_trend for those months
// Navigate to /historico
// Assert: page has 2 month group headers
// Assert: "Maio/2026" group shows 2 invoices, correct total
```

### Pass criteria

- All imported invoices appear (FR-003: SC-003)
- Groups in descending chronological order (FR-002)
- Total matches `MonthlySnapshot.net_total` for that month
- Loading completes in < 2s (SC-001)

---

## Scenario 2 — US2: Filtrar dashboard por mês

### Manual validation

1. From Histórico page, click "Ver dashboard" on any month group
2. Expected:
   - Navigated to Dashboard tab automatically
   - Dashboard header shows filter badge: "Filtrado: Mai/2026 · ✕ Limpar"
   - KPI cards, charts, transactions reflect only that month's data
3. Click "✕ Limpar" on filter badge
4. Expected:
   - Badge disappears
   - Dashboard shows all-time aggregated data again

### Automated (Playwright)

```ts
// tests/history.spec.ts
// Click "Ver dashboard" button on first month group
// Assert: URL is "/"
// Assert: filter badge visible with correct month label
// Assert: net_total matches mock monthly_trend for that month
// Click clear button
// Assert: filter badge gone
// Assert: net_total matches all-time total
```

### Pass criteria

- Dashboard loads filtered data < 1s after click (SC-002)
- Filter badge shows correct month label
- Clearing filter restores all-time dashboard (FR-008)

---

## Scenario 3 — US3: Remover fatura

### Manual validation

1. In Histórico, find a month group with 2 invoices
2. Click "Remover" on one invoice
3. Native confirm dialog appears → click "Sim"
4. Expected:
   - Invoice row disappears
   - Month group total updates (recalculated, SC-004)
   - Group still shows with 1 invoice
5. Repeat for the remaining invoice in that group
6. Expected: entire month group disappears (FR-007)

### Automated (Playwright)

```ts
// tests/history.spec.ts
// Mock remove_invoice to return void
// Click remove on invoice
// Accept confirm dialog (page.once('dialog', d => d.accept()))
// Assert: invoice row count decreases by 1
// Assert: store.invoices.length decreases by 1
```

### Pass criteria

- After removal, list + totals update < 1s (SC-004)
- Remove with single invoice in group → group disappears (FR-007)
- No page reload required

---

## Scenario 4 — Edge cases

| Edge case | Steps | Expected |
|-----------|-------|----------|
| No invoices | Open Histórico with empty store | "Nenhuma fatura importada" empty state |
| Unknown month file | Import file with non-standard name | File appears in "Mês desconhecido" group at bottom |
| Filter month then remove all its invoices | Filter by month, remove last invoice | Auto-redirect to Histórico, filter cleared |
| 2 files same month | Import 2 files with same `YYYY-MM` prefix | Single group with both files, total = sum of both |

---

## Running Tests

```bash
# Unit tests (Vitest)
npm run test

# E2E (Playwright) — requires dev server on :1420
npm run dev &
npx playwright test tests/history.spec.ts
```

## Reference

- Data model: [data-model.md](./data-model.md)
- UI contracts: [contracts/ui-contracts.md](./contracts/ui-contracts.md)
- Spec: [spec.md](./spec.md)
