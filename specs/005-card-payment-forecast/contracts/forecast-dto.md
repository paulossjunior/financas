# Contract — Forecast DTO (frontend ⇄ backend)

Sem comando novo. A projeção viaja nos DTOs já existentes, retornados por
`get_year_summary_cmd` e `get_dashboard_cmd`. Campos de dinheiro são **strings** (Decimal).

## Tipos (TS — `src/types/api.types.ts`)

```ts
export interface ForecastItem {
  description: string;
  parcela: string;   // "2/5"
  amount: string;    // Decimal como string
}

export interface ForecastPoint {
  month: string;             // "YYYY-MM"
  amount: string;            // soma do mês (Decimal string)
  items: ForecastItem[];     // composição (maior primeiro)
}
```

## YearSummary (tela Ano — gráfico completo)

```ts
export interface YearSummary {
  // …campos existentes…
  card_forecast: ForecastPoint[];   // série contínua âncora+1 → última parcela ([] se vazio)
}
```

## DashboardData (tela Mês — resumo compacto)

```ts
export interface DashboardData {
  // …campos existentes…
  forecast_next: ForecastPoint[];       // próximos ~6 meses
  forecast_committed_total: string;     // total ainda a pagar (Decimal string)
  forecast_last_month: string;          // "YYYY-MM" | ""  (mês que zera)
}
```

## Struct Rust correspondente (`domain/forecast.rs`)

```rust
pub struct ForecastItem { pub description: String, pub parcela: String,
    #[serde(with = "rust_decimal::serde::str")] pub amount: Decimal }

pub struct ForecastPoint { pub month: String,
    #[serde(with = "rust_decimal::serde::str")] pub amount: Decimal,
    pub items: Vec<ForecastItem> }

pub fn compute_card_forecast(invoices: &[Invoice]) -> Vec<ForecastPoint>;
```

## Garantias

- `card_forecast` vazio ⇒ frontend mostra estado vazio ("sem parcelas futuras").
- `Σ card_forecast[].amount` == `installments_future_total` (mesma base, projeção só redistribui no tempo).
- Determinístico: âncora = mês de referência mais recente (não usa relógio).
