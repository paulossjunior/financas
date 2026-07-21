# Implementation Plan: Categorias recorrentes + baseline + anti-duplicação

**Branch**: `010-recurring-categories` | **Date**: 2026-07-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/010-recurring-categories/spec.md`

## Summary

Marcar categorias como **recorrentes** (aluguel, água, luz, internet…) e derivar as
**contas fixas do mês** automaticamente dos lançamentos já importados (transações de
fatura + débitos do extrato) dessas categorias — a categoria recorrente vira a **fonte
de verdade**. Um fixo manual equivalente (mesma categoria + mês) é **suprimido** pelo
importado (mesmo padrão do contracheque → salário), evitando dupla contagem. Sem dado
importado no mês, usa-se um **baseline = média dos últimos 3 meses** (com selo
"estimado") para o Teto/projeção, substituído pelo realizado ao importar. Recorrências
finitas têm **vigência** (mês início → fim) e saem das fixas/baseline/Teto após o fim,
inclusive em recálculo histórico. Uma **detecção opt-in** sugere marcar categorias que
aparecem em ≥3 dos últimos 4 meses com pouca variação — o app sugere, o usuário confirma;
sugestões dispensadas ficam persistidas.

A recorrência é armazenada **por categoria** numa tabela nova (`recurring_categories`),
não em `category_rules` (ver [research.md](research.md)). A lógica de derivação é um
módulo de domínio puro novo (`domain/recurring.rs`), consumido por `get_dashboard` e
`compute_year_summary`.

## Technical Context

**Language/Version**: Rust 1.x (backend Tauri) + TypeScript / Vue 3 (`<script setup>`, Composition API).

**Primary Dependencies**: Tauri v2, `rusqlite`, `rust_decimal`, `chrono`, `uuid`, `serde` (todas já no projeto); Vue 3 + Pinia + Vite no frontend.

**Storage**: SQLite (`financas.db` em `app_data_dir`) via `rusqlite`. Duas tabelas novas: `recurring_categories` e `dismissed_recurring_suggestions`. Migrações idempotentes na inicialização (`CREATE TABLE IF NOT EXISTS`).

**Testing**: `cargo test` (domínio puro, TDD, ≥90% no core) + Vitest (`npm run test:run`) para stores/serviço. Type-check com `npx vue-tsc --noEmit`.

**Target Platform**: App desktop Tauri v2 (macOS / Windows).

**Project Type**: Aplicativo desktop (backend Rust + frontend Vue no mesmo repositório).

**Performance Goals**: Interativo; recálculo do painel a partir de dados locais já em memória (dezenas de meses / milhares de lançamentos). Sem metas de throughput especiais.

**Constraints**: Offline / local-first (nenhuma chamada de rede nesta feature). Dinheiro em `rust_decimal::Decimal`, serializado como **string** na fronteira IPC. Todos os cálculos **determinísticos** dado o mesmo conjunto de dados (reabrir o app / reimportar = mesmos números).

**Scale/Scope**: 1 usuário local; 4 comandos Tauri novos; 1 módulo de domínio novo; alterações em `get_dashboard`, `compute_year_summary` e 4 telas Vue. Sem mudança no modelo de dinheiro nem no princípio local-first.

## Constitution Check

*GATE: Deve passar antes da pesquisa (Fase 0). Reavaliar após o design (Fase 1).* → **PASS**.

- **I. TDD (NÃO-NEGOCIÁVEL)**: Toda a lógica nova mora em `domain/recurring.rs` (puro). Testes escritos primeiro (red → green → refactor) cobrindo: `derive_fixed_expenses` (soma por categoria, origem Extrato/Fatura, estornos reduzem o total), supersede de fixo manual, `baseline` (média de N=3 / meses disponíveis / zero sem histórico, flag `is_baseline`), vigência inclusiva jan..mar e exclusão após o fim (inclusive em recálculo histórico), `detect_recurring` (≥3/4 meses + limite de variação) e filtro de sugestões dispensadas. Meta de cobertura ≥90% no core. Integração adicional em `get_dashboard`/`compute_year_summary`.
- **II. Clean Architecture**: Respeita `commands → application → domain`; `infrastructure` só para I/O. `domain/recurring.rs` é **livre de Tauri e de DB** (recebe transações, `BankEntry`, `ManualEntry` e a lista de recorrentes como parâmetros). `application` (get_dashboard, year) orquestra; `infrastructure/db.rs` persiste as duas tabelas; `commands/recurring.rs` é fino. Frontend só chama `invoke` via `services/tauri.service.ts`.
- **III. Simplicidade & Otimização**: YAGNI — reusa o pipeline existente de `ManualAgg` e o padrão de supersede do contracheque em vez de criar uma agregação paralela. Sem abstrações especulativas. Uma tabela dedicada por categoria (chave simples) em vez de colunas espalhadas em `category_rules`.
- **IV. Integridade de Dados**: Dinheiro em `Decimal` exato; estornos reduzem o total da fixa do mês; anti-duplicação garante contagem única; resultados determinísticos. Meses inválidos e valores decimais corrompidos tratados explicitamente (log, nunca silêncio), coerente com `parse_money`/`parse_month_start` já existentes.
- **V. Local-First & Privacidade**: Nenhuma chamada de rede. Tudo lido/gravado em `financas.db` local. Sem telemetria.

Sem violações a registrar em Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/010-recurring-categories/
├── plan.md              # Este arquivo
├── research.md          # Decisões (Fase 0)
├── data-model.md        # Entidades, DTOs, transições de estado (Fase 1)
├── quickstart.md        # Cenários de validação (Fase 1)
├── contracts/
│   └── commands.md      # Contratos dos comandos Tauri + DTOs alterados
├── checklists/          # Checklists de qualidade (pré-existente)
└── tasks.md             # Fase 2 (/speckit-tasks — NÃO criado por /speckit-plan)
```

### Source Code (repository root)

```text
src-tauri/src/
├── domain/
│   ├── recurring.rs          # NOVO — puro: derive_fixed_expenses, baseline,
│   │                         #   supersede, vigência, detect_recurring (+ testes)
│   ├── recurring.rs (mod)    # registrar em domain/mod.rs; re-exportar DTOs
│   ├── dashboard.rs          # + campos is_baseline / fixed derivadas no DashboardData
│   ├── year.rs               # compute_year_summary usa fixas derivadas + baseline p/ Teto
│   ├── manual_entry.rs       # (reuso) fonte dos fixos manuais / supersede
│   ├── bank_statement.rs     # (reuso) BankEntry — débitos alimentam a derivação
│   └── categorizer.rs        # (reuso) categorias vêm daqui
├── application/
│   └── get_dashboard.rs      # deriva fixas via domínio (realizado > baseline), anti-dup, vigência
├── infrastructure/
│   └── db.rs                 # + recurring_categories + dismissed_recurring_suggestions
│                             #   (CRUD + migração idempotente no init())
├── commands/
│   ├── recurring.rs          # NOVO — set_category_recurring, list_recurring_categories,
│   │                         #   recurring_suggestions, dismiss_recurring_suggestion
│   ├── dashboard.rs          # (get_dashboard_cmd / get_year_summary_cmd — passam dados crus)
│   └── mod.rs                # + pub mod recurring
└── lib.rs                    # registrar os 4 comandos novos no invoke_handler

src/
├── pages/
│   ├── MappingPage.vue       # switch "Recorrente" + baseline/mês + origem + banner de sugestão
│   ├── ManualEntriesPage.vue # Fixos & Renda: fixas DERIVADAS (read-only, chip origem/status)
│   │                         #   + botão "adicionar fixo manual"
│   ├── DashboardPage.vue     # chip "base: média" no Teto quando estimado
│   └── (Year page)           # mesmo chip "base: média" no Teto anual
├── services/tauri.service.ts # únicos invokes dos 4 comandos novos
├── types/api.types.ts        # DTOs espelhando Rust (RecurringCategory, DerivedFixedExpense,
│                             #   RecurringSuggestion, + campos is_baseline nos DTOs de painel)
└── stores/                   # store(s) que consomem os novos serviços
```

**Structure Decision**: App desktop de projeto único (backend `src-tauri/`, frontend `src/`),
seguindo o layering existente `commands → application → domain` com `infrastructure` para I/O.
A lógica nova concentra-se em `domain/recurring.rs` (puro, testável isolado); persistência em
`infrastructure/db.rs`; um arquivo fino `commands/recurring.rs` expõe os 4 comandos. O frontend
mantém a regra de que só `services/tauri.service.ts` chama `invoke`.

## Complexity Tracking

> Preencher SOMENTE se o Constitution Check tiver violações a justificar.

Sem violações. O reuso do padrão de supersede (contracheque → salário) e do pipeline de
`ManualAgg` evita nova lógica de agregação; a tabela dedicada por categoria mantém a chave
simples sem inflar `category_rules`.
