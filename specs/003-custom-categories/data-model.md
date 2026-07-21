# Data Model: Categorias Personalizadas de Despesas

**Date**: 2026-06-08
**Feature**: specs/003-custom-categories

---

## Entities

### CategoryRule (existing, unchanged schema)

Represents a keyword-based automatic categorization rule.

```rust
pub struct CategoryRule {
    pub keywords: Vec<String>,  // case-insensitive substring match
    pub category: String,       // target category name
    pub priority: u8,           // lower = higher priority; sort ascending before matching
}
```

**Constraints**:
- `category` must be non-empty and unique per rule (one rule per category in UI model)
- `keywords` may be empty (category exists but has no auto-match rules yet)
- `priority` range: 0–255; lower number wins on conflict
- keyword matching: case-insensitive, substring (`desc.to_uppercase().contains(kw.to_uppercase())`)

---

### AppConfig (extended)

Persisted as `config.json` in the app config directory.

```rust
pub struct AppConfig {
    pub faturas_directory: String,
    pub category_rules: Vec<CategoryRule>,
    pub transaction_overrides: HashMap<String, String>,  // NEW — US3
}
```

**New field — `transaction_overrides`**:
- Key: `transaction_id.to_string()` (UUID v5, deterministic)
- Value: override category name (must be non-empty string)
- Default: empty map (via `#[serde(default)]`)
- Existing `config.json` files without this field deserialize cleanly

---

### Invoice (changed — ID generation)

```rust
pub struct Invoice {
    pub id: Uuid,      // CHANGE: Uuid::new_v5(NAMESPACE_URL, filename.as_bytes())
    pub filename: String,
    pub reference_month: YearMonth,
    pub due_date: Option<NaiveDate>,
    pub transactions: Vec<Transaction>,
    pub imported_at: NaiveDateTime,
}
```

**Change**: `id` changes from `Uuid::new_v4()` (random) to `Uuid::new_v5(Uuid::NAMESPACE_URL, filename.as_bytes())` (deterministic from filename). This makes `Transaction.id` stable across sessions for the same XLSX file, enabling `transaction_overrides` to survive app restarts.

**Impact**: `InvoiceStore` is in-memory only — no migration needed.

---

### Transaction (unchanged)

```rust
pub struct Transaction {
    pub id: Uuid,            // Uuid::new_v5(&invoice_id, row_index_str)
    pub invoice_id: Uuid,
    pub date: NaiveDate,
    pub description: String,
    pub amount: Decimal,
    pub category: String,   // set by Categorizer, then overridden if override exists
    pub installment: Option<InstallmentInfo>,
    pub is_reversal: bool,
}
```

After making `invoice_id` deterministic, `transaction.id` becomes deterministic — same XLSX row always → same UUID.

---

## State Transitions

### Category lifecycle

```
[Not Exists] --(create with name)--> [Exists, empty keywords]
                                            |
                              (add keywords) |
                                            v
                                  [Exists, with keywords]
                                            |
                  (rename category name) ---+
                  (add/remove keywords) ----+
                                            |
                              (delete) ------v
                                       [Not Exists]
                                (all rules with this category removed)
                                (transactions reclassified → "Outros" or next matching rule)
```

### Override lifecycle

```
[Transaction: auto-categorized] --(user selects category)--> [Transaction: manually overridden]
                                                                         |
                                              (user removes override) ---v
                                                        [Transaction: auto-categorized]
```

---

## Relationships

- `AppConfig` 1 → N `CategoryRule`: one config holds all rules
- `AppConfig` 1 → N `(transaction_id, category)` overrides: sparse map, only for manually changed rows
- `CategoryRule` N → M `keyword`: many keywords per rule, stored in `keywords: Vec<String>`
- `Invoice` 1 → N `Transaction`: one invoice has many rows
- `Transaction.id` ↔ `AppConfig.transaction_overrides` key: deterministic UUID enables cross-session lookup

---

## Validation Rules

| Entity | Field | Rule |
|--------|-------|------|
| `CategoryRule` | `category` | Non-empty, trimmed |
| `CategoryRule` | `keywords` | Each keyword non-empty, trimmed; uniqueness across categories warned (not blocked) |
| `AppConfig` | `transaction_overrides` value | Non-empty category name string |
| `Invoice` | `id` | Must be `Uuid::new_v5(NAMESPACE_URL, filename.as_bytes())` |
