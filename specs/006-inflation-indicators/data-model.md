# Data Model — Indicadores de inflação

## IpcaGroup

Variação mensal de um grupo do IPCA (do agregado 7060).

| Campo | Tipo | Descrição |
|------|------|-----------|
| name | String | Nome do grupo ("Alimentação e bebidas"…) |
| month_var | String (Decimal) | Variação do mês (%) |

## IpcaHeadline

IPCA oficial do último período (agregado 1737).

| Campo | Tipo | Descrição |
|------|------|-----------|
| ref_month | String `YYYY-MM` | Mês de referência |
| month | String (Decimal) | Variação do mês (%) |
| year | String (Decimal) | Acumulado no ano (%) |
| twelve | String (Decimal) | Acumulado em 12 meses (%) |

## InflationCache (persistido — tabela `inflation_cache`)

| Campo | Tipo | Descrição |
|------|------|-----------|
| headline | IpcaHeadline | IPCA oficial |
| groups | IpcaGroup[] | 9 grupos com variação do mês |
| fetched_at | String (ISO) | Quando foi baixado |

## InflationData (DTO devolvido ao frontend)

| Campo | Tipo | Descrição |
|------|------|-----------|
| available | bool | Há cache? (senão, estado vazio) |
| headline | IpcaHeadline \| null | IPCA oficial |
| groups | IpcaGroup[] | Grupos (para exibir/inspecionar) |
| personal_month | String (Decimal) | Inflação pessoal do mês (reponderada) |
| personal_diff | String (Decimal) | Diferença p.p. vs IPCA geral do mês (pessoal − geral) |
| fetched_at | String | Data da última atualização |

## Invariantes

- `available=false` ⇒ frontend mostra estado vazio ("Atualize os índices").
- Σ pesos usados na inflação pessoal = 100% dos gastos considerados (SC-004).
- Categoria sem grupo mapeado usa a variação geral do mês.
- Sem gastos ⇒ `personal_month == headline.month` e `personal_diff == 0`.
- Todos os percentuais em `Decimal` (string na serialização).
