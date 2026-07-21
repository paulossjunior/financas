# Tauri Commands: Categorias Personalizadas

**Feature**: specs/003-custom-categories
**Date**: 2026-06-08

---

## Existing Commands (unchanged signatures)

### `get_config() -> Result<AppConfig, String>`

Returns current `AppConfig` from in-memory state. After this feature, `AppConfig` includes `transaction_overrides`.

### `save_config(new_config: AppConfig) -> Result<(), String>`

Persists updated `AppConfig` (including `category_rules` and `transaction_overrides`) to `config.json`. After save, frontend MUST call `recategorize_invoices` to apply new rules to in-memory invoices.

---

## New Commands

### `recategorize_invoices() -> Result<usize, String>`

Re-runs keyword categorization on all invoices currently loaded in `InvoiceStore`. Applies `transaction_overrides` on top of rule-based results.

**Returns**: Count of transactions whose category was changed.

**Behavior**:
1. Lock `InvoiceStore` (read), lock `AppConfig` (read)
2. Build `Categorizer::new(config.category_rules)` if rules non-empty, else `Categorizer::with_defaults()`
3. For each `Invoice` → for each `Transaction`:
   - Compute `new_category = categorizer.categorize(&tx.description)`
   - If `config.transaction_overrides.contains_key(&tx.id.to_string())`: use override value instead
   - If `tx.category != new_category`: update and increment counter
4. Return counter

**Called by**: Frontend, immediately after `save_config` when `category_rules` changed.

**Note**: In-memory only — does not reimport XLSX files.

---

### `override_transaction_category(transaction_id: String, category: String) -> Result<(), String>`

Saves a manual category override for a specific transaction. Persists to `AppConfig.transaction_overrides` in `config.json`. Updates the in-memory transaction's `category` field immediately.

**Args**:
- `transaction_id`: UUID string of the transaction
- `category`: Target category name (must be non-empty)

**Behavior**:
1. Validate `category` is non-empty
2. Update `config.transaction_overrides[transaction_id] = category`
3. Persist updated config via `ConfigStore::save`
4. Update in-memory `InvoiceStore`: find transaction by id, set `category`
5. Return `Ok(())`

**Called by**: Frontend transaction detail UI (US3).

---

### `remove_transaction_override(transaction_id: String) -> Result<(), String>`

Removes a manual override, restoring rule-based categorization for that transaction.

**Args**:
- `transaction_id`: UUID string of the transaction

**Behavior**:
1. Remove `transaction_id` from `config.transaction_overrides`
2. Persist updated config via `ConfigStore::save`
3. Re-categorize that specific transaction using current rules (or "Outros" if no match)
4. Update in-memory `InvoiceStore`
5. Return `Ok(())`

**Called by**: Frontend, when user removes a manual override (US3).

---

## AppConfig Schema Changes

```json
{
  "faturas_directory": "faturas",
  "category_rules": [
    {
      "keywords": ["IFOOD", "RESTAURANTE"],
      "category": "Alimentação",
      "priority": 10
    }
  ],
  "transaction_overrides": {
    "550e8400-e29b-41d4-a716-446655440000": "Educação"
  }
}
```

The `transaction_overrides` field is optional in JSON — missing = empty map (`#[serde(default)]`).
