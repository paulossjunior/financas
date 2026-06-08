# Implementation Plan: Listagem Mensal de Faturas

**Branch**: `002-modern-dashboard-ui` | **Date**: 2026-06-07 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/002-monthly-invoice-list/spec.md`

---

## Summary

Implementar listagem mensal de faturas BTG agrupadas por mês/ano na aba "Histórico", com filtragem do dashboard por mês selecionado e remoção individual de faturas. Todo o backend Tauri (comandos `list_invoices`, `remove_invoice`, `get_dashboard_cmd` com `DashboardFilter`) já está implementado — o trabalho é **exclusivamente frontend** (Vue 3 + Pinia).

---

## Technical Context

**Language/Version**: TypeScript 5.x / Rust 1.80+ (Rust: no changes required)

**Primary Dependencies**:
- Vue 3 + Composition API (existing)
- Pinia (existing)
- vue-router (existing)
- `@tauri-apps/api/core` — `invoke()` (existing)
- `@tauri-apps/plugin-dialog` — `ask()` for remove confirmation (existing, already used by ImportButton)

**Storage**: In-memory `InvoiceStore` (Rust, Arc<Mutex>). No new persistence needed.

**Testing**: Vitest (unit) + Playwright (E2E)

**Target Platform**: macOS desktop (Tauri v2 webview)

**Performance Goals**:
- Invoice list render < 2s (SC-001)
- Filtered dashboard load < 1s after click (SC-002)
- Remove + re-render < 1s (SC-004)

**Constraints**:
- No network calls (Constitution V)
- No backend changes (YAGNI — all Tauri commands already exist)
- ≤ 24 invoice groups (personal use, ~2 years of monthly invoices)

**Scale/Scope**: Single user, desktop app, ≤ 24 `MonthGroup` entries. No pagination needed.

**Project Type**: Desktop app (Tauri v2 + Vue 3 SPA frontend)

---

## Constitution Check

*GATE: All gates must pass before implementation begins.*

| Principle | Gate | Status |
|-----------|------|--------|
| I. TDD | Tests written before implementation code. Vitest unit tests for `monthGroups` computed, `setMonthFilter`. Playwright E2E for all 3 user stories. | ✅ PLANNED |
| II. Clean Architecture | No new Rust layer crossings. Frontend: store (application) → service (infrastructure) → Tauri (external). `MonthGroup` computed lives in store, not component. | ✅ PLANNED |
| III. YAGNI/Simplicity | No new Tauri commands. No new router. `MonthGroup` is a computed, not a persisted entity. | ✅ PASS |
| IV. Data Integrity | `net_total` always parsed with `parseFloat(x) || 0`. Month key validation on assignment. | ✅ PLANNED |
| V. Local-First | Zero network calls. `ask()` dialog is native OS, no external service. | ✅ PASS |

**Complexity Tracking**: No violations. No new backend files.

---

## Project Structure

### Documentation (this feature)

```text
specs/002-monthly-invoice-list/
├── plan.md              ← this file
├── research.md          ✅ complete
├── data-model.md        ✅ complete
├── quickstart.md        ✅ complete
├── contracts/
│   └── ui-contracts.md  ✅ complete
└── tasks.md             ← /speckit-tasks output (next step)
```

### Source Code Changes

```text
src/
├── stores/
│   └── invoice.store.ts          MODIFY — add monthFilter state + setMonthFilter + monthGroups computed
├── pages/
│   ├── HistoryPage.vue           REWRITE — grouped invoice list (replaces trend-only view)
│   └── DashboardPage.vue         MODIFY — add month filter badge + clear button
├── components/
│   └── history/
│       ├── MonthGroup.vue        NEW — month header + collapsible invoice list
│       └── InvoiceRow.vue        NEW — individual invoice row with remove button
└── __tests__/
    ├── stores/
    │   └── invoice.store.test.ts MODIFY — add tests for monthFilter, setMonthFilter, monthGroups
    └── components/
        ├── MonthGroup.test.ts    NEW
        └── InvoiceRow.test.ts    NEW

tests/
└── history.spec.ts               NEW — Playwright E2E for all 3 user stories
```

**No Rust changes required.**

---

## Implementation Strategy

### MVP Scope (US1 only — P1)

Deliver monthly grouping first: `monthGroups` computed in store + `HistoryPage` refactor + `MonthGroup`/`InvoiceRow` components. Independently testable without US2/US3.

### Incremental delivery

1. **Phase 1** (US1): `monthGroups` computed + `HistoryPage` + components + tests
2. **Phase 2** (US2): `monthFilter` state + `setMonthFilter` + DashboardPage badge + E2E
3. **Phase 3** (US3): Remove button + `ask()` confirm + Playwright test

Each phase is independently deployable and testable.

---

## Key Implementation Details

### `monthGroups` computed (Pinia store)

```typescript
const monthGroups = computed<MonthGroup[]>(() => {
  const groups = new Map<string, InvoiceInfo[]>();
  for (const inv of invoices.value) {
    const key = inv.month || "0000-00";
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(inv);
  }
  const trend = dashboard.value?.monthly_trend ?? [];
  const trendMap = new Map(trend.map(s => [s.month, s.net_total]));

  return [...groups.entries()]
    .map(([month, invs]) => ({
      month,
      label: formatMonthLabel(month),
      invoices: invs.sort((a, b) => b.imported_at.localeCompare(a.imported_at)),
      net_total: trendMap.get(month) ?? null,
      invoice_count: invs.length,
    }))
    .sort((a, b) => {
      if (a.month === "0000-00") return 1;
      if (b.month === "0000-00") return -1;
      return b.month.localeCompare(a.month); // descending
    });
});
```

### `setMonthFilter` action

```typescript
async function setMonthFilter(month: string | null): Promise<void> {
  monthFilter.value = month;
  await loadDashboard();
}
```

### `loadDashboard` modification

```typescript
async function loadDashboard(): Promise<void> {
  const filterInvoiceIds = monthFilter.value
    ? invoices.value.filter(i => i.month === monthFilter.value).map(i => i.id)
    : undefined;
  const filter = filterInvoiceIds ? { invoice_ids: filterInvoiceIds } : undefined;
  dashboard.value = await getDashboard(filter);
}
```

### Auto-clear stale filter

After `removeInvoice(id)`: if `monthFilter` is set but no invoices remain in that month, set `monthFilter = null`.

```typescript
async function removeInvoice(id: string): Promise<void> {
  await removeInvoiceService(id);
  await refreshInvoices();
  const stillExists = invoices.value.some(i => i.month === monthFilter.value);
  if (!stillExists) monthFilter.value = null;
  await loadDashboard();
}
```
