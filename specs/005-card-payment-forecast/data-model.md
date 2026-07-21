# Data Model — Previsão de pagamento do cartão

Entidades derivadas (calculadas em memória a partir das transações; nada novo persistido).

## InstallmentPurchase (interno, pós-dedup)

Uma compra parcelada única, após deduplicar ocorrências entre faturas (ver research D2).

| Campo | Tipo | Descrição |
|------|------|-----------|
| description | String | Descrição da compra (normalizada para a chave de dedup) |
| current | u8 | Nº da parcela mais recente conhecida |
| total | u8 | Total de parcelas |
| amount | Decimal | Valor de uma parcela |
| ref_month | String `YYYY-MM` | Mês de referência da parcela `current` |

Derivado: `remaining = total − current`; parcela `current+k` cai em `ref_month + k` (k = 1..remaining).

## ForecastItem

Uma parcela que compõe um mês futuro (composição — US2).

| Campo | Tipo | Descrição |
|------|------|-----------|
| description | String | Descrição da compra |
| parcela | String | Rótulo "x/y" da parcela que cai nesse mês |
| amount | String (Decimal) | Valor da parcela |

## ForecastPoint

Um mês futuro da projeção.

| Campo | Tipo | Descrição |
|------|------|-----------|
| month | String `YYYY-MM` | Mês projetado |
| amount | String (Decimal) | Soma das parcelas que caem no mês |
| items | ForecastItem[] | Parcelas que compõem o mês (desc → maior primeiro) |

## Agregados de resumo (tela Mês)

Expostos junto ao `DashboardData`:

| Campo | Tipo | Descrição |
|------|------|-----------|
| forecast_next | ForecastPoint[] | Próximos ~6 meses (subconjunto da série completa) |
| forecast_committed_total | String (Decimal) | Total ainda a pagar (= soma de todas as parcelas futuras) |
| forecast_last_month | String `YYYY-MM` \| "" | Mês da última parcela (quando o compromisso zera); vazio se não há parcelas |

## Invariantes

- `Σ ForecastPoint.amount` (série completa) = soma de `remaining × amount` de todas as compras deduplicadas = consistente com `installments_future_total` (SC-002).
- Série contínua de `âncora+1` até `forecast_last_month` (meses sem parcela = amount 0).
- Transações com `is_reversal` não entram.
- Lista vazia quando não há parcelamentos em aberto.
