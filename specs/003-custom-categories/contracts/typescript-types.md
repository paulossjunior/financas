# TypeScript Type Contracts: Categorias Personalizadas

**Feature**: specs/003-custom-categories
**Date**: 2026-06-08

---

## Changes to `src/types/api.types.ts`

### `AppConfig` (modified — add `transaction_overrides`)

```typescript
export interface AppConfig {
  faturas_directory: string;
  category_rules: CategoryRule[];
  transaction_overrides: Record<string, string>;  // NEW: transaction_id → category name
}
```

`transaction_overrides` is a plain object map (always present; empty object `{}` if none).

---

## New Types for `src/types/api.types.ts`

### `CategoryGroup` (frontend view model — NOT in Rust)

Groups all rules for one category into a single UI object. Computed in the frontend from `AppConfig.category_rules`.

```typescript
export interface CategoryGroup {
  name: string;       // category name (unique)
  keywords: string[]; // flat list of all keywords for this category
  priority: number;   // maps to CategoryRule.priority
}
```

Used exclusively in `SettingsPage.vue` and its child components. Not sent to Rust.

---

## New Types for `src/services/tauri.service.ts`

### `recategorizeInvoices(): Promise<number>`

```typescript
export async function recategorizeInvoices(): Promise<number> {
  return invoke<number>("recategorize_invoices");
}
```

### `overrideTransactionCategory(transactionId: string, category: string): Promise<void>`

```typescript
export async function overrideTransactionCategory(
  transactionId: string,
  category: string
): Promise<void> {
  return invoke<void>("override_transaction_category", {
    transactionId,
    category,
  });
}
```

### `removeTransactionOverride(transactionId: string): Promise<void>`

```typescript
export async function removeTransactionOverride(
  transactionId: string
): Promise<void> {
  return invoke<void>("remove_transaction_override", { transactionId });
}
```

---

## Frontend Component Props

### `CategoryList.vue`

```typescript
interface Props {
  groups: CategoryGroup[];   // all categories (from AppConfig.category_rules, grouped)
}
interface Emits {
  update: (groups: CategoryGroup[]) => void;  // emit when user edits
}
```

### `CategoryGroupEditor.vue`

```typescript
interface Props {
  group: CategoryGroup;
  allKeywords: string[];     // flat list of ALL keywords across ALL categories (for conflict detection)
}
interface Emits {
  save:   (group: CategoryGroup) => void;
  delete: (name: string) => void;
  rename: (oldName: string, newName: string) => void;
}
```

### `TransactionCategoryOverride.vue` (US3)

```typescript
interface Props {
  transactionId: string;
  currentCategory: string;
  availableCategories: string[];  // all known category names
  hasOverride: boolean;
}
interface Emits {
  override: (transactionId: string, category: string) => void;
  removeOverride: (transactionId: string) => void;
}
```
