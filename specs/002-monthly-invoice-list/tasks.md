# Tasks: Listagem Mensal de Faturas

**Input**: Design documents from `specs/002-monthly-invoice-list/`

**Constitution**: TDD mandatory — tests MUST be written first and MUST fail before implementation code.

**Scope**: 100% frontend (Vue 3 + Pinia + TypeScript). No Rust changes needed.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no cross-dependencies)
- **[Story]**: Which user story (US1/US2/US3)

---

## Phase 1: Setup

**Purpose**: Create component directory and shared type for new feature.

- [X] T001 Create directory `src/components/history/` (mkdir only)
- [X] T002 Add `MonthGroup` interface to `src/types/api.types.ts`:
  ```typescript
  export interface MonthGroup {
    month: string;        // "YYYY-MM" or "0000-00"
    label: string;        // "Maio 2026" or "Mês desconhecido"
    invoices: InvoiceInfo[];
    net_total: string | null;
    invoice_count: number;
  }
  ```

**Checkpoint**: Types compile cleanly — `npm run type-check` passes.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Extend Pinia store with `monthFilter` state + `monthGroups` computed + `setMonthFilter` action. All user stories depend on these.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tests first (TDD) — write these BEFORE modifying the store

- [X] T003 Add failing unit tests for `monthGroups` computed in `src/__tests__/stores/invoice.store.test.ts`:
  - `monthGroups groups invoices by month, sorted descending`
  - `monthGroups joins net_total from monthly_trend`
  - `monthGroups puts "0000-00" month at end`
  - `monthGroups returns empty array when no invoices`

- [X] T004 Add failing unit tests for `setMonthFilter` action in `src/__tests__/stores/invoice.store.test.ts`:
  - `setMonthFilter sets monthFilter and triggers dashboard reload with invoice_ids filter`
  - `setMonthFilter(null) clears filter and reloads unfiltered dashboard`

### Implementation

- [X] T005 Extend `src/stores/invoice.store.ts`:
  - Add `monthFilter: ref<string | null>(null)`
  - Add `monthGroups: computed<MonthGroup[]>()` — groups `invoices` by `month`, joins `net_total` from `dashboard.monthly_trend`, sorts descending (unknown month last)
  - Add `setMonthFilter(month: string | null): Promise<void>` — sets `monthFilter`, calls `loadDashboard()`
  - Modify `loadDashboard()` to pass `DashboardFilter.invoice_ids` when `monthFilter` is set
  - Export `monthFilter` and `monthGroups` from store

**Checkpoint**: Run `npm run test` — all 6 new store tests must pass.

---

## Phase 3: User Story 1 — Listar faturas agrupadas por mês (Priority: P1) 🎯 MVP

**Goal**: HistoryPage shows all invoices grouped by month, sorted most-recent-first, with total per month.

**Independent Test**: Import 3 invoices across 2 months → Histórico tab shows 2 month groups, correct totals, individual file rows.

### Tests first (TDD)

- [X] T006 [P] [US1] Create `src/__tests__/components/InvoiceRow.test.ts`:
  - `renders invoice filename`
  - `renders row_count and imported_at formatted as dd/mm/yyyy`
  - `emits "remove" event with invoiceId when remove button clicked`
  - `remove button is present and enabled`

- [X] T007 [P] [US1] Create `src/__tests__/components/MonthGroup.test.ts`:
  - `renders month label and net_total formatted as R$`
  - `renders invoice_count badge`
  - `renders one InvoiceRow per invoice in group`
  - `emits "filter-month" with month string when "Ver dashboard" button clicked`
  - `emits "remove-invoice" with id when InvoiceRow emits "remove"`
  - `shows "—" for net_total when null (unknown month group)`

### Implementation

- [X] T008 [P] [US1] Create `src/components/history/InvoiceRow.vue`:
  - Props: `{ invoice: InvoiceInfo }`
  - Emits: `{ remove: (invoiceId: string) => void }`
  - Renders: filename, row_count ("N transações"), imported_at (dd/mm/yyyy format)
  - Remove button: Fluent style (red, small, icon "🗑" or "✕")
  - On click remove: emits `remove` with `invoice.id` (confirmation handled by parent)

- [X] T009 [P] [US1] Create `src/components/history/MonthGroup.vue`:
  - Props: `{ group: MonthGroup, isActive: boolean }`
  - Emits: `{ 'filter-month': (month: string) => void, 'remove-invoice': (invoiceId: string) => void }`
  - Header: month label, invoice_count badge, net_total (R$ or "—"), "Ver dashboard" button
  - Active state: highlight border/bg when `isActive`
  - List: `InvoiceRow` for each `group.invoices`
  - Fluent Design styling (`.card`, `--clr-*` tokens from App.vue)

- [X] T010 [US1] Rewrite `src/pages/HistoryPage.vue`:
  - `onMounted`: `store.refreshInvoices()` → `store.loadDashboard()`
  - Renders `MonthGroup` for each entry in `store.monthGroups`
  - Passes `isActive="store.monthFilter === group.month"`
  - Handles `filter-month`: `store.setMonthFilter(month)` → `router.push('/')`
  - Handles `remove-invoice`: calls `ask()` from `@tauri-apps/plugin-dialog` for confirm → `store.removeInvoice(id)`
  - After `removeInvoice`: checks if `monthFilter` still valid (store handles auto-clear)
  - Empty state when `store.invoices.length === 0`: "Nenhuma fatura importada" message
  - Loading shimmer while `store.loading`
  - Page header: "Histórico" title + `ImportButton` (same pattern as DashboardPage)
  - Fluent Design layout (same `.page`, `.card` structure)

**Checkpoint**: Navigate to Histórico with real invoices → grouped list renders correctly. `npm run test` still passes.

---

## Phase 4: User Story 2 — Ver detalhes de um mês específico (Priority: P2)

**Goal**: Click "Ver dashboard" on any month group → navigate to DashboardPage showing only that month's data, with a visible filter badge.

**Independent Test**: Click "Fev/2026" group → dashboard KPIs match Fev/2026 totals. Click "✕ Limpar" → full totals restore.

### Tests first (TDD)

- [X] T011 [US2] Add failing unit tests in `src/__tests__/stores/invoice.store.test.ts`:
  - `loadDashboard passes invoice_ids filter when monthFilter is set`
  - `loadDashboard passes no filter when monthFilter is null`

- [X] T012 [US2] Create `tests/history.spec.ts` Playwright E2E — US2 scenario:
  - Mock `list_invoices` with invoices across 2 months
  - Mock `get_dashboard_cmd`: return month-specific data when `filter.invoice_ids` matches, full data otherwise
  - Click "Ver dashboard" on first month group
  - Assert: URL is `/`, filter badge shows "Filtrado: [Month] · ✕ Limpar"
  - Assert: KPI net_total matches mock for that month
  - Click "✕ Limpar" → assert: badge gone, net_total = full total

### Implementation

- [X] T013 [US2] Add month filter badge to `src/pages/DashboardPage.vue`:
  - `v-if="store.monthFilter"` block below page header
  - Shows: "Filtrado: [formatted month] · " + "✕ Limpar" button
  - "✕ Limpar" → `store.setMonthFilter(null)`
  - Fluent style: small pill badge with accent border, same as `period-badge`

**Checkpoint**: Click month in Histórico → Dashboard filters correctly. Clear button restores full view. `npm run test` passes.

---

## Phase 5: User Story 3 — Remover fatura da listagem (Priority: P3)

**Goal**: Each invoice row has a remove button. Clicking it shows native confirmation dialog. On confirm, invoice is permanently removed, totals recalculate, empty month groups disappear.

**Independent Test**: Remove last invoice in a month group → entire group disappears. Remove one of two invoices → group remains with updated total.

### Tests first (TDD)

- [X] T014 [US3] Add failing unit tests in `src/__tests__/stores/invoice.store.test.ts`:
  - `removeInvoice removes invoice from store and reloads dashboard`
  - `removeInvoice clears monthFilter when removed invoice was the last in filtered month`
  - `removeInvoice keeps monthFilter when other invoices remain in that month`

- [X] T015 [US3] Add failing unit test in `src/__tests__/components/MonthGroup.test.ts`:
  - `propagates remove-invoice event from InvoiceRow to parent`

- [X] T016 [US3] Add US3 scenarios to `tests/history.spec.ts` Playwright E2E:
  - Mock `remove_invoice` to return void
  - Accept dialog: `page.once('dialog', d => d.accept())`
  - Click remove on first invoice in a 2-invoice group
  - Assert: that invoice row no longer in DOM
  - Assert: `remove_invoice` was called with correct ID
  - Add scenario: remove last invoice in group → group header disappears

### Implementation

- [X] T017 [US3] Extend `src/stores/invoice.store.ts` `removeInvoice` action:
  - After `removeInvoiceService(id)` + `refreshInvoices()`: check if `monthFilter` still has matching invoices
  - If no invoices remain for current `monthFilter`: set `monthFilter.value = null`
  - Call `loadDashboard()` to recalculate

- [X] T018 [US3] Update `src/components/history/InvoiceRow.vue` remove button:
  - Confirm that remove button emits `'remove'` with `invoice.id` (already done in T008; verify no changes needed)
  - If `ask()` call was kept in `HistoryPage.vue` (T010), no changes needed here

**Checkpoint**: Remove flows work end-to-end. Auto-clear of stale monthFilter works. All Vitest + Playwright tests pass.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T019 [P] Verify Fluent Design consistency: MonthGroup and InvoiceRow use same `--clr-*` tokens, `--radius-lg`, `--shadow-sm` as DashboardPage cards
- [X] T020 [P] Add empty-state for HistoryPage when `store.invoices.length === 0` (matches DashboardPage empty state pattern)
- [X] T021 Verify "Mês desconhecido" group (month = "0000-00") renders label correctly and appears last in list
- [X] T022 Run full Playwright suite: `npx playwright test` — all tests pass (import.spec.ts, dashboard.spec.ts, history.spec.ts)
- [X] T023 Run `npm run type-check` — zero TypeScript errors
- [X] T024 Validate quickstart.md scenarios 1–4 manually in native Tauri window (`npx tauri dev`)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1** (Setup): No dependencies — start immediately
- **Phase 2** (Foundational): Depends on Phase 1 — blocks all user stories
- **Phase 3** (US1): Depends on Phase 2 — no dependency on US2/US3
- **Phase 4** (US2): Depends on Phase 2 — requires `monthFilter` from store (Phase 2)
- **Phase 5** (US3): Depends on Phase 2 — `InvoiceRow` already has remove button from Phase 3 (T008), but US3 adds confirm + auto-clear
- **Phase 6** (Polish): Depends on all story phases complete

### User Story Dependencies

- **US1** blocks nothing (standalone invoice list)
- **US2** depends on `monthFilter` in store (Phase 2) — independently testable from US1
- **US3** reuses `InvoiceRow.vue` from US1 (T008) — builds on US1 but independently testable

### Parallel Opportunities

- T006 + T007 (tests for InvoiceRow + MonthGroup) — different files, run in parallel
- T008 + T009 (create InvoiceRow + MonthGroup components) — different files, run in parallel
- T003 + T004 (store tests) — same file, must be sequential within Phase 2

---

## Parallel Example: Phase 3 (US1)

```bash
# Tests (run first — must fail):
T006: InvoiceRow.test.ts
T007: MonthGroup.test.ts   ← parallel with T006

# Components (run after tests are red):
T008: InvoiceRow.vue
T009: MonthGroup.vue        ← parallel with T008

# Page (depends on both components):
T010: HistoryPage.vue       ← after T008 + T009
```

---

## Implementation Strategy

### MVP (US1 only — Phases 1–3)

1. Phase 1: Setup types
2. Phase 2: Store extension (monthGroups)
3. Phase 3: InvoiceRow + MonthGroup + HistoryPage
4. **STOP**: Validate grouped list works with real invoices

### Incremental

1. MVP (US1) → grouped list works
2. US2 → filter badge works
3. US3 → remove works with confirm + auto-clear

### TDD cycle per task

Red → Green → Refactor. No implementation file before its test is written and failing.

---

## Notes

- `ask()` import: `import { ask } from "@tauri-apps/plugin-dialog"` — already in `package.json`
- `formatMonthLabel(month: string)` helper: `"2026-05"` → `"Maio 2026"`, `"0000-00"` → `"Mês desconhecido"` — implement in store or shared util
- Month label format for filter badge in DashboardPage: same `formatPeriod()` helper already in `DashboardPage.vue` can be reused or extracted to a shared util
- Constitution check: all tasks follow Red→Green→Refactor. Test tasks (T003, T004, T006, T007, T011, T012, T014–T016) must be written first and verified failing before their corresponding implementation tasks
