# Contract — Inflation (frontend ⇄ backend)

Dois comandos Tauri novos. Dinheiro/percentuais como **string** (Decimal).

## Comandos

```
fetch_ipca() -> InflationData        // OPT-IN: faz GET no IBGE, salva cache, calcula pessoal, retorna. Erro → Err(msg), cache preservado.
get_inflation() -> InflationData     // Offline: lê o cache e calcula o pessoal com as categorias atuais. available=false se nunca atualizou.
```

`fetch_ipca` é a **única** operação de rede do app e só roda por ação explícita do usuário (botão "Atualizar índices").

## Tipos (TS — `src/types/api.types.ts`)

```ts
export interface IpcaGroup { name: string; month_var: string; }

export interface IpcaHeadline {
  ref_month: string;   // "YYYY-MM"
  month: string; year: string; twelve: string;   // % (Decimal string)
}

export interface InflationData {
  available: boolean;
  headline: IpcaHeadline | null;
  groups: IpcaGroup[];
  personal_month: string;   // % reponderada
  personal_diff: string;    // p.p. vs geral do mês
  fetched_at: string;       // "" se sem cache
}
```

## Struct Rust (`domain/inflation.rs`)

```rust
pub struct IpcaGroup { pub name: String, #[serde(with="rust_decimal::serde::str")] pub month_var: Decimal }
pub struct IpcaHeadline { pub ref_month: String, /* month/year/twelve as Decimal str */ }
pub struct InflationData { pub available: bool, /* … */ }

// Puro, testável:
pub fn compute_personal_inflation(categories: &[(String, Decimal)], groups: &[IpcaGroup], general_month: Decimal) -> (Decimal, Decimal);
pub fn map_category_to_group(cat: &str) -> Option<&'static str>;
```

## Garantias

- Nenhum dado do usuário é enviado (só GET de índice público).
- `get_inflation` nunca faz rede; `fetch_ipca` é o único ponto de rede.
- Falha de rede em `fetch_ipca` → `Err(mensagem)`, sem apagar o cache.
