---
description: "Task list for Gestor Financeiro — Dashboard de Faturas BTG"
---

# Tasks: Gestor Financeiro — Dashboard de Faturas BTG

**Input**: Design documents from `/specs/001-credit-card-dashboard/`

**Prerequisites**: plan.md ✅ spec.md ✅ research.md ✅ data-model.md ✅ contracts/tauri-commands.md ✅

**TDD Mandate**: Constitution Principle I is NON-NEGOTIABLE. All test tasks MUST be written and confirmed failing before their corresponding implementation tasks are started.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1=Import, US2=Dashboard, US3=Maior Gasto, US4=Evolução Mensal)
- Exact file paths in every description

---

## Phase 1: Setup (Project Initialization) — ✅ COMPLETE

**Purpose**: Tauri v2 + Vue 3 scaffold with testing infrastructure

- [X] T001 Initialize Tauri v2 project with Vue 3 + TypeScript template via `cargo tauri init` (creates src-tauri/ and src/)
- [X] T002 [P] Add Rust dependencies in src-tauri/Cargo.toml: `calamine`, `rust_decimal`, `serde`, `serde_json`, `chrono`, `uuid`, `thiserror`, `tokio`, `tauri-plugin-store`, `tauri-plugin-fs`, `tauri-plugin-dialog`
- [X] T003 [P] Add frontend dependencies in package.json: `vue-echarts`, `echarts`, `pinia`, `vue-router`, `@tauri-apps/api`, `vitest`, `@vue/test-utils`, `playwright`, `@tauri-apps/cli`
- [X] T004 [P] Configure Vitest in vite.config.ts: test environment `happy-dom`, coverage provider `v8`, include `src/**/*.test.ts`
- [X] T005 [P] Configure Playwright in playwright.config.ts for Tauri webdriver E2E; add `tests/fixtures/` directory
- [X] T006 [P] Configure TypeScript strict mode in tsconfig.json (`strict: true`, `noImplicitAny: true`)
- [X] T007 [P] Add `faturas/` and `tests/fixtures/*.xlsx` to .gitignore; create `tests/fixtures/` placeholder
- [X] T008 Create full directory structure: `src-tauri/src/{domain,application,infrastructure,commands}/`, `src/{stores,services,types,components/{dashboard,import,shared},pages,__tests__/{components,stores,services}}/`

---

## Phase 2: Foundational (Blocking Prerequisites) — ✅ COMPLETE

**Purpose**: Core types and wiring that ALL user stories depend on.

⚠️ **CRITICAL**: Complete this entire phase before starting Phase 3+

- [X] T009 Create Transaction and InstallmentInfo types in src-tauri/src/domain/transaction.rs with serde Serialize/Deserialize derives
- [X] T010 [P] Create Invoice and YearMonth types in src-tauri/src/domain/invoice.rs with serde derives
- [X] T011 [P] Create DashboardData, DashboardFilter, Category, MonthlySnapshot, CategorySnapshot, TransactionSummary types in src-tauri/src/domain/dashboard.rs
- [X] T012 [P] Create AppConfig and CategoryRule types in src-tauri/src/domain/mod.rs; pub-use all domain types
- [X] T013 Create TypeScript API types in src/types/api.types.ts mirroring all Tauri command input/output shapes from contracts/tauri-commands.md (Decimal as string, dates as string)
- [X] T014 [P] Create Pinia invoice store skeleton in src/stores/invoice.store.ts: empty state shape, placeholder actions, TypeScript typed
- [X] T015 [P] Create Tauri service wrapper skeleton in src/services/tauri.service.ts: typed `invoke` wrappers returning `Promise<T>` for all 6 commands
- [X] T016 Register all Tauri commands in src-tauri/src/lib.rs: `import_invoices`, `get_dashboard`, `list_invoices`, `remove_invoice`, `get_config`, `save_config`
- [X] T017 [P] Wire Pinia and vue-router into src/main.ts; configure App.vue with `<RouterView>` and navigation tabs

**Checkpoint**: `cargo build` passes. `npm run type-check` passes. App launches showing empty shell.

---

## Phase 3: User Story 1 — Importar Fatura BTG (Priority: P1) 🎯 MVP

**Goal**: User selects a decrypted BTG XLSX, clicks import, sees transaction count confirmed.

**Independent Test**: Import `tests/fixtures/sample_fatura.xlsx` → banner shows "X transações importadas" → no errors.

### Tests for User Story 1 ⚠️ Write FIRST — confirm RED before implementing

- [X] T018 [P] [US1] Write unit test in src-tauri/src/infrastructure/btg_mapper.rs: `test_btg_finds_transaction_section` — given BTG multi-section sheet, detects header with auth-code column and parses transactions
- [X] T019 [P] [US1] Write unit test in src-tauri/src/infrastructure/btg_mapper.rs: `test_btg_two_sections_merged` — given two transaction sections, returns transactions from both
- [X] T020 [P] [US1] Write unit test in src-tauri/src/infrastructure/btg_mapper.rs: `test_btg_no_section_returns_error` — given sheet without auth-code column header, returns NoTransactionSection error
- [X] T021 [P] [US1] Write unit test in src-tauri/src/infrastructure/xlsx_parser.rs: `test_encrypted_detection` — given OLE2 magic bytes, is_encrypted returns true
- [X] T022 [P] [US1] Write integration test in src-tauri/tests/xlsx_parser_integration.rs: `test_parse_btg_fixture_returns_transactions` — uses tests/fixtures/sample_fatura.xlsx, expects > 0 transactions with valid dates
- [X] T023 [P] [US1] Write Vitest test in src/__tests__/components/ImportButton.test.ts: mount ImportButton, simulate click, verify `import-requested` event emitted with file paths
- [X] T024 [P] [US1] Write Vitest test in src/__tests__/stores/invoice.store.test.ts: `addInvoice` stores result, `listInvoices` returns it, `removeInvoice` removes it

### Implementation for User Story 1

- [X] T025 [US1] Implement xlsx_parser in src-tauri/src/infrastructure/xlsx_parser.rs: open calamine workbook, detect OLE2 encryption, convert ExcelDateTime to ISO date strings via `as_datetime()`, return all rows including metadata rows
- [X] T026 [US1] Implement btg_mapper in src-tauri/src/infrastructure/btg_mapper.rs: scan all rows for BTG transaction section headers (rows containing Data + Descrição + Código de autorização), build per-section column map, parse transactions from each section, extract installment from description `(N/M)` pattern
- [X] T027 [US1] Implement InvoiceStore in src-tauri/src/application/store.rs: `Arc<Mutex<HashMap<Uuid, Invoice>>>` with add (dedup by filename), get, remove, list methods
- [X] T028 [US1] Implement import_invoice use case in src-tauri/src/application/import_invoice.rs: validate file exists → parse XLSX → map rows → dedup → insert to store → return ImportResult with warnings
- [X] T029 [US1] Implement `import_invoices` Tauri command in src-tauri/src/commands/import.rs: accept `paths: Vec<String>`, call import_invoice for each, return Vec<ImportResult>
- [X] T030 [US1] Create decrypted sample XLSX fixture at tests/fixtures/sample_fatura.xlsx — copy from the real BTG-2.xlsx already in faturas/ (strip personal data or create minimal synthetic XLSX with BTG multi-section layout matching btg_mapper expectations)
- [X] T031 [US1] Implement ImportButton.vue in src/components/import/ImportButton.vue: Tauri dialog.open to select XLSX files, emit `import-requested` with paths, show loading state
- [X] T032 [US1] Implement ImportWarnings.vue in src/components/import/ImportWarnings.vue: display list of ParseWarning (row number + message); collapse if empty
- [X] T033 [US1] Implement `importInvoices()` in src/services/tauri.service.ts: call `invoke('import_invoices', { paths })`, map error strings to user-facing Portuguese messages
- [X] T034 [US1] Implement `addInvoice`, `listInvoices`, `removeInvoice` actions in src/stores/invoice.store.ts
- [X] T035 [US1] Wire ImportButton + ImportWarnings into src/pages/DashboardPage.vue: call store.addInvoice on success, show imported invoice list with filename and row count

**Checkpoint**: Import `tests/fixtures/sample_fatura.xlsx` → invoice appears in list → all T018–T024 tests GREEN.

---

## Phase 4: User Story 2 — Dashboard de Categorias (Priority: P2)

**Goal**: After import, user sees donut chart + ranked bar chart of spending by category with totals and percentages.

**Independent Test**: With sample fixture imported, dashboard shows ≥ 1 category; sum of category `net_total` values equals `DashboardData.net_total`.

### Tests for User Story 2 ⚠️ Write FIRST — confirm RED before implementing

- [X] T036 [P] [US2] Write unit test in src-tauri/src/domain/categorizer.rs: `test_keyword_match_alimentacao` — "IFOOD" → "Alimentação"; `test_keyword_match_transporte` — "UBER" → "Transporte"
- [X] T037 [P] [US2] Write unit test in src-tauri/src/domain/categorizer.rs: `test_no_match_returns_outros` — unknown description → "Outros"
- [X] T038 [P] [US2] Write unit test in src-tauri/src/domain/category.rs: `test_aggregate_two_categories` — given transactions in 2 categories, returns correct totals and percentages
- [X] T039 [P] [US2] Write unit test in src-tauri/src/domain/category.rs: `test_reversals_reduce_net_total` — reversal transaction reduces category net_total
- [X] T040 [P] [US2] Write unit test in src-tauri/src/domain/dashboard.rs: `test_get_dashboard_returns_categories` — returns categories sorted by net_total descending
- [X] T041 [P] [US2] Write Vitest test in src/__tests__/components/CategoryChart.test.ts: mount CategoryChart with mock DashboardData, verify ECharts option has correct series data length
- [X] T042 [P] [US2] Write Vitest test in src/__tests__/components/CategoryRanking.test.ts: mount CategoryRanking, verify first row matches category with highest net_total

### Implementation for User Story 2

- [X] T043 [US2] Implement categorizer in src-tauri/src/domain/categorizer.rs: apply CategoryRule list (sorted by priority) against transaction description using case-insensitive substring match; return first match or "Outros"; 8 default rules seeded at startup
- [X] T044 [US2] Implement category aggregation in src-tauri/src/domain/category.rs: `aggregate_by_category(transactions)` → Vec<Category> sorted by net_total desc; compute percentage from grand total
- [X] T045 [US2] Implement get_dashboard use case in src-tauri/src/application/get_dashboard.rs: apply DashboardFilter → flatten transactions → categorize → aggregate → build DashboardData
- [X] T046 [US2] Implement `get_dashboard` and `list_invoices` Tauri commands in src-tauri/src/commands/dashboard.rs
- [X] T047 [US2] Implement `getDashboard()`, `listInvoices()` in src/services/tauri.service.ts
- [X] T048 [US2] Implement `setDashboard`, `setFilter` actions in src/stores/invoice.store.ts; add `dashboard: DashboardData | null` and `filter: DashboardFilter` to state
- [X] T049 [US2] Implement MoneyAmount.vue in src/components/shared/MoneyAmount.vue: format Decimal string (e.g., "1234.56") as "R$ 1.234,56" using Brazilian locale
- [X] T050 [US2] Implement CategoryChart.vue in src/components/dashboard/CategoryChart.vue: ECharts pie/donut chart with `categories` prop; tooltip showing name, net_total, percentage
- [X] T051 [US2] Implement CategoryRanking.vue in src/components/dashboard/CategoryRanking.vue: ECharts horizontal bar chart ordered by net_total desc; uses MoneyAmount for labels
- [X] T052 [US2] Wire CategoryChart + CategoryRanking + MoneyAmount into src/pages/DashboardPage.vue; auto-load dashboard on invoice import

**Checkpoint**: Dashboard shows categories after import → all T036–T042 tests GREEN.

---

## Phase 5: User Story 3 — Identificar Maior Gasto (Priority: P2)

**Goal**: Biggest expense category highlighted with banner; top 5 individual transactions listed below charts.

**Independent Test**: BiggestSpendBanner shows category matching `categories[0]` from DashboardData; TopTransactions shows 5 rows ordered by amount desc.

### Tests for User Story 3 ⚠️ Write FIRST — confirm RED before implementing

- [X] T053 [P] [US3] Write unit test in src-tauri/src/domain/dashboard.rs: `test_top_transactions_returns_5_largest` — given 10 transactions, top_transactions has 5, first is largest non-reversal
- [X] T054 [P] [US3] Write Vitest test in src/__tests__/components/BiggestSpendBanner.test.ts: mount BiggestSpendBanner with category prop, verify category name and formatted amount render
- [X] T055 [P] [US3] Write Vitest test in src/__tests__/components/TopTransactions.test.ts: mount TopTransactions with 5 transactions, verify 5 rows and first row has largest amount

### Implementation for User Story 3

- [X] T056 [US3] Implement `top_transactions` selection in src-tauri/src/domain/dashboard.rs: filter out reversals, sort by amount desc, take 5, convert to TransactionSummary
- [X] T057 [US3] Implement BiggestSpendBanner.vue in src/components/dashboard/BiggestSpendBanner.vue: accepts `topCategory: Category` prop, displays name, net_total (via MoneyAmount), percentage with distinct highlight styling
- [X] T058 [US3] Implement TopTransactions.vue in src/components/dashboard/TopTransactions.vue: table showing date, description, category, amount (MoneyAmount) for each of the 5 transactions
- [X] T059 [US3] Wire BiggestSpendBanner + TopTransactions into src/pages/DashboardPage.vue below the charts

**Checkpoint**: Dashboard displays highlighted banner and transaction table → all T053–T055 tests GREEN.

---

## Phase 6: User Story 4 — Evolução Mensal (Priority: P3)

**Goal**: When 2+ invoices from different months are imported, History page shows line chart of spending per category over time.

**Independent Test**: Import 2 fixtures from different months → HistoryPage shows line chart with 2 x-axis points.

### Tests for User Story 4 ⚠️ Write FIRST — confirm RED before implementing

- [X] T060 [P] [US4] Write unit test in src-tauri/src/domain/dashboard.rs: `test_monthly_trend_empty_when_single_invoice` — 1 invoice returns empty monthly_trend
- [X] T061 [P] [US4] Write unit test in src-tauri/src/domain/dashboard.rs: `test_monthly_trend_two_months` — 2 invoices from different months return 2 MonthlySnapshot ordered by month asc
- [X] T062 [P] [US4] Write Vitest test in src/__tests__/components/MonthlyTrend.test.ts: mount MonthlyTrend with 2 MonthlySnapshot, verify ECharts xAxis has 2 data points

### Implementation for User Story 4

- [X] T063 [US4] Implement monthly_trend computation in src-tauri/src/domain/dashboard.rs: group invoices by YearMonth, compute CategorySnapshot per month, sort by month asc, return empty Vec if < 2 months
- [X] T064 [US4] Implement MonthlyTrend.vue in src/components/dashboard/MonthlyTrend.vue: ECharts line chart; one series per category; x-axis = months (YYYY-MM formatted as "MMM/YYYY"); tooltip shows month total + categories
- [X] T065 [US4] Create src/pages/HistoryPage.vue: load all invoices (no filter) → get_dashboard → display MonthlyTrend + summary table of totals per month
- [X] T066 [US4] Configure vue-router in src/router/index.ts: routes for `/` (DashboardPage), `/historico` (HistoryPage), `/configuracoes` (SettingsPage); add navigation tabs to App.vue

**Checkpoint**: Import 2 monthly fixtures → History tab shows line chart → all T060–T062 tests GREEN.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Vitest coverage, E2E tests, lint, settings, final validation.

- [X] T067 [P] Implement `remove_invoice` Tauri command in src-tauri/src/commands/dashboard.rs; add `removeInvoice()` wrapper in src/services/tauri.service.ts and `removeInvoice` action in src/stores/invoice.store.ts
- [X] T068 [P] Implement AppConfig persistence in src-tauri/src/infrastructure/config_store.rs using tauri-plugin-store: load on startup, save on change
- [X] T069 [P] Implement `get_config` and `save_config` Tauri commands in src-tauri/src/commands/config.rs
- [X] T070 [P] Create src/pages/SettingsPage.vue: faturas directory picker (Tauri dialog), category rules table with add/edit/delete, save button calling saveConfig()
- [X] T071 [P] Add error boundary in src/App.vue: catch unhandled errors from Tauri commands, display user-friendly Portuguese error messages (never raw Rust errors)
- [X] T072 Write Playwright E2E test in tests/import.spec.ts: launch app → import fixture → verify category chart renders with ≥ 1 category
- [X] T073 [P] Write Playwright E2E test in tests/dashboard.spec.ts: verify BiggestSpendBanner text contains "R$" and category name
- [X] T074 Run `cargo clippy -- -D warnings` in src-tauri/ and fix all linter warnings (currently 13 warnings)
- [X] T075 [P] Run `npm run type-check` and fix all TypeScript errors in src/
- [X] T076 Validate all 4 quickstart.md scenarios manually end-to-end; mark each scenario ✅ in quickstart.md

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: ✅ Complete
- **Foundational (Phase 2)**: ✅ Complete — unblocked all user stories
- **US1 (Phase 3)**: ✅ Implementation done — tests T022–T024 and fixture T030 pending
- **US2 (Phase 4)**: ✅ Implementation done — tests T041–T042 pending
- **US3 (Phase 5)**: ✅ Implementation done — tests T054–T055 pending
- **US4 (Phase 6)**: ✅ Implementation done — test T062 pending
- **Polish (Phase 7)**: Partially done — T071–T076 pending

### Within Each User Story

1. Write tests → confirm RED ← **TDD gate — do not skip**
2. Models / domain types
3. Services / use cases
4. Tauri commands (thin wrappers)
5. Vue service wrappers
6. Pinia store actions
7. Vue components
8. Page wiring

---

## Remaining Work (Priority Order)

### 🔴 High Priority (blocks test validation)

- **T030**: Create `tests/fixtures/sample_fatura.xlsx` — create minimal synthetic XLSX with BTG multi-section layout; needed by T022 and E2E tests
- **T022**: Integration test `src-tauri/tests/xlsx_parser_integration.rs` using the fixture

### 🟡 Medium Priority (Vitest component coverage)

- **T023**: `src/__tests__/components/ImportButton.test.ts`
- **T024**: `src/__tests__/stores/invoice.store.test.ts`
- **T041**: `src/__tests__/components/CategoryChart.test.ts`
- **T042**: `src/__tests__/components/CategoryRanking.test.ts`
- **T054**: `src/__tests__/components/BiggestSpendBanner.test.ts`
- **T055**: `src/__tests__/components/TopTransactions.test.ts`
- **T062**: `src/__tests__/components/MonthlyTrend.test.ts`

### 🟢 Low Priority (polish & quality gates)

- **T071**: Error boundary in App.vue
- **T072–T073**: Playwright E2E tests
- **T074**: `cargo clippy -- -D warnings` (13 current warnings)
- **T075**: `npm run type-check`
- **T076**: Manual quickstart validation

---

## Parallel Opportunities (Remaining)

```
T030 (fixture) → T022 (integration test) → T072–T073 (E2E)
T023 ‖ T024 ‖ T041 ‖ T042 ‖ T054 ‖ T055 ‖ T062  (all Vitest tests — different files)
T074 ‖ T075  (lint passes — independent)
```

---

## Implementation Strategy

### Current State

- **Rust backend**: 24/24 tests passing ✅
- **Vue frontend**: All components implemented ✅
- **App running**: `cargo tauri dev` launches successfully ✅
- **BTG parser**: Multi-section format handled correctly ✅

### Remaining MVP validation

1. Create `tests/fixtures/sample_fatura.xlsx` (T030)
2. Write Vitest component tests (T023–T024, T041–T042, T054–T055, T062)
3. Fix `cargo clippy` warnings (T074)
4. Run `npm run type-check` (T075)
5. Manual end-to-end validation via quickstart.md (T076)

---

## Notes

- `[P]` = different files, no shared state — safe to parallelize
- `[USn]` label maps task to user story for traceability
- Tests MUST be written before implementation — TDD is non-negotiable (Constitution §I)
- Monetary values: always `rust_decimal::Decimal` in Rust, `string` in TypeScript — never `f64` / `number`
- BTG format: two transaction sections detected by presence of `Código de autorização` column; installments in description `(N/M)` pattern
