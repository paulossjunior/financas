# Quickstart: Categorias recorrentes + baseline + anti-duplicação

Cenários de validação executáveis. Detalhes de shapes em
[data-model.md](data-model.md) e [contracts/commands.md](contracts/commands.md);
requisitos em [spec.md](spec.md).

## Comandos

```bash
cd src-tauri && cargo test        # domínio puro (recurring.rs) + integração — TDD, ≥90% core
npm run test:run                  # Vitest (stores/serviço)
npx vue-tsc --noEmit              # type-check frontend
npm run tauri dev                 # app com hot-reload para validação manual
```

Regra TDD (constituição, Princípio I): escrever o teste **antes** da implementação, ver
falhar (red), implementar o mínimo (green), refatorar. Começar pelos testes de
`domain/recurring.rs`.

## Cenário 1 — Fixa derivada do dado real (US1, FR-002/003)

1. Marcar "Moradia & Serviços" como recorrente: `set_category_recurring("Moradia & Serviços", true)`.
2. Importar um extrato com um débito de aluguel de R$ 2.000 nessa categoria no mês.
3. Abrir o painel do mês.

**Esperado**: "Moradia & Serviços" aparece em `fixed_expenses` com `amount = "2000"`,
`origin = "extrato"`, `status = "realizado"`, **sem cadastro manual**. Categoria não
recorrente com lançamentos no extrato NÃO entra em `fixed_expenses`.

**Teste de domínio**: `derive_fixed_expenses` soma transações+bank debits da categoria no
mês; origem correta; categoria não-recorrente é ignorada.

## Cenário 2 — Anti-duplicação: importado supersede manual (US2, FR-005, SC-002)

1. Cadastrar um fixo manual "Aluguel R$ 2.000" (recorrente, categoria "Moradia & Serviços").
2. Importar o extrato com o mesmo aluguel no mesmo mês.
3. Abrir o painel.

**Esperado**: o aluguel conta **uma única vez** (valor do extrato). O manual aparece com
`status = "suprimido"` (riscado) e **não** soma em `net_total`. `DerivationResult`
devolve o id do manual suprimido.

**Contraprova (fallback)**: fixo manual em categoria recorrente **sem** importado
equivalente no mês (ex.: seguro em débito automático) → `status = "manual"`, contado
normalmente.

## Cenário 3 — Baseline para o Teto antes de importar (US3, FR-007/008/009)

1. Ter 3 meses de histórico importado das recorrentes (aluguel/água/luz).
2. No mês corrente, sem import: abrir o painel/ano.

**Esperado**: `fixed_expenses` usam a média dos 3 meses (`origin = "baseline"`,
`status = "estimado"`); `card_ceiling_is_baseline = true` → chip "base: média" no Teto
(DashboardPage e Year page).

3. Importar o extrato/fatura do mês.

**Esperado**: valor **realizado** substitui a média automaticamente; `is_baseline`/chip
somem. Com `< 3` meses, usa os disponíveis (`months_used = 1|2`); sem histórico, fixa = 0.

**Teste de domínio**: `baseline(n=3)` — média de 3 / disponíveis / zero.

## Cenário 4 — Vigência finita cai após o fim (US5, FR-012/013, SC-004)

1. `set_category_recurring("Saúde", true, startMonth="2026-01", endMonth="2026-03")`
   (compromisso "psicólogo" por 3 meses).
2. Abrir os painéis de fev/2026 e abr/2026.

**Esperado**: conta nas fixas de jan/fev/mar; em abr/2026 **não** aparece em
`fixed_expenses`, **não** entra no baseline nem no Teto — inclusive em recálculo
histórico (determinístico).

**Teste de domínio**: `is_active(month)` inclusivo jan..mar; excluído em abr; recompute
histórico idêntico.

## Cenário 5 — Detecção opt-in: aceitar / ignorar (US4, FR-010/011, SC-005)

1. Importar histórico com uma despesa que se repete ~mensal com valor parecido
   (ex.: Academia ~R$ 110 em ≥3 dos últimos 4 meses).
2. Abrir o Mapeamento: `recurring_suggestions()`.

**Esperado**: aparece uma sugestão (`kind="start"`, `avg≈"110"`, `months_seen≥3`).

3a. Clicar "Marcar" → `set_category_recurring(target, true)` → a categoria vira recorrente
    e as fixas passam a considerá-la; a sugestão some.
3b. Clicar "Ignorar" → `dismiss_recurring_suggestion(target)` → a sugestão some e **não
    reaparece** (persistida); nada é marcado.

**Teste de domínio**: `detect_recurring` respeita ≥3/4 + limite de variação (CV ≤ 0,15);
alvos dispensados são filtrados.

## Cenário 6 — Determinismo & idempotência (FR-015/018, SC-006)

1. Reabrir o app (ou reimportar o mesmo extrato/fatura).

**Esperado**: totais de fixas, baseline e Teto **idênticos**; nenhum lançamento
duplicado (dedup por id determinístico já existente); marcar/desmarcar recorrência
recalcula sem duplicar.

## Verificação de integração

- `cargo test` verde (novos testes de `domain/recurring.rs` + integração em
  `get_dashboard`/`compute_year_summary`).
- `npm run test:run` e `npx vue-tsc --noEmit` verdes (novos DTOs em `api.types.ts`).
- Em `npm run tauri dev`: Mapeamento com switch "Recorrente" + baseline/mês + origem +
  banner de sugestão; Fixos & Renda mostrando fixas derivadas (read-only, chips
  origem/status) + botão "adicionar fixo manual"; chip "base: média" no Teto quando estimado.
