# Data Model: Listagem Mensal de Faturas

**Feature**: specs/002-monthly-invoice-list
**Date**: 2026-06-07

---

## Existing Entities (unchanged)

### Invoice (Rust: `domain::Invoice` / TS: `InvoiceInfo`)

Already persisted in `InvoiceStore` (in-memory, Arc<Mutex>).

| Field | Rust type | TS type | Notes |
|-------|-----------|---------|-------|
| `id` | `Uuid` | `string` | UUID v4 |
| `filename` | `String` | `string` | Original file name |
| `reference_month` | `YearMonth` | serialized as `month: string` | `YYYY-MM` |
| `due_date` | `Option<NaiveDate>` | `string \| undefined` | ISO date |
| `transactions` | `Vec<Transaction>` | — | Not exposed to frontend |
| `imported_at` | `NaiveDateTime` | `string` | ISO datetime |
| `row_count` | computed | `number` | `transactions.len()` |

### MonthlySnapshot (TS: `MonthlySnapshot`)

Returned by `get_dashboard_cmd` in `DashboardData.monthly_trend`.

| Field | Type | Notes |
|-------|------|-------|
| `month` | `string` | `YYYY-MM` key |
| `net_total` | `string` | Decimal string (exact arithmetic) |
| `categories` | `CategorySnapshot[]` | Per-category breakdown |

---

## New Frontend-Only Entities

### MonthGroup (computed, not persisted)

Computed in the Vue store or component from `InvoiceInfo[]` + `MonthlySnapshot[]`.

| Field | Type | Source | Notes |
|-------|------|--------|-------|
| `month` | `string` | `InvoiceInfo.month` key | `YYYY-MM` or `"0000-00"` |
| `label` | `string` | computed | `"Maio 2026"` or `"Mês desconhecido"` |
| `invoices` | `InvoiceInfo[]` | grouped from store | Sorted by `imported_at` desc |
| `net_total` | `string \| null` | `MonthlySnapshot.net_total` | null if month unknown |
| `invoice_count` | `number` | `invoices.length` | |

### MonthFilter (Pinia store state)

| Field | Type | Initial | Notes |
|-------|------|---------|-------|
| `monthFilter` | `string \| null` | `null` | `YYYY-MM` of selected month |

---

## Relationships

```
InvoiceStore (in-memory, Rust)
  └── Invoice[]  ──────────────── list_invoices() ──────> InvoiceInfo[] (TS)
                                                                │
DashboardData.monthly_trend                                     │
  └── MonthlySnapshot[]  ─────── get_dashboard_cmd() ──────┐   │
                                                            ▼   ▼
                                               MonthGroup[] (computed, TS)
                                                            │
                                               monthFilter (Pinia) ──> DashboardFilter.invoice_ids
```

---

## State Transitions

### MonthFilter

```
null
  ──[click month in HistoryPage]──> "YYYY-MM"
  ──[click "Limpar filtro" in DashboardPage OR navigate away]──> null
```

### Invoice lifecycle

```
imported ──[import_invoices()]──> stored
stored   ──[remove_invoice()]──> removed (permanent, no undo)
```

---

## Validation Rules

- `month` key must match `/^\d{4}-\d{2}$/` or be `"0000-00"` (unknown).
- `net_total` is a decimal string; never `NaN` (guarded at render with `parseFloat(x) || 0`).
- `invoice_count` ≥ 1 for any displayed `MonthGroup` (empty groups are never shown).
- `MonthFilter` value must exist in `invoices` array; stale filters (invoice removed) auto-clear.
