# Tasks: Explicador do impacto da inflação

**Feature**: `007-inflation-explainer` | TDD.

## Phase 1 — Foundational (puro, testável)
- [ ] T001 [P] Testes Vitest para `src/utils/inflation-explainer.ts` (projeção 12m/3a/5a; erosão de renda; poder de compra de R$100; deflação; sem dado → vazio).
- [ ] T002 `src/utils/inflation-explainer.ts`: `annualizeMonthly`, `project(value, annualPct, years)`, `realValue`, e `buildExplainer({inflAnnual, personalAnnual, monthlyExpense, monthlyIncome, personalDiff, topGroup})` → itens com número + frase. Passar T001.

## Phase 2 — US1..US4 (UI)
- [ ] T003 `src/components/dashboard/InflationExplainer.vue`: consome `getInflation` + props gasto/renda; renderiza cards (projeção de gastos, poder de compra da renda, pessoal vs IPCA, valor do dinheiro); estados vazio/sem-dado; prop `compact`.
- [ ] T004 [US1] Integrar na `DashboardPage.vue` (mês) — passa gasto/renda do mês.
- [ ] T005 [US2] Integrar na `YearPage.vue` (ano) — passa média/totais do período.

## Phase 3 — Polish
- [ ] T006 [P] `npx vue-tsc --noEmit` + `npm run test:run` verde.
- [ ] T007 Validar no app: frases claras, reais, rótulo de estimativa; offline com cache; sem índice → convite.

## MVP
US1+US2 (projeção de gastos + poder de compra) já entrega o valor central.
