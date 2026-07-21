# Tasks: Previsão de pagamento do cartão (parcelamentos)

**Feature**: `005-card-payment-forecast` | **Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)

TDD obrigatório (Constituição I): testes falham primeiro, depois implementação mínima.
`[P]` = paralelizável (arquivos distintos, sem dependência pendente).

## Phase 1 — Setup

- [ ] T001 Criar módulo `src-tauri/src/domain/forecast.rs` com os structs `ForecastItem` e `ForecastPoint` (serde, `Decimal` como string) e reexportar em `src-tauri/src/domain/mod.rs`.

## Phase 2 — Foundational (bloqueia todas as histórias)

A projeção de domínio é a base de US1/US2/US3.

- [ ] T002 [P] Escrever testes que FALHAM para `compute_card_forecast(invoices)` em `src-tauri/src/domain/forecast.rs` (mod tests): compra 1/3 → 2 meses futuros; duas compras no mesmo mês → soma + 2 itens; mesma compra em 2 faturas (1/3 e 2/3) → dedup (sem dupla contagem); última parcela (3/3) → vazio; sem parcelamentos → vazio; compra estornada → ignorada; invariante `Σ pontos == installments_future_total`.
- [ ] T003 Implementar `compute_card_forecast` em `src-tauri/src/domain/forecast.rs`: dedup por `(desc normalizada, total, valor)` mantendo maior `current`; âncora = mês de referência mais recente; espalhar parcelas `current+1..total` em `refMonth+k`; série contínua até a última parcela; ignorar `is_reversal`. Fazer os testes de T002 passarem.

## Phase 3 — US1: Ver quanto pagarei por mês (P1) 🎯 MVP

**Meta**: gráfico completo na tela Ano com o valor projetado por mês.
**Teste independente**: com faturas parceladas importadas, a tela Ano mostra uma barra por mês futuro com a soma correta.

- [ ] T004 [US1] Adicionar `card_forecast: Vec<ForecastPoint>` ao `YearSummary` em `src-tauri/src/domain/year.rs` e preencher chamando `compute_card_forecast` (usar os invoices do período).
- [ ] T005 [P] [US1] Teste (Rust) em `year.rs` (mod tests): `compute_year_summary` retorna `card_forecast` coerente (soma == parcelas futuras).
- [ ] T006 [P] [US1] Adicionar `ForecastItem`/`ForecastPoint` e o campo `card_forecast` em `src/types/api.types.ts` (`YearMonthPoint`? não — no `YearSummary`).
- [ ] T007 [US1] Criar `src/components/dashboard/CardForecastChart.vue` — gráfico ECharts de barras por mês (paleta do app, tema claro/escuro, `tabular-nums`), prop `points: ForecastPoint[]` + prop `compact?`.
- [ ] T008 [US1] Integrar na `src/pages/YearPage.vue`: seção "Previsão do cartão" com `CardForecastChart` (completo) + estado vazio ("sem parcelas futuras").

## Phase 4 — US2: Composição de um mês (P2)

**Meta**: ver quais compras compõem cada mês.
**Teste independente**: passar o mouse/selecionar um mês mostra a lista de parcelas.

- [ ] T009 [US2] No `CardForecastChart.vue`, tooltip por barra listando `items` (descrição · "x/y" · valor) a partir do `ForecastPoint.items` (já vindo do backend).

## Phase 5 — US3: Total comprometido + mês que zera (P3) — resumo na tela Mês

**Meta**: total ainda a pagar e mês final; resumo compacto no Mês.
**Teste independente**: tela Mês mostra os próximos meses + total + mês que zera.

- [ ] T010 [US3] Adicionar `forecast_next: Vec<ForecastPoint>` (próximos ~6), `forecast_committed_total` e `forecast_last_month` ao `DashboardData` em `src-tauri/src/domain/dashboard.rs`; preencher em `src-tauri/src/application/get_dashboard.rs` via `compute_card_forecast`.
- [ ] T011 [P] [US3] Teste (Rust) em `dashboard.rs`/`get_dashboard.rs`: total comprometido == parcelas futuras; `forecast_last_month` == mês da última parcela.
- [ ] T012 [P] [US3] Adicionar os campos ao `DashboardData` em `src/types/api.types.ts`.
- [ ] T013 [US3] Integrar na `src/pages/DashboardPage.vue`: card "Próximos meses do cartão" com `CardForecastChart` (compact) + total comprometido + mês que zera + estado vazio.

## Phase 6 — Polish & consistência

- [ ] T014 [P] Rodar `cd src-tauri && cargo test` e `cargo clippy -p financas --all-targets -- -D warnings` — verde.
- [ ] T015 [P] Rodar `npx vue-tsc --noEmit` e `npm run test:run` — verde.
- [ ] T016 Validar o invariante do quickstart no app: soma das barras (Ano) == parcelas futuras do painel; estado vazio sem erro.

## Dependencies

- T001 → T002/T003 (Foundational).
- T003 (compute) bloqueia US1 (T004), US3 (T010).
- US1 (T004–T008) = MVP entregável sozinho.
- US2 (T009) depende de US1 (T007).
- US3 (T010–T013) depende de T003; independente de US2.
- Polish (T014–T016) por último.

## Parallel opportunities

- T002 pode ser escrito enquanto T001 fecha.
- T005/T006 [P], T011/T012 [P] (arquivos distintos: Rust test vs TS types).
- T014/T015 [P] (backend vs frontend).

## MVP

**US1** (T001–T008): gráfico de previsão na tela Ano. Já entrega o valor central da feature.
