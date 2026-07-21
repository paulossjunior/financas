# Research: Listagem Mensal de Faturas

**Date**: 2026-06-07
**Feature**: specs/002-monthly-invoice-list

---

## Decision 1: Source of per-month totals

**Decision**: Join `MonthlySnapshot[]` (from `DashboardData.monthly_trend`) with `InvoiceInfo[]` (from `list_invoices`) in the frontend. Month key = `YYYY-MM` string.

**Rationale**: `InvoiceInfo` (Rust `InvoiceInfo` struct + `list_invoices` command) does not expose `net_total` per invoice. Adding it would require Rust changes. `monthly_trend` already provides `net_total` per month via the existing `get_dashboard_cmd`. Joining in the frontend avoids backend changes (YAGNI) and keeps the domain layer stable.

**Alternatives considered**:
- Add `net_total` to `InvoiceInfo` in Rust — cleaner data model but unnecessary backend change for this feature.
- Call `get_dashboard_cmd` once per month group — N+1 Tauri invocations, bad performance.

---

## Decision 2: Month filter mechanism

**Decision**: Add `monthFilter: string | null` (YYYY-MM format) to the Pinia `useInvoiceStore`. When set, `loadDashboard()` passes `{ invoice_ids: invoicesForMonth }` as `DashboardFilter`. The frontend computes `invoicesForMonth` from the in-memory invoice list already loaded by `refreshInvoices()`.

**Rationale**: `DashboardFilter.invoice_ids` already exists in both the TypeScript types and the Rust command signature. No new Tauri commands needed. The store already holds `invoices: InvoiceInfo[]` which can be filtered client-side.

**Alternatives considered**:
- New Tauri command `get_dashboard_for_month(month: string)` — redundant, existing filter covers this.
- URL query param `?month=2026-05` — overkill for a desktop app with no deep-linking requirement.

---

## Decision 3: Remove confirmation

**Decision**: Use `ask()` from `@tauri-apps/plugin-dialog` for native OS confirmation dialog before calling `remove_invoice`.

**Rationale**: `@tauri-apps/plugin-dialog` is already installed (used by `ImportButton`). `ask()` returns a boolean. Native dialog is more appropriate than a custom Vue modal for a destructive action in a desktop app.

**Alternatives considered**:
- Inline confirmation row (expand to show "Are you sure?" + buttons) — more complex, adds interaction state.
- Browser `window.confirm()` — works in Tauri but not native-looking.

---

## Decision 4: HistoryPage redesign

**Decision**: Replace the current `HistoryPage.vue` (which shows `MonthlyTrend` chart + a plain table) with a grouped invoice list — month headers with total + N faturas, expandable rows per invoice with remove button. The `MonthlyTrend` chart is already visible on the DashboardPage; it does not need to be duplicated in HistoryPage.

**Rationale**: The spec defines HistoryPage as the invoice listing view (US1 = grouping, US3 = remove). The trend chart belongs to the dashboard analysis, not the history listing.

**Alternatives considered**:
- Keep trend chart in HistoryPage + add invoice list below — redundant with DashboardPage, clutters the view.

---

## Decision 5: "Mês desconhecido" group

**Decision**: Invoices where `month` cannot be parsed (month = `"0000-00"` or empty) are grouped under a sentinel key `"0000-00"` displayed as "Mês desconhecido" and sorted to the bottom of the list.

**Rationale**: Spec FR-009 requires this grouping. The Rust `btg_mapper` already falls back to a default `YearMonth::new(0, 0)` for unparseable months (or would need a small guard — verify in implementation).

---

## Decision 6: No persistence of filter across navigation

**Decision**: Month filter is cleared when the user navigates away from DashboardPage, or explicitly via a "Limpar filtro" button. It is not persisted to localStorage.

**Rationale**: Simple, stateless — user returns to full view on re-entry. The filter is a session-level convenience, not a saved preference.
