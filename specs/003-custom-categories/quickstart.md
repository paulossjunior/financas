# Quickstart Validation: Categorias Personalizadas

**Feature**: specs/003-custom-categories
**Date**: 2026-06-08

## Prerequisites

- App running: `npx tauri dev` (or production build)
- At least one XLSX invoice imported (use any file from `faturas/`)
- Unit tests passing: `npm run test`
- E2E tests passing: `npx playwright test`

---

## Scenario 1 — Create category and verify persistence (US1 P1)

**Goal**: Confirms FR-001, FR-009, SC-004.

1. Open the app → navigate to **Configurações**
2. In the "Categorias & Regras" section, click **+ Nova Categoria**
3. Type `Pets` → confirm
4. Verify: `Pets` appears in the category list
5. Close the app completely
6. Reopen the app → navigate to Configurações
7. Verify: `Pets` category still appears in the list

**Expected**: Category persists across sessions. `config.json` in app config dir contains `"category": "Pets"` entry.

---

## Scenario 2 — Add keyword rule and verify automatic categorization (US2 P2)

**Goal**: Confirms FR-004, FR-005, FR-008, SC-001, SC-003.

1. In Configurações → category `Pets` → click **+ Palavra-chave**
2. Type `COBASI` → save
3. Verify: keyword appears under `Pets`
4. Click **Salvar** on the settings page
5. Navigate to **Dashboard** — check that any existing transaction with "COBASI" in description now shows category `Pets`
6. (If no COBASI transaction exists: import an invoice, confirm categorization on import)

**Expected**: After save + recategorization, transactions with "COBASI" in description are categorized as `Pets`. No reimport needed.

---

## Scenario 3 — Conflict detection (US2 P2)

**Goal**: Confirms FR-006.

1. In Configurações → category `Alimentação` — note an existing keyword (e.g., `IFOOD`)
2. Create or select another category (e.g., `Delivery`)
3. Try to add the keyword `IFOOD` to `Delivery`
4. Verify: inline warning appears — "Palavra-chave 'IFOOD' já usada em 'Alimentação'"
5. User can still save (override the conflict), or choose a different keyword

**Expected**: Warning shown; user is not blocked.

---

## Scenario 4 — Delete category with rules (US1 P1)

**Goal**: Confirms FR-002, FR-003.

1. In Configurações → select category `Pets` (with keyword `COBASI`)
2. Click **Deletar**
3. Verify: confirmation dialog shows "Esta categoria tem 1 regra de palavras-chave. As transações voltarão para 'Outros'. Confirmar?"
4. Confirm → category disappears from list
5. Navigate to Dashboard → transactions previously categorized as `Pets` now show `Outros`

**Expected**: Deletion cascades, transactions recategorized automatically.

---

## Scenario 5 — Manual transaction override (US3 P3)

**Goal**: Confirms FR-007, SC-003.

1. In Dashboard → find a transaction under any category (e.g., "AMAZON MKTPL" in "Compras Online")
2. Click on the transaction → category dropdown appears
3. Select `Educação` from the dropdown
4. Verify: transaction immediately shows `Educação`
5. Navigate away and back → override persists
6. Close and reopen app → transaction still shows `Educação`
7. Click the override indicator → option to "Restaurar automático"
8. Confirm → transaction returns to rule-based category

**Expected**: Override persists across navigation and app restarts. Removal restores automatic categorization.

---

## Automated Test Targets

| Scenario | Vitest unit | Playwright E2E |
|----------|-------------|----------------|
| CategoryGroup computed from AppConfig.category_rules | `src/__tests__/stores/settings.store.test.ts` | — |
| Conflict detection logic | `src/__tests__/utils/category-conflict.test.ts` | — |
| recategorize_invoices Rust command | `src-tauri/tests/recategorize.rs` | — |
| Invoice deterministic ID | `src-tauri/tests/invoice_id.rs` | — |
| US1: Create/rename/delete category via Settings UI | — | `tests/categories.spec.ts` |
| US2: Add keyword → import → verify categorization | — | `tests/categories.spec.ts` |
| US3: Override transaction, persist, remove | — | `tests/categories.spec.ts` |

See [contracts/tauri-commands.md](contracts/tauri-commands.md) for command signatures.
See [data-model.md](data-model.md) for entity relationships.
