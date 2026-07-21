# Implementation Plan: Indicadores de inflação (IPCA + inflação pessoal)

**Branch**: `006-inflation-indicators` | **Date**: 2026-07-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/006-inflation-indicators/spec.md`

## Summary

Mostrar o IPCA oficial (mês/ano/12m) e a **inflação pessoal** do usuário (variações dos grupos do IPCA reponderadas pelos pesos de gasto das categorias) nas telas **Ano** (card completo, com botão "Atualizar índices") e **Mês** (resumo compacto). O fetch do IBGE é **opt-in** no backend Rust; os índices são **salvos localmente** (SQLite) e usados offline. O cálculo da inflação pessoal é função pura de domínio (TDD).

## Technical Context

**Language/Version**: Rust 1.75+ (Tauri) · TypeScript 5 / Vue 3

**Primary Dependencies**: `reqwest` (rustls-tls, json) para o fetch; `serde_json`, `rust_decimal`, `rusqlite` (já no projeto); frontend `vue-echarts`/Pinia

**Storage**: SQLite. **Nova tabela** `inflation_cache` (linha única: payload JSON + fetched_at). Sem tocar nas demais.

**Testing**: `cargo test` (cálculo puro + mapeamento, TDD ≥90%) · Vitest (componente)

**Target Platform**: Desktop macOS + Windows (Tauri v2)

**Project Type**: Desktop app (Rust backend + Vue frontend)

**Performance Goals**: Fetch < 5 s em conexão comum; cálculo pessoal < 20 ms

**Constraints**: Offline por padrão — rede **só** no clique "Atualizar índices"; nenhum dado pessoal transmitido; dinheiro/percentuais com `Decimal`

**Scale/Scope**: Usuário único; 1 comando de fetch + 1 de leitura; 1 componente reutilizado em 2 telas

## Constitution Check

*GATE: aprovado (com exceção justificada ao Princípio V).*

- **I. TDD**: `compute_personal_inflation` + `map_category_to_group` são puros → testes primeiro (mapeamento, reponderação, categoria sem grupo, sem gastos, invariante de pesos). ≥90%.
- **II. Clean Architecture**: `domain/inflation.rs` (puro); `infrastructure/ibge.rs` (HTTP) e `infrastructure/db.rs` (cache) fazem I/O; `commands/inflation.rs` fino.
- **III. Simplicidade**: reusa categorias já agregadas; 1 tabela nova de cache; sem reprocessar faturas.
- **IV. Integridade**: `Decimal` nos percentuais/pesos; serialização como string.
- **V. Local-first & Privacy** — *exceção justificada*: é a primeira chamada de rede do app. Mitigações: **opt-in explícito** (nunca automático), **somente leitura** de índice público, **nenhum dado pessoal enviado**, e **offline por padrão** via cache. Sem o clique, o comportamento local-first é idêntico ao de hoje.

## Project Structure

### Documentation (this feature)

```text
specs/006-inflation-indicators/
├── plan.md
├── research.md          # Fase 0 — endpoints IBGE, mapeamento, cache, cálculo, exceção de rede
├── data-model.md        # Fase 1 — InflationData / IpcaGroup / cache / personal
├── quickstart.md        # Fase 1 — validação (TDD + fetch manual)
├── contracts/
│   └── inflation-dto.md # Fase 1 — comandos fetch_ipca / get_inflation + DTO
└── tasks.md             # Fase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
src-tauri/src/
├── domain/
│   ├── inflation.rs       # NOVO — tipos + compute_personal_inflation + map_category_to_group (+ testes)
│   └── mod.rs             # reexporta inflation
├── infrastructure/
│   ├── ibge.rs            # NOVO — fetch IPCA geral (1737) + grupos (7060) via reqwest
│   └── db.rs              # + inflation_cache (save/load) + migração da tabela
├── commands/
│   ├── inflation.rs       # NOVO — fetch_ipca (async, opt-in) + get_inflation (cache + pessoal)
│   └── mod.rs
└── lib.rs                 # registra os comandos

src/
├── components/dashboard/
│   └── InflationCard.vue  # NOVO — IPCA + inflação pessoal; prop compact; botão Atualizar
├── pages/
│   ├── YearPage.vue       # + card completo (com botão Atualizar índices)
│   └── DashboardPage.vue  # + resumo compacto
├── services/tauri.service.ts   # + fetchIpca() / getInflation()
└── types/api.types.ts     # + InflationData / IpcaGroup
```

**Structure Decision**: domínio puro para o cálculo; infraestrutura isola o único ponto de rede (IBGE) e o cache local; um componente de UI reutilizado (Ano completo, Mês compacto).

## Complexity Tracking

| Desvio | Por quê | Alternativa rejeitada |
|--------|---------|-----------------------|
| Chamada de rede (Princípio V) | Índices oficiais só existem online; valor pra decisão financeira | Entrada 100% manual — rejeitada por atrito; fetch é opt-in + cacheado, preserva offline |
