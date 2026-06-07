# Data Model: Gestor Financeiro — Dashboard de Faturas BTG

**Branch**: `001-credit-card-dashboard` | **Date**: 2026-06-07

## Domain Entities

### Transaction

Represents a single line item on a BTG credit card invoice.

```
Transaction {
  id:          Uuid          // generated deterministically from (invoice_id + row_index)
  invoice_id:  Uuid          // parent invoice reference
  date:        NaiveDate     // transaction date (DD/MM/YYYY from XLSX)
  description: String        // merchant name or label (trimmed, normalized)
  amount:      Decimal       // positive = charge; negative = reversal/credit
  category:    String        // explicit from XLSX or inferred; never empty (default: "Outros")
  installment: Option<InstallmentInfo>  // present only for parceled transactions
  is_reversal: bool          // true when amount < 0
}

InstallmentInfo {
  current: u8   // e.g., 2 (in "2/12")
  total:   u8   // e.g., 12
}
```

**Validation rules**:
- `description` must not be empty after trimming; if empty → reject row with error.
- `amount` must be parseable as Decimal from Brazilian locale format (comma decimal separator).
- `date` must be a valid calendar date.
- `category` is always set; inference runs if XLSX column is absent or blank.

**Invariants**:
- `is_reversal` is derived from `amount < 0`, not stored separately as mutable state.
- Two transactions from the same invoice at the same row index produce the same `id` (idempotent re-import).

---

### Invoice

Represents one BTG credit card statement file.

```
Invoice {
  id:              Uuid
  filename:        String           // original file name (e.g., "2026-06-05_Fatura...BTG.xlsx")
  reference_month: YearMonth        // e.g., 2026-06
  due_date:        Option<NaiveDate> // extracted from XLSX metadata if present
  transactions:    Vec<Transaction>
  imported_at:     NaiveDateTime    // when this file was processed
}

YearMonth {
  year:  i32
  month: u8   // 1–12
}
```

**Validation rules**:
- Two Invoice records with the same `filename` are considered duplicates; re-import replaces the existing record.
- `reference_month` is inferred from the filename date prefix (YYYY-MM-DD format) if not available in XLSX metadata.

---

### Category

An aggregated view over transactions, computed on demand — not stored as a persistent entity.

```
Category {
  name:              String
  total:             Decimal    // sum of all non-reversal transaction amounts
  reversal_total:    Decimal    // sum of reversal amounts (negative, for display)
  net_total:         Decimal    // total + reversal_total (effective spend)
  percentage:        f64        // net_total / dashboard_total * 100.0
  transaction_count: u32
  top_transactions:  Vec<TransactionSummary>  // top 3 by amount within category
}

TransactionSummary {
  description: String
  amount:      Decimal
  date:        NaiveDate
}
```

**Computation rules**:
- `percentage` is computed from `net_total`, not `total`, to reflect actual spend including reversals.
- Categories with `net_total <= 0` (net credits) are displayed separately, not in the main ranking.

---

### DashboardData

The top-level output of the `get_dashboard` Tauri command. Fully computed from Invoice records.

```
DashboardData {
  period:             DashboardPeriod
  total_charged:      Decimal              // sum of all positive amounts
  total_reversals:    Decimal              // sum of all negative amounts
  net_total:          Decimal              // effective total spend
  categories:         Vec<Category>        // ordered by net_total desc
  top_transactions:   Vec<Transaction>     // top 5 individual charges by amount
  monthly_trend:      Vec<MonthlySnapshot> // one entry per invoice month; empty if < 2 invoices
  invoice_count:      u32
}

DashboardPeriod {
  from: YearMonth
  to:   YearMonth
}

MonthlySnapshot {
  month:       YearMonth
  net_total:   Decimal
  categories:  Vec<CategorySnapshot>   // name + net_total per category for this month
}

CategorySnapshot {
  name:      String
  net_total: Decimal
}
```

---

### DashboardFilter

Input parameter for filtering dashboard data.

```
DashboardFilter {
  invoice_ids:  Option<Vec<Uuid>>    // null = all invoices
  categories:   Option<Vec<String>>  // null = all categories
  date_from:    Option<NaiveDate>
  date_to:      Option<NaiveDate>
}
```

---

### AppConfig

Persisted application configuration (stored in Tauri app data dir as `config.json`).

```
AppConfig {
  faturas_directory:    PathBuf          // user-selected invoices folder
  category_rules:       Vec<CategoryRule>
  last_import_path:     Option<PathBuf>
}

CategoryRule {
  keywords:  Vec<String>   // case-insensitive substrings to match against description
  category:  String        // assigned category name
  priority:  u8            // lower = higher priority (0 = highest)
}
```

---

## State Transitions

### Invoice Import Flow

```
File selected by user
  → Validate file extension (.xlsx)
  → Open with calamine
    → Error: file encrypted → return ImportError::Encrypted (with help message)
    → Error: columns missing → return ImportError::InvalidFormat { missing: Vec<String> }
  → Parse header row → map column names to indices
  → Parse each data row
    → Skip blank rows
    → Validate fields → on error: collect ParseError, continue to next row
  → Infer categories for uncategorized transactions
  → Create Invoice record
  → Deduplicate against existing invoices (by filename)
  → Emit ImportResult { invoice, warnings: Vec<ParseError> }
```

### Dashboard Computation Flow

```
DashboardFilter received
  → Load matching Invoice records from in-memory store
  → Flatten transactions, apply date filters
  → Group by category → compute Category aggregates
  → Sort categories by net_total desc
  → Identify top 5 transactions by amount
  → If invoice_count >= 2: compute monthly trend
  → Return DashboardData
```

---

## Entity Relationships

```
AppConfig (1) ──── (n) CategoryRule
Invoice (1) ──── (n) Transaction
DashboardData (computed from) ──── (n) Invoice
Category (computed from) ──── (n) Transaction
```

---

## Type Decisions

| Domain Concept | Type | Reason |
|---------------|------|--------|
| Monetary amount | `rust_decimal::Decimal` | Exact base-10 arithmetic; no float rounding |
| Transaction date | `chrono::NaiveDate` | Date only (no timezone needed) |
| UUID keys | `uuid::Uuid` | Deterministic generation for idempotent import |
| Category | `String` (not enum) | User-configurable; new categories can be added at runtime |
| Percentage | `f64` | Display-only; precision loss acceptable for 2-decimal percentage display |
