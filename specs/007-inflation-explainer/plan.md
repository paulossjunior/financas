# Implementation Plan: Explicador do impacto da inflação

**Branch**: `007-inflation-explainer` | **Date**: 2026-07-21 | **Spec**: [spec.md](spec.md)

## Summary
Painel que traduz IPCA + inflação pessoal em frases simples e reais, com base em gasto/renda mensais. **Frontend-only**: reusa `InflationData` (cache, comando `get_inflation` já existente) + totais de gasto/renda das telas. Sem backend novo.

## Technical Context
- **Stack**: Vue 3 / TS. Sem mudança Rust.
- **Dados**: `InflationData` (cache offline) + gasto/renda mensais das páginas.
- **Testing**: Vitest (helper puro de projeção — TDD).
- **Constraints**: local/offline; projeções são estimativas (juros compostos sobre inflação observada), rotuladas.

## Constitution Check
- **I. TDD**: helper `src/utils/inflation-explainer.ts` (puro) testado primeiro (projeção, erosão, poder de compra, deflação, sem dado).
- **II. Clean arch**: lógica de cálculo isolada no util; componente só apresenta.
- **III. Simplicidade**: zero backend; reusa `get_inflation`.
- **V. Local-first**: nenhuma rede nova (usa cache).
Projeções usam float (estimativa de display, não dinheiro persistido) — aceitável e rotulado.

## Project Structure
```
src/
  utils/inflation-explainer.ts        # NOVO — funções puras + tipos (+ testes Vitest)
  components/dashboard/InflationExplainer.vue  # NOVO — cards com frases-resumo
  pages/DashboardPage.vue             # + explicador (mês)
  pages/YearPage.vue                  # + explicador (ano)
```

## Complexity Tracking
Sem desvios (float em projeção estimada é intencional; dinheiro real segue em Decimal no resto do app).
