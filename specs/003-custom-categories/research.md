# Research: Categorias Personalizadas de Despesas

**Date**: 2026-06-08
**Feature**: specs/003-custom-categories

---

## Decision 1: Category data model — flat rules vs. separate entity

**Decision**: Keep category management as a grouped view over `Vec<CategoryRule>` (existing). Do NOT add a separate `Category` entity to `AppConfig`. Categories are implicitly defined by unique `category` string values in the rule list. One `CategoryRule` per category (with all keywords in one list).

**Rationale**: The backend `CategoryRule { keywords, category, priority }` already handles the use case. Adding a separate `Category` struct in AppConfig would require a migration path and add surface area with no algorithmic benefit. The UI groups rules by `category` name to present the abstraction. YAGNI — no second entity needed.

**Alternatives considered**:
- `AppConfig.categories: Vec<{ name, rules }>` — cleaner schema but would break existing `save_config`/`get_config` deserialization; requires migration and doubles Rust surface area.
- Separate `CategoryStore` — overkill for a local JSON config.

---

## Decision 2: Reprocessing existing in-memory invoices

**Decision**: New Tauri command `recategorize_invoices()` re-runs categorization on all invoices currently in `InvoiceStore`. Called by the frontend after every `save_config` that changes `category_rules`. Returns `usize` (count of recategorized transactions).

**Rationale**: `InvoiceStore` is in-memory only — invoices are loaded for the current session and not persisted to disk. Re-importing XLSX files is destructive and slow. A dedicated `recategorize_invoices` command satisfies FR-008 without reimport, and keeps the responsibility in the backend where `Categorizer` lives. The command must also apply `transaction_overrides` on top of rule-based categorization.

**Alternatives considered**:
- Re-import all XLSX files from `faturas_directory` automatically — destructive (resets `imported_at`, re-detects `is_replace`), slow, and requires file system access from a frontend action.
- Frontend-only recategorization — would require exporting full transaction data to JS, violating Clean Architecture (domain logic in domain layer).

---

## Decision 3: Transaction ID stability for override persistence (US3)

**Decision**: Change `Invoice::new()` to generate `id` as `Uuid::new_v5(NAMESPACE_URL, filename.as_bytes())` instead of `Uuid::new_v4()`. This makes `invoice_id` deterministic from filename, which transitively makes `transaction_id` (= `Uuid::new_v5(invoice_id, row_index)`) stable across sessions.

**Rationale**: `transaction_overrides` in `AppConfig` must survive app restarts (persisted to `config.json`). The key is `transaction_id`. If `invoice_id` is random (`new_v4`), reimporting the same file generates new `invoice_id` → new `transaction_id` → override lookup misses. With `new_v5(filename)`, the same XLSX file always produces the same invoice+transaction IDs regardless of session.

**Impact**: Safe change — `InvoiceStore` is in-memory only, so there are no persisted records to migrate. The `is_replace` detection in `InvoiceStore::add()` currently compares filenames; it would now also match by UUID (bonus consistency).

**Alternatives considered**:
- Use `description + date + amount` as override key — stable without code changes, but verbose, error-prone on edits, and harder to store/look up.
- Store override as `(filename, row_index)` — stable and simple, but requires changes throughout the codebase to carry filename context everywhere.

---

## Decision 4: Transaction overrides persistence

**Decision**: Add `transaction_overrides: HashMap<String, String>` to `AppConfig`. Key = `transaction_id.to_string()` (UUID), value = override category name. Persisted in `config.json` via existing `ConfigStore::save`. Applied in both `import_invoice` and `recategorize_invoices` after rule-based categorization.

**Rationale**: Reuses existing `ConfigStore` and `save_config`/`get_config` commands. New field is additive — existing `config.json` files without the field deserialize cleanly via `#[serde(default)]`.

**Alternatives considered**:
- Separate `overrides.json` file — extra `ConfigStore` variant, more infra code, no benefit.
- Store overrides in `InvoiceStore` (in-memory) — would be lost on app restart, violating FR-007.

---

## Decision 5: Conflict detection for duplicate keywords

**Decision**: Client-side conflict detection in the Settings page before calling `save_config`. When user tries to add keyword K to category C, the frontend checks if K already exists in any other category's keywords list. Displays inline warning; user can override.

**Rationale**: The Rust `Categorizer` resolves conflicts by priority (lowest wins). But showing the conflict upfront is better UX (FR-006). Client-side check is sufficient because all rules are loaded in memory; no server roundtrip needed.

**Alternatives considered**:
- Backend validation in `save_config` — unnecessary complexity; config is a dumb blob, conflict resolution belongs in the application layer.

---

## Decision 6: "Default rules" initialization

**Decision**: When `AppConfig.category_rules` is empty, the Settings page initializes the UI with the default rules (Alimentação, Transporte, Saúde, etc.) pre-populated. Saving these to config transitions the system from "use built-in defaults" to "use user-managed rules". User can then edit or delete them.

**Rationale**: An empty category list in the UI would be confusing — user sees no starting point. Pre-filling with defaults makes the transition from automatic to manual management explicit and non-destructive. The backend logic (`if cat_rules.is_empty() { Categorizer::with_defaults() }`) already handles the empty case.

**Alternatives considered**:
- Always write defaults to config on first launch — would lock users into defaults before they even open Settings.
- Show empty list and let user start from scratch — poor UX; forces manual re-entry of common categories.

---

## Decision 7: US3 transaction access point

**Decision**: Manual transaction override (US3, P3) is triggered from the Dashboard category detail view. Each category card already shows `top_transactions`. Expand to "Ver todas" → full transaction list for that category → inline category dropdown per row.

**Rationale**: Dashboard already has category context and partial transaction data. Avoids creating a new page for P3. Consistent with existing pattern where category detail is visible in the dashboard.

**Alternatives considered**:
- New "Transações" page — out of scope for this feature; would require new route, new Tauri command for full transaction list.
- HistoryPage invoice rows with transaction sub-list — requires expanding InvoiceRow, adds complexity to already-complete US3 remove flow.
