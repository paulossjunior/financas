# Tasks: Categorias recorrentes + baseline + anti-duplicação

Feature branch `010-recurring-categories`. All tasks below were implemented (TDD on
the domain). Status: ✅ done.

## Phase 1 — Domain (pure, tested)
- [x] T001 `domain/recurring.rs`: `RecurringCategory` (vigência + `base_amount`), `Observation`/`DerivedFixed` with `kind`, `FixedOrigin`.
- [x] T002 `derive_month` (realized > user base > baseline), `baseline` (last 3 months), `is_manual_superseded` (anti-dup), `dominant_kind`.
- [x] T003 `detect_suggestions` (≥3/4 months, low variation, opt-in) + dismissals.
- [x] T004 Unit tests for all of the above (28 tests incl. income inference, vigência, base override).

## Phase 2 — Application
- [x] T005 `application/recurring_fixed.rs`: `build_observations` (card=Fatura, bank=Extrato incl. income), `recurring_category_infos` (baseline/origin/varies/base), `fixed_for_month`, `suggestions`.

## Phase 3 — Infrastructure
- [x] T006 `db.rs`: tables `recurring_categories` (+`base_amount`), `dismissed_recurring_suggestions`, `categories`; `bank_entries.user_categorized`; idempotent migrations.
- [x] T007 CRUD: set/list recurring, set base, dismiss, list dismissed, all_category_names.
- [x] T008 `recategorize_bank_entries` (keyword rules over extrato, override-safe); `CategoryRule.priority` widened u8→u32.

## Phase 4 — Commands + integration
- [x] T009 `commands/recurring.rs` (7 commands) registered in `lib.rs`.
- [x] T010 `get_dashboard`/`year`: reclassify recurring-category extrato spend as fixed + anti-dup.
- [x] T011 Unified categorization: recategorize + startup + add_category_keyword apply rules to card + extrato.

## Phase 5 — Frontend
- [x] T012 Types + `tauri.service` wrappers.
- [x] T013 `CategoriasPage.vue` (2 tabs, recurring/vigência/base/origin, suggestions, new-category modal, delete confirm).
- [x] T014 `MappingPage.vue`: card+extrato "Outros" queue, "recém-saído" highlight (option A).
- [x] T015 `ManualEntriesPage.vue`: derived fixas/rendas card (split by kind).
- [x] T016 Nav "Despesas & Receitas"; friendlier fallback error.

## Phase 6 — Docs / quality
- [x] T017 `nielsen-heuristics` skill + CLAUDE.md reference; ARCHITECTURE updated.
- [ ] T018 Merge to main + release v0.2.1 (pending user go-ahead).
