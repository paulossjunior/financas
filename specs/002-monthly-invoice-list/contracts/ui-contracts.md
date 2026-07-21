# UI Contracts: Listagem Mensal de Faturas

**Feature**: specs/002-monthly-invoice-list
**Date**: 2026-06-07

All Tauri backend commands are already implemented. Contracts below define the **frontend component interfaces** and **Pinia store additions**.

---

## Tauri Commands (existing, no changes)

| Command | Input | Output | Used by |
|---------|-------|--------|---------|
| `list_invoices` | — | `InvoiceInfo[]` | HistoryPage on mount |
| `remove_invoice` | `{ invoice_id: string }` | `void` | InvoiceRow remove action |
| `get_dashboard_cmd` | `{ filter?: DashboardFilter }` | `DashboardData` | DashboardPage, filtered by month |

### DashboardFilter (existing)
```typescript
interface DashboardFilter {
  invoice_ids?: string[];   // IDs of invoices to include (month filter)
  categories?: string[];
  date_from?: string;
  date_to?: string;
}
```

---

## Pinia Store Additions (`useInvoiceStore`)

New state and actions added to the existing store:

```typescript
// New state
monthFilter: string | null   // "YYYY-MM" or null

// New actions
setMonthFilter(month: string | null): void
  // Sets monthFilter, reloads dashboard with invoice_ids for that month
  // If month is null, clears filter and reloads unfiltered

// Modified action
loadDashboard(): Promise<void>
  // Now passes DashboardFilter when monthFilter is set:
  //   filter = monthFilter
  //     ? { invoice_ids: invoices.filter(i => i.month === monthFilter).map(i => i.id) }
  //     : undefined

// New computed
monthGroups: MonthGroup[]
  // Derived from invoices[] + dashboard.monthly_trend
  // Sorted: most recent first, "0000-00" always last
```

---

## Component Contracts

### `MonthGroup.vue`

```typescript
props: {
  group: MonthGroup       // { month, label, invoices, net_total, invoice_count }
  isActive: boolean       // true if this month is the current monthFilter
}
emits: {
  'filter-month': (month: string) => void    // user clicked month header
  'remove-invoice': (invoiceId: string) => void
}
```

**Renders**: Month header row (label + total + count + "Ver dashboard" button) + collapsible list of `InvoiceRow` components.

---

### `InvoiceRow.vue`

```typescript
props: {
  invoice: InvoiceInfo
}
emits: {
  'remove': (invoiceId: string) => void
}
```

**Renders**: filename, row_count, imported_at, remove button (with native confirm dialog).

---

### `HistoryPage.vue` (refactored)

No props. Uses `useInvoiceStore()`.

**Behavior**:
1. `onMounted` → `store.refreshInvoices()` + `store.loadDashboard()`
2. Renders `MonthGroup` for each group in `store.monthGroups`
3. Handles `filter-month` → `store.setMonthFilter(month)` + `router.push('/')`
4. Handles `remove-invoice` → native confirm → `store.removeInvoice(id)` → refresh

---

### `DashboardPage.vue` (addition: filter badge)

When `store.monthFilter` is set:
- Shows `MonthFilterBadge` below page header: "Filtrado: Mai/2026 · [✕ Limpar]"
- Clicking ✕ → `store.setMonthFilter(null)`

```typescript
// MonthFilterBadge — inline in DashboardPage, no separate component needed
// Visibility: v-if="store.monthFilter"
```

---

## Error Contracts

| Scenario | Expected behavior |
|----------|------------------|
| `remove_invoice` with unknown ID | Tauri returns `"INVOICE_NOT_FOUND"` → mapped to PT-BR message in tauri.service.ts |
| `remove_invoice` while store locked | Tauri returns lock error → generic PT-BR error |
| Month filter with no matching invoices | `invoice_ids: []` → `get_dashboard_cmd` returns `NO_DATA` → store shows error |
| `list_invoices` with empty store | Returns `[]` → HistoryPage shows empty state |
