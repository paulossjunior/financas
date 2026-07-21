# Tasks: Categorias Personalizadas de Despesas

**Input**: Design documents from `specs/003-custom-categories/`

**Constitution**: TDD mandatory — tests MUST be written first and MUST fail before implementation code.

**Scope**: Rust backend (new command + domain changes) + Vue 3 / Pinia frontend (Settings UI + Dashboard override).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no cross-dependencies)
- **[Story]**: Which user story (US1/US2/US3)

---

## Phase 1: Setup

**Purpose**: Add new AppConfig field and TypeScript types. No logic yet.

- [X] T001 Add `transaction_overrides: HashMap<String, String>` with `#[serde(default)]` to `AppConfig` struct in `src-tauri/src/domain/mod.rs` (add `use std::collections::HashMap;` import)
- [X] T002 [P] Add `transaction_overrides: Record<string, string>` to `AppConfig` interface and add `CategoryGroup` interface (`{ name: string; keywords: string[]; priority: number }`) in `src/types/api.types.ts`
- [X] T003 [P] Add `recategorizeInvoices(): Promise<number>` service function using `invoke<number>("recategorize_invoices")` in `src/services/tauri.service.ts`

**Checkpoint**: `cargo test` passes (new field deserializes cleanly). `npm run type-check` passes.

---

## Phase 2: Foundational — Backend Core (blocks all stories)

**Purpose**: Deterministic Invoice ID + override application in import + `recategorize_invoices` use case. All TDD.

**⚠️ CRITICAL**: No frontend story work begins until this phase is complete.

### Tests first (TDD) — write these BEFORE modifying source

- [X] T004 Add failing Rust test `invoice_id_deterministic_from_filename` in `src-tauri/src/domain/invoice.rs` `#[cfg(test)]` block: assert same filename produces same `Invoice::new(filename, ...).id` across two calls, and different filenames produce different IDs.

- [X] T005 Add failing Rust test `appconfig_deserializes_without_overrides_field` in `src-tauri/src/domain/mod.rs` `#[cfg(test)]` block: deserialize `{"faturas_directory":"f","category_rules":[]}` (no `transaction_overrides`) — assert result has empty `transaction_overrides`.

- [X] T006 Add failing Rust test `recategorize_applies_rules_and_overrides` in `src-tauri/src/application/recategorize.rs` `#[cfg(test)]` block:
  - Build `InvoiceStore` with one invoice (two transactions: one matching a rule, one with an override)
  - Call `recategorize_invoices(&store, &config)` 
  - Assert: rule-matched transaction gets new category; override transaction gets override category; returns correct changed count.

- [X] T007 Add failing Rust test `import_applies_overrides_after_categorization` in `src-tauri/src/application/import_invoice.rs` `#[cfg(test)]` block:
  - Build `AppConfig` with one `transaction_override` keyed to the deterministic transaction ID for a specific row
  - Call `import_invoice` — assert that transaction's category equals the override value, not the rule result.

### Implementation

- [X] T008 Change `Invoice::new()` in `src-tauri/src/domain/invoice.rs` to generate `id` as `Uuid::new_v5(Uuid::NAMESPACE_URL, filename.as_bytes())` instead of `Uuid::new_v4()`. Add `use uuid::Uuid;` if not present. Run T004 — must pass.

- [X] T009 Create `src-tauri/src/application/recategorize.rs` with `pub fn recategorize_invoices(store: &SharedStore, config: &AppConfig) -> usize`:
  - Lock store, lock config
  - Build `Categorizer::new(config.category_rules.clone())` (or `with_defaults()` if empty)
  - Iterate all invoices → all transactions: compute new category via categorizer; if override exists in `config.transaction_overrides` for tx.id, use override value instead; if changed, update tx.category and increment counter
  - Return counter
  Run T006 — must pass.

- [X] T010 Add `pub mod recategorize;` to `src-tauri/src/application/mod.rs`.

- [X] T011 Apply overrides in `src-tauri/src/application/import_invoice.rs`: after `map_sheet_to_transactions`, iterate transactions and apply `config.transaction_overrides` lookup by `tx.id.to_string()`. Run T007 — must pass.

- [X] T012 Create `src-tauri/src/commands/categories.rs` with three Tauri commands:
  - `recategorize_invoices_cmd(store: State<SharedStore>, config: State<Mutex<AppConfig>>) -> Result<usize, String>`: calls `application::recategorize::recategorize_invoices`
  - `override_transaction_category(transaction_id: String, category: String, config: State<Mutex<AppConfig>>, store: State<SharedStore>, app: AppHandle) -> Result<(), String>`: validates non-empty category, saves override to AppConfig, persists via ConfigStore, updates in-memory transaction category
  - `remove_transaction_override(transaction_id: String, config: State<Mutex<AppConfig>>, store: State<SharedStore>, app: AppHandle) -> Result<(), String>`: removes override from AppConfig, persists, re-categorizes that transaction using current rules

- [X] T013 Add `pub mod categories;` to `src-tauri/src/commands/mod.rs`.

- [X] T014 Register the three new commands in `src-tauri/src/lib.rs`:
  - Import: `commands::categories::{recategorize_invoices_cmd, override_transaction_category, remove_transaction_override}`
  - Add to `invoke_handler!` macro

**Checkpoint**: `cargo test` — all Rust tests pass (T004, T005, T006, T007 all green). `cargo build` succeeds.

---

## Phase 3: User Story 1 — Gerenciar Categorias (Priority: P1) 🎯 MVP

**Goal**: User can create, rename, and delete expense categories via the Settings page. Changes persist to `config.json`. Invoices in session are re-categorized automatically after save.

**Independent Test**: Open Settings → create "Pets" category → save → close and reopen → "Pets" still appears in list. Delete "Pets" → confirm → gone.

### Tests first (TDD) — write these BEFORE implementation

- [X] T015 Create `src/__tests__/stores/settings.store.test.ts` with failing tests:
  - `categoryGroups returns grouped CategoryGroups from config.category_rules`
  - `categoryGroups returns default rules pre-filled when category_rules is empty`
  - `addCategory appends new CategoryGroup with empty keywords and priority`
  - `deleteCategory removes all rules for that category name`
  - `renameCategory updates category string in all matching rules`
  - `saveCategories calls saveConfig then recategorizeInvoices and updates store`

- [X] T016 [P] Create `src/__tests__/components/CategoryList.test.ts` with failing tests:
  - `renders one row per CategoryGroup`
  - `add button emits add-category event`
  - `delete button emits delete-category with category name`
  - `rename input emits rename-category with old and new name`

### Implementation

- [X] T017 Create `src/stores/settings.store.ts` — Pinia store:
  - State: `config: ref<AppConfig>({ faturas_directory: 'faturas', category_rules: [], transaction_overrides: {} })`
  - Computed `categoryGroups: CategoryGroup[]`: group `config.category_rules` by `category` name; if `config.category_rules` is empty, pre-fill with the 8 default rule groups (Alimentação, Transporte, Saúde, Lazer, Compras Online, Educação, Viagem, Moradia) as editable UI state (do NOT write to config yet)
  - Actions: `loadConfig()`, `addCategory(name)`, `deleteCategory(name)`, `renameCategory(oldName, newName)`, `saveCategories()` (calls `saveConfig` then `recategorizeInvoices`)
  Run T015 — must pass.

- [X] T018 [P] Create `src/components/settings/CategoryList.vue`:
  - Props: `{ groups: CategoryGroup[] }`
  - Emits: `{ 'add-category': () => void, 'delete-category': (name: string) => void, 'rename-category': (oldName: string, newName: string) => void }`
  - Renders one row per group: name label, inline rename input (on double-click), delete button
  - "+ Nova Categoria" button at bottom
  - Fluent Design styling (`--clr-*` tokens, `--radius-lg`, `--shadow-sm`)
  Run T016 — must pass.

- [X] T019 [US1] Update `src/pages/SettingsPage.vue`:
  - Import and initialize `useSettingsStore`; call `settingsStore.loadConfig()` on `onMounted`
  - Add "Categorias & Regras" section with `CategoryList` component after existing "Importação" section
  - Wire events from `CategoryList` to `settingsStore.addCategory/deleteCategory/renameCategory`
  - On save button click: call `settingsStore.saveCategories()` (replaces current `handleSave`)
  - Show success/error feedback (same pattern as existing `saved`/`error` refs)

- [X] T020 [P] [US1] Write failing Playwright E2E for US1 in `tests/categories.spec.ts`:
  - Mock `get_config`, `save_config` (capture payload), `recategorize_invoices` (return 3)
  - Test: navigate to `/configuracoes` → add "Pets" category → verify in list
  - Test: rename "Pets" to "Animais" → verify updated in list
  - Test: delete "Animais" → verify gone from list
  - Test: save → verify `save_config` called with correct `category_rules` payload
  - Test: default categories pre-filled when `category_rules` is empty

**Checkpoint**: All US1 Playwright tests pass. `npm run test` passes. `npm run type-check` zero errors.

---

## Phase 4: User Story 2 — Regras de Categorização por Palavras-Chave (Priority: P2)

**Goal**: User can add/remove keywords per category. Conflict detection warns when keyword already used by another category. After save, existing in-session invoices are re-categorized.

**Independent Test**: Add keyword "COBASI" to "Pets" → save → mock invoice with "COBASI" transaction → transaction categorized as "Pets" in Dashboard.

### Tests first (TDD) — write these BEFORE implementation

- [X] T021 [P] Create `src/__tests__/utils/category-conflict.test.ts` with failing tests:
  - `detectConflict returns null when keyword not used by any category`
  - `detectConflict returns existing category name when keyword is duplicate`
  - `detectConflict is case-insensitive`
  - `detectConflict ignores current category when checking (rename-in-place scenario)`

- [X] T022 [P] Create `src/__tests__/components/CategoryGroupEditor.test.ts` with failing tests:
  - `renders category name and keyword chips`
  - `add keyword input shows conflict warning when keyword exists in other category`
  - `add keyword emits update event with new keyword appended`
  - `remove keyword chip emits update event with keyword removed`
  - `emits delete event when delete button clicked`

- [X] T023 [P] Add failing unit tests for keyword operations to `src/__tests__/stores/settings.store.test.ts`:
  - `addKeyword appends keyword to matching category group`
  - `removeKeyword removes keyword from matching category group`
  - `getConflict returns category name if keyword exists in another category`

### Implementation

- [X] T024 [P] [US2] Create `src/utils/category-conflict.ts`:
  - Export `detectConflict(keyword: string, currentCategoryName: string, groups: CategoryGroup[]): string | null`
  - Returns name of conflicting category (or null if no conflict). Case-insensitive comparison. Ignores `currentCategoryName` group.
  Run T021 — must pass.

- [X] T025 [P] [US2] Create `src/components/settings/CategoryGroupEditor.vue`:
  - Props: `{ group: CategoryGroup, allKeywords: { keyword: string; category: string }[] }`
  - Emits: `{ update: (group: CategoryGroup) => void, delete: (name: string) => void }`
  - Renders: keyword chips (removable), text input to add new keyword with inline conflict warning (`detectConflict`), delete button
  - Fluent Design styling
  Run T022 — must pass.

- [X] T026 [US2] Add `addKeyword`, `removeKeyword`, `getConflict` actions to `src/stores/settings.store.ts` and integrate `CategoryGroupEditor` into `CategoryList.vue` (each row expands to show `CategoryGroupEditor`). Run T023 — must pass.

- [X] T027 [US2] Add US2 Playwright scenarios to `tests/categories.spec.ts`:
  - Mock `get_config` returning rules with "Pets" category, `save_config`, `recategorize_invoices`
  - Test: add "COBASI" keyword to "Pets" → verify appears in chips
  - Test: try to add "IFOOD" (already in "Alimentação") → conflict warning shown
  - Test: remove keyword chip → gone from list
  - Test: save → `save_config` payload includes updated keywords

**Checkpoint**: All US1 + US2 Playwright tests pass. `npm run test` passes.

---

## Phase 5: User Story 3 — Reclassificação Manual de Transação (Priority: P3)

**Goal**: User can manually override a transaction's category from the Dashboard. Override persists across sessions and app restarts. User can remove override to restore automatic categorization.

**Independent Test**: Find transaction "AMAZON MKTPL" → change to "Educação" → reload Dashboard → still shows "Educação" → remove override → reverts to auto-category.

### Tests first (TDD) — write these BEFORE implementation

- [X] T028 [P] Add failing service function types to `src/services/tauri.service.ts`:
  - `overrideTransactionCategory(transactionId: string, category: string): Promise<void>`
  - `removeTransactionOverride(transactionId: string): Promise<void>`

- [X] T029 [P] Create `src/__tests__/components/TransactionCategoryOverride.test.ts` with failing tests:
  - `renders current category`
  - `dropdown shows all available category names`
  - `selecting different category emits override event`
  - `shows override indicator badge when hasOverride is true`
  - `remove-override button emits removeOverride event when hasOverride is true`

### Implementation

- [X] T030 [P] [US3] Create `src/components/settings/TransactionCategoryOverride.vue`:
  - Props: `{ transactionId: string, currentCategory: string, availableCategories: string[], hasOverride: boolean }`
  - Emits: `{ override: (transactionId: string, category: string) => void, removeOverride: (transactionId: string) => void }`
  - Renders: category `<select>` or dropdown with `availableCategories`, small "✏" badge if `hasOverride`, "Restaurar automático" link visible when `hasOverride`
  - Fluent Design styling
  Run T029 — must pass.

- [X] T031 [US3] Integrate `TransactionCategoryOverride.vue` into Dashboard category detail transaction rows (`src/pages/DashboardPage.vue` or relevant category card component):
  - For each `TransactionSummary` in a category's `top_transactions`, render override component
  - On `override` event: call `overrideTransactionCategory(id, category)` service → reload dashboard
  - On `removeOverride` event: call `removeTransactionOverride(id)` service → reload dashboard
  - Pass `hasOverride` by checking `invoiceStore.config.transaction_overrides[tx.id]`

- [X] T032 [US3] Add US3 Playwright scenarios to `tests/categories.spec.ts`:
  - Mock `override_transaction_category` (return null), `remove_transaction_override` (return null), `get_config` (with one override in transaction_overrides), `get_dashboard_cmd`
  - Test: select "Educação" for transaction → `override_transaction_category` called with correct args
  - Test: transaction shows override badge when `hasOverride = true`
  - Test: click "Restaurar automático" → `remove_transaction_override` called

**Checkpoint**: All US1 + US2 + US3 Playwright tests pass. `npm run test` passes.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T033 [P] Run `cargo test` — all Rust tests pass (T004, T005, T006, T007 green + all prior tests)
- [X] T034 [P] Run `npm run type-check` — zero TypeScript errors
- [X] T035 [P] Run `npm run test` — all Vitest tests pass (settings.store, CategoryList, CategoryGroupEditor, TransactionCategoryOverride, category-conflict utility)
- [X] T036 Verify Fluent Design consistency: `CategoryList.vue`, `CategoryGroupEditor.vue`, `TransactionCategoryOverride.vue` use `--clr-*` CSS custom properties and `--radius-lg`/`--shadow-sm` tokens consistent with existing cards
- [X] T037 Run `npx playwright test` — all test suites pass (import.spec.ts, dashboard.spec.ts, history.spec.ts, categories.spec.ts)
- [X] T038 Validate quickstart.md scenarios 1–5 manually in `npx tauri dev`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1** (Setup): No dependencies — start immediately
- **Phase 2** (Backend Core): Depends on Phase 1 — BLOCKS all user stories
- **Phase 3** (US1): Depends on Phase 2 backend commands registered + TypeScript types from Phase 1
- **Phase 4** (US2): Depends on Phase 3 settings store (adds keyword actions to same store)
- **Phase 5** (US3): Depends on Phase 2 backend commands (override command) + Phase 3 settings store for available categories
- **Phase 6** (Polish): Depends on all story phases complete

### User Story Dependencies

- **US1 (P1)** blocks US2 and US3 on the frontend (settings store is shared)
- **US2 (P2)** extends US1 settings store and category editor
- **US3 (P3)** is independent of US1/US2 UI — depends only on Phase 2 backend

### Parallel Opportunities (within phases)

- T001, T002, T003 — different files, run in parallel (Phase 1)
- T004, T005 — different files, run in parallel (Phase 2 tests)
- T015, T016 — different files, run in parallel (Phase 3 tests)
- T020 — Playwright test write, parallel with T017–T019 component implementation
- T021, T022, T023 — different files, run in parallel (Phase 4 tests)
- T024, T025 — different files, run in parallel (Phase 4 implementation)
- T028, T029 — different files, run in parallel (Phase 5 tests)
- T030 — parallel with T028 (different file)
- T033, T034, T035, T036 — parallel validation tasks (Phase 6)

---

## Parallel Example: Phase 3 (US1)

```bash
# Tests (write first — must fail):
T015: settings.store.test.ts
T016: CategoryList.test.ts     ← parallel with T015

# Implementation (after tests are red):
T017: settings.store.ts
T018: CategoryList.vue         ← parallel with T017

# Page integration (depends on both):
T019: SettingsPage.vue         ← after T017 + T018
```

---

## Implementation Strategy

### MVP (US1 only — Phases 1–3)

1. Phase 1: Setup types
2. Phase 2: Backend domain + commands
3. Phase 3: Settings store + CategoryList + SettingsPage
4. **STOP**: Validate category create/rename/delete + persistence works in `npx tauri dev`

### Incremental

1. MVP (US1) → category management works
2. US2 → keyword rules + conflict detection + recategorization works
3. US3 → per-transaction manual override works

### TDD cycle per task

Red → Green → Refactor. No implementation file before its test is written and failing.

---

## Notes

- `Uuid::NAMESPACE_URL` is the standard DNS namespace UUID — stable across Rust versions
- `#[serde(default)]` on `transaction_overrides` requires `HashMap<String,String>` to implement `Default` (it does)
- `recategorize_invoices_cmd` Tauri command name: use `recategorize_invoices` (rename from `recategorize_invoices_cmd` internally if needed to avoid collision with function name)
- Default categories pre-fill in settings store is UI-only state — only written to `config.json` when user clicks Save
- Constitution check: T004/T005/T006/T007 (Rust tests), T015/T016/T021/T022/T023/T028/T029 (TS tests) MUST be written first and verified failing before their corresponding implementation tasks
