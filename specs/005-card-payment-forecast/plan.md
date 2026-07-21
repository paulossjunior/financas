# Implementation Plan: Previsão de pagamento do cartão (parcelamentos)

**Branch**: `005-card-payment-forecast` | **Date**: 2026-07-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/005-card-payment-forecast/spec.md`

## Summary

Projetar, mês a mês, o valor de cartão já comprometido pelas compras parceladas existentes, e exibir como gráfico completo na tela **Ano** + um resumo compacto na tela **Mês**. O cálculo é uma função pura no domínio Rust que espalha as parcelas restantes de cada compra pelos meses futuros (dedup por compra para não contar em dobro), exposta num DTO e consumida por um componente ECharts reutilizável.

## Technical Context

**Language/Version**: Rust 1.75+ (backend Tauri) · TypeScript 5 / Vue 3 (frontend)

**Primary Dependencies**: `rust_decimal` (dinheiro exato), `chrono` (aritmética de meses), `rusqlite` (já em uso, sem mudança de schema); frontend: `vue-echarts` (ECharts), Pinia

**Storage**: SQLite existente. **Sem migração** — a projeção é derivada de `transactions.installment` (parcela atual/total) já persistido.

**Testing**: `cargo test` (unidade no domínio, TDD, ≥90% na lógica de projeção) · Vitest (componente/loja)

**Target Platform**: Desktop macOS + Windows (Tauri v2)

**Project Type**: Desktop app (Rust backend + Vue frontend)

**Performance Goals**: Cálculo da projeção < 50 ms para dados típicos (≤ ~2000 transações, dezenas de parcelamentos)

**Constraints**: 100% local, offline; dinheiro sempre em `Decimal` (sem float); determinístico (não depende do relógio — âncora = mês de referência mais recente das faturas)

**Scale/Scope**: Usuário único; 1 componente de gráfico reutilizado em 2 telas; 1 função de domínio + campos de DTO

## Constitution Check

*GATE: aprovado.*

- **I. TDD**: `compute_card_forecast` é lógica de negócio pura → testes primeiro. Casos: parcela única restante, várias compras no mesmo mês, dedup entre faturas, última parcela, sem parcelas, estorno. Meta ≥90%.
- **II. Clean Architecture**: cálculo em `domain/forecast.rs` (puro, sem Tauri/SQLite); exposto via DTO existente (`YearSummary`/`DashboardData`); `commands` inalterado; frontend consome via `tauri.service`.
- **III. Simplicidade**: reaproveita a extração de parcelas que já existe (`transaction.installment`), sem novo comando nem schema.
- **IV. Integridade**: `Decimal` ponta a ponta; dedup por compra evita dupla contagem; serialização como string.
- **V. Local-first**: nenhuma rede.

Nenhuma violação.

## Project Structure

### Documentation (this feature)

```text
specs/005-card-payment-forecast/
├── plan.md              # Este arquivo
├── research.md          # Fase 0 — decisões (âncora, dedup, horizonte, onde no DTO)
├── data-model.md        # Fase 1 — entidades (ForecastPoint, ForecastItem)
├── quickstart.md        # Fase 1 — roteiro de validação manual
├── contracts/
│   └── forecast-dto.md  # Fase 1 — shape do DTO exposto ao frontend
└── tasks.md             # Fase 2 (/speckit-tasks — NÃO criado aqui)
```

### Source Code (repository root)

```text
src-tauri/src/
├── domain/
│   ├── forecast.rs        # NOVO — compute_card_forecast(invoices) -> Vec<ForecastPoint>  (+ testes)
│   ├── dashboard.rs       # + resumo da projeção no DashboardData
│   ├── year.rs            # + campo card_forecast no YearSummary
│   └── mod.rs             # reexporta forecast
├── application/
│   ├── get_dashboard.rs   # preenche o resumo da projeção
│   └── (year via command) # preenche card_forecast
└── (commands inalterados)

src/
├── components/dashboard/
│   └── CardForecastChart.vue   # NOVO — gráfico ECharts (barras por mês) reutilizável
├── pages/
│   ├── YearPage.vue            # + seção "Previsão do cartão" (gráfico completo)
│   └── DashboardPage.vue       # + card compacto "Próximos meses do cartão"
└── types/api.types.ts          # + ForecastPoint/ForecastItem nos DTOs
```

**Structure Decision**: App desktop existente (Rust + Vue). A projeção é uma função de domínio nova consumida por ambas as telas via os DTOs que já trafegam; um único componente de gráfico é reutilizado (completo no Ano, compacto no Mês).

## Complexity Tracking

Sem desvios da constituição — tabela não necessária.
