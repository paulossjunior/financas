# Tasks: Cálculo rigoroso de inflação pessoal

**Feature**: `011-personal-inflation-calc` | **Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)

TDD (Constituição I): testes falham primeiro. `[P]` = paralelizável. `[x]` = feito.

## Phase 1 — Domínio puro + testes (TDD) 🎯 núcleo

**Meta**: cálculo determinístico, testável, sem I/O.

- [x] T001 [P] Testes que FALHAM em `src-tauri/src/domain/personal_inflation.rs`: exemplo
  de referência (7,7% / 1,7 p.p. / R$5.385 / R$385 / R$7.539 / 10,78% / R$539), soma das
  contribuições == pessoal + ordenação + Σ pesos = 1, categoria única, zero, deflação,
  pesos base vs atuais, proveniência, conversão composta, acumulação por produto, e os
  erros (vazio, total ≤0, gasto negativo, duplicata, base ausente), aviso presente. (15 testes)
- [x] T002 Tipos: `CategoryInput`, `WeightMode` (Current/Base), `Contribution`,
  `PersonalInflationResult`, `PersonalInflationError` (+ `Display`); constantes
  `DEFAULT_BEHAVIORAL_COEFFICIENT = 1.4` e `METHODOLOGY_NOTE`. Dinheiro em `Decimal`
  (serde str), taxas em `f64`.
- [x] T003 `compute(categories, inflacao_oficial, renda, coeficiente, weight_mode)`:
  validações, pesos (atual/base), contribuições ordenadas desc, inflação pessoal,
  diferença em p.p., custo/aumento da cesta, renda corrigida (+ variante conservadora),
  perda de poder de compra, simulação comportamental opcional, `aviso`. Passar T001.
- [x] T004 [P] Helpers de período: `annual_to_monthly` ((1+a)^(1/12)−1),
  `monthly_to_annual` ((1+m)^12−1), `quarterly_to_monthly`, `accumulate` (∏(1+π)−1).
- [x] T005 Reexportar `personal_inflation` em `src-tauri/src/domain/mod.rs`.

## Phase 2 — Comando (orquestração fina, sem rede) 🎯 integração

**Meta**: montar entradas do estado local e expor o DTO.

- [x] T006 `commands/inflation.rs`: `get_personal_inflation_detail` — lê o cache de
  índices (006), obtém gastos por categoria + renda do dashboard, converte
  percent→decimal (÷100), mapeia categoria→grupo (fallback IPCA geral com proveniência),
  filtra gasto > 0, chama `compute` com coeficiente 1,4 e `WeightMode::Current`; retorna
  `Option<PersonalInflationResult>`. Sem cache/sem gastos → `None`. Sem novas chamadas de rede.
- [x] T007 Registrar o comando em `src-tauri/src/lib.rs` (`invoke_handler`).

## Phase 3 — Frontend (tipos, serviço, componente)

**Meta**: espelhar o DTO e exibir contribuições/comparações na tela Ano.

- [x] T008 [P] Tipos `PersonalInflationDetail` e `InflationContribution` em
  `src/types/api.types.ts` (dinheiro como string, taxas como number).
- [x] T009 `getPersonalInflationDetail()` em `src/services/tauri.service.ts`
  (único ponto de `invoke`; `mapError`).
- [x] T010 `src/components/dashboard/InflationContributions.vue`: lista de contribuições
  ordenada (categoria, peso, inflação, contribuição), comparação com o oficial em p.p.,
  impactos em reais (cesta/renda/perda), simulação comportamental rotulada, proveniências
  e o aviso metodológico; estados vazio/carregando/erro. Aplicar a checklist
  `nielsen-heuristics`.
- [x] T011 Integrar `InflationContributions.vue` em `src/pages/YearPage.vue`
  (carrega `getPersonalInflationDetail` no mount; estado vazio quando `null`).

## Phase 4 — Docs + qualidade

- [x] T012 [P] `cd src-tauri && cargo test personal_inflation` + `cargo clippy` verdes.
- [x] T013 [P] `npx vue-tsc --noEmit` + `npm run test:run` verdes.
- [x] T014 Atualizar docs desta feature (plan/research/data-model/contracts/quickstart)
  e o marcador SPECKIT em `CLAUDE.md` apontando para este plano.

## Phase 5 — Merge / release

- [ ] T015 Validar o quickstart no app (contribuições no Ano; `null` sem cache/gastos;
  offline), revisar o diff e abrir/mesclar o PR de release da feature 011.

## Dependencies

- T001 → T002/T003/T004 (TDD). T003 → T006. T006 → T007.
- T008/T009 → T010 → T011.
- T012–T014 após implementação; T015 por último.

## Status

- **Feito**: domínio puro + 15 testes (T001–T005), comando + registro (T006–T007),
  tipos e serviço no frontend (T008–T009), componente `InflationContributions.vue`
  + integração no Ano e Mês (T010–T011), qualidade/docs (T012–T014).
- **Pendente**: validação manual do quickstart no app + release (T015).
