# Implementation Plan: Cálculo rigoroso de inflação pessoal

**Branch**: `011-personal-inflation-calc` | **Date**: 2026-07-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/011-personal-inflation-calc/spec.md`

## Summary

Consolidar e tornar **rigoroso e explicável** o cálculo de inflação pessoal já
esboçado em 006/007. Um novo módulo de domínio **puro e determinístico**
(`domain/personal_inflation.rs`) recebe, por categoria, `{gasto, inflação}` mais
`{renda, inflação oficial, coeficiente comportamental opcional, modo de peso}` e
devolve um **DTO rico**: inflação pessoal = Σ(peso×inflação), lista de
**contribuições** ordenada, diferença vs oficial em **pontos percentuais**, custo
atualizado da cesta, renda corrigida (mais a variante conservadora), perda de poder
de compra, uma **simulação comportamental** opcional (coeficiente default 1,4) e um
**aviso metodológico** obrigatório. Inclui helpers de **conversão de período** por
juro composto (`annual_to_monthly`, `monthly_to_annual`, `quarterly_to_monthly`) e
**acumulação por produto** (`accumulate`). Dinheiro em `Decimal`; taxas em `f64`
(conversões de período exigem potência fracionária).

O comando fino `get_personal_inflation_detail` (`commands/inflation.rs`) monta as
entradas a partir dos gastos por categoria do dashboard e das variações mensais dos
grupos do IPCA já **em cache** (reusa `domain/inflation`, `map_category_to_group`),
convertendo percent→decimal (÷100); quando não há grupo mapeado cai no IPCA geral e
**registra a proveniência**. Sem novas chamadas de rede. O frontend ganha o DTO
espelhado e o serviço `getPersonalInflationDetail`; a exibição das contribuições na
tela **Ano** é o último passo pendente.

## Technical Context

**Language/Version**: Rust 1.75+ (Tauri v2) · TypeScript 5 / Vue 3 (Composition API)

**Primary Dependencies**: `rust_decimal` (dinheiro), `serde`/`serde_json` (DTO);
reusa `domain/inflation` (grupos IPCA + `map_category_to_group`) e o dashboard
agregado. Frontend: Pinia + `services/tauri.service.ts`.

**Storage**: SQLite (`financas.db`) — **nenhuma tabela nova**. Consome a tabela de
cache `inflation_cache` (payload JSON dos índices) criada em 006.

**Testing**: `cargo test` (domínio puro, TDD ≥90% no cálculo) · Vitest (tipos/serviço/componente)

**Target Platform**: Desktop macOS + Windows (Tauri v2)

**Project Type**: Desktop app (backend Rust + frontend Vue)

**Performance Goals**: cálculo pessoal detalhado < 20 ms (dezenas de categorias)

**Constraints**: **offline** — sem novas chamadas de rede; **dinheiro em `Decimal`**
serializado como string; **taxas em `f64`** (conversão de período composta);
resultado **determinístico** (mesma entrada → mesma saída); tolerância de ponto
flutuante nas comparações.

**Scale/Scope**: Usuário único; 1 módulo de domínio novo + 1 comando novo; ~9 grupos
IPCA; 1 DTO espelhado no frontend + 1 componente de contribuições.

## Constitution Check

*GATE: PASS. Re-checado após o design — sem violações.*

- **I. TDD (não-negociável)**: o cálculo é função pura → testes primeiro. Já há **15
  testes unitários** em `domain/personal_inflation.rs`, incluindo o exemplo de
  referência (7,7% / 1,7 p.p. / R$5.385 / R$385 / R$7.539 / 10,78% / R$539), soma das
  contribuições == pessoal, pesos base vs atual, deflação, erros (vazio, total ≤0,
  gasto negativo, duplicata, base ausente), proveniência, conversão composta e
  acumulação por produto. Cobertura ≥90% no cálculo.
- **II. Clean Architecture**: `domain/personal_inflation.rs` é **puro** (sem Tauri/DB/rede);
  `commands/inflation.rs` apenas orquestra (lê cache, monta entradas, chama `compute`);
  o frontend só toca `invoke` via `services/tauri.service.ts`. Fronteiras respeitadas.
- **III. Simplicidade/DRY**: reaproveita a coleta e o mapeamento de 006 em vez de
  duplicar — o módulo novo faz apenas o cálculo rico. Sem abstração especulativa.
- **IV. Integridade de dados**: dinheiro em `Decimal` (serializado como string); erros
  explícitos (nunca zero silencioso) para categoria sem inflação, gasto negativo,
  total não-positivo, duplicata; determinístico.
- **V. Local-first & Privacy**: **nenhuma** chamada de rede nova — consome apenas o
  cache local de índices. Nenhum dado pessoal sai da máquina.

## Project Structure

### Documentation (this feature)

```text
specs/011-personal-inflation-calc/
├── plan.md              # Este arquivo
├── research.md          # Fase 0 — decisões (Decimal vs f64, reuso 006, p.p. vs %, pesos, acumulação, proveniência, coeficiente, período)
├── data-model.md        # Fase 1 — CategoryInput / WeightMode / Contribution / PersonalInflationResult / PersonalInflationError + espelho no frontend
├── quickstart.md        # Fase 1 — cenários de validação + comandos
├── contracts/
│   └── commands.md      # Fase 1 — get_personal_inflation_detail (sem params → PersonalInflationResult | null)
├── checklists/
│   └── requirements.md  # Checklist de requisitos (já existente)
└── tasks.md             # Fase 2 — tarefas (domínio+testes feitos; comando feito; frontend parcial; docs)
```

### Source Code (repository root)

```text
src-tauri/src/
├── domain/
│   ├── personal_inflation.rs   # NOVO (feito) — CategoryInput, WeightMode, Contribution,
│   │                           #   PersonalInflationResult, PersonalInflationError, compute(),
│   │                           #   annual_to_monthly/monthly_to_annual/quarterly_to_monthly/accumulate,
│   │                           #   DEFAULT_BEHAVIORAL_COEFFICIENT=1.4, METHODOLOGY_NOTE + 15 testes
│   ├── inflation.rs            # REUSADO — grupos/headline em cache + map_category_to_group
│   └── mod.rs                  # reexporta personal_inflation
├── commands/
│   ├── inflation.rs            # + get_personal_inflation_detail (feito) — monta entradas do dashboard
│   │                           #   + grupos IPCA (percent→decimal), coeficiente 1,4, WeightMode::Current
│   └── mod.rs
├── infrastructure/
│   └── db.rs                   # REUSADO — load_inflation_cache (cache de 006, sem tabela nova)
└── lib.rs                      # registra get_personal_inflation_detail

src/
├── types/api.types.ts          # + PersonalInflationDetail / InflationContribution (feito)
├── services/tauri.service.ts   # + getPersonalInflationDetail() (feito)
├── components/dashboard/
│   └── InflationContributions.vue  # PENDENTE — render das contribuições/comparações na tela Ano
└── pages/
    └── YearPage.vue            # PENDENTE — integrar o componente de contribuições
```

**Structure Decision**: cálculo isolado num módulo de **domínio puro** (testável e
determinístico); o comando é fino e apenas orquestra, reaproveitando a
infraestrutura de índices de 006 (sem rede nova, sem tabela nova); o frontend
espelha o DTO e exibe as contribuições numa tela existente (Ano).

## Complexity Tracking

*Sem violações da Constituição — nada a justificar.*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| — | — | — |
