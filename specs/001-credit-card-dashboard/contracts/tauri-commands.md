# Tauri Command Contracts

**Branch**: `001-credit-card-dashboard` | **Date**: 2026-06-07

These are the Tauri IPC commands exposed by the Rust backend to the Vue frontend.
All commands are invoked via `invoke()` from `@tauri-apps/api/core`.

---

## Commands

### `import_invoices`

Import one or more XLSX invoice files. Files must be decrypted (password-removed) before import.

**Invoke**:
```typescript
invoke<ImportResult[]>('import_invoices', { paths: string[] })
```

**Input**:
```typescript
{ paths: string[] }  // absolute file paths selected via dialog
```

**Output** (success):
```typescript
ImportResult {
  invoice_id:  string      // UUID
  filename:    string
  month:       string      // "2026-06" (YYYY-MM)
  row_count:   number      // total rows parsed
  warnings:    ParseWarning[]
}

ParseWarning {
  row:     number
  message: string
}
```

**Error cases**:
```typescript
// Tauri throws string errors; frontend catches and maps:
"ENCRYPTED_FILE"         // file is password-protected — user must unlock first
"INVALID_FORMAT:<cols>"  // expected columns not found; cols = comma-separated missing names
"FILE_NOT_FOUND"         // path no longer accessible
"DUPLICATE_INVOICE"      // filename already imported; re-import replaces existing
```

---

### `get_dashboard`

Compute and return dashboard data for the given filter. All invoices used if filter is null.

**Invoke**:
```typescript
invoke<DashboardData>('get_dashboard', { filter?: DashboardFilter })
```

**Input**:
```typescript
DashboardFilter {
  invoice_ids?: string[]   // UUID strings; null = all
  categories?:  string[]   // null = all
  date_from?:   string     // "YYYY-MM-DD"; null = no lower bound
  date_to?:     string     // "YYYY-MM-DD"; null = no upper bound
}
```

**Output** (success):
```typescript
DashboardData {
  period: { from: string; to: string }   // "YYYY-MM"
  total_charged:   string   // Decimal as string to preserve precision
  total_reversals: string
  net_total:       string
  invoice_count:   number
  categories: Category[]
  top_transactions: TransactionSummary[]  // top 5
  monthly_trend: MonthlySnapshot[]        // empty if < 2 invoices
}

Category {
  name:              string
  total:             string   // Decimal as string
  reversal_total:    string
  net_total:         string
  percentage:        number   // float, 2 decimal places
  transaction_count: number
  top_transactions:  TransactionSummary[]  // top 3 within category
}

TransactionSummary {
  id:          string
  date:        string   // "YYYY-MM-DD"
  description: string
  amount:      string   // Decimal as string
  category:    string
}

MonthlySnapshot {
  month:     string   // "YYYY-MM"
  net_total: string
  categories: CategorySnapshot[]
}

CategorySnapshot {
  name:      string
  net_total: string
}
```

**Error cases**:
```typescript
"NO_DATA"   // no invoices imported yet
```

---

### `list_invoices`

List all imported invoices.

**Invoke**:
```typescript
invoke<InvoiceInfo[]>('list_invoices')
```

**Output** (success):
```typescript
InvoiceInfo {
  id:          string   // UUID
  filename:    string
  month:       string   // "YYYY-MM"
  due_date?:   string   // "YYYY-MM-DD" or null
  row_count:   number
  imported_at: string   // ISO 8601 datetime
}
```

---

### `remove_invoice`

Remove an imported invoice and all its transactions from the in-memory store.

**Invoke**:
```typescript
invoke<void>('remove_invoice', { invoice_id: string })
```

**Error cases**:
```typescript
"INVOICE_NOT_FOUND"
```

---

### `get_config`

Retrieve current app configuration.

**Invoke**:
```typescript
invoke<AppConfig>('get_config')
```

**Output**:
```typescript
AppConfig {
  faturas_directory: string   // absolute path
  category_rules: CategoryRule[]
}

CategoryRule {
  keywords:  string[]
  category:  string
  priority:  number
}
```

---

### `save_config`

Persist updated app configuration.

**Invoke**:
```typescript
invoke<void>('save_config', { config: AppConfig })
```

---

## Type Conventions

| Convention | Reason |
|-----------|--------|
| Decimals serialized as `string` | Avoids JavaScript `number` precision loss for large amounts |
| Dates as `"YYYY-MM-DD"` strings | Unambiguous; no timezone conversion needed |
| Months as `"YYYY-MM"` strings | Compact, sortable |
| Errors as string codes | Simple to match in frontend without deserializing error objects |
| UUIDs as lowercase hyphenated strings | Standard UUID v4 format |
