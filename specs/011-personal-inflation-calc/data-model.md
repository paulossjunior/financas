# Data Model: Cálculo rigoroso de inflação pessoal

**Feature**: `011-personal-inflation-calc` | **Plan**: [plan.md](plan.md)

Entidades do domínio puro (`src-tauri/src/domain/personal_inflation.rs`) e o DTO
espelhado no frontend (`src/types/api.types.ts`). **Convenção de unidades**: campos de
**dinheiro** são `Decimal` (2 casas, serializados como **string**); campos de **taxa**
são `f64` — como fração decimal (0,077 = 7,7%) salvo os que terminam em `_pct`/`_pp`,
que já vêm em pontos percentuais.

## Entidades de entrada

### `CategoryInput` (entrada, não serializada)

Uma linha de gasto por categoria. Construída pela camada de aplicação.

| Campo | Tipo | Unidade | Regras |
|-------|------|---------|--------|
| `category` | `String` | — | não vazio; **único** na lista (duplicata → erro) |
| `gasto` | `Decimal` | dinheiro | ≥ 0 (negativo → erro); gasto do período atual |
| `base_gasto` | `Option<Decimal>` | dinheiro | obrigatório em `WeightMode::Base`; senão pode ser `None` |
| `inflacao` | `f64` | taxa (fração) | pode ser 0 ou negativa (deflação) |
| `provenance` | `Option<String>` | — | preenchido quando a inflação foi emprestada de um grupo agregador |

Construtor `CategoryInput::new(category, gasto, inflacao)` → `base_gasto = None`,
`provenance = None`.

### `WeightMode` (enum, serde `snake_case`)

| Variante | Semântica |
|----------|-----------|
| `Current` (default) | pesos = gasto atual / Σ gasto atual (índice tipo Paasche) |
| `Base` | pesos = `base_gasto` / Σ `base_gasto` (cesta fixa, tipo Laspeyres) |

## Parâmetros de `compute(...)`

`compute(categories, inflacao_oficial: f64, renda: Decimal, coeficiente: Option<f64>, weight_mode: WeightMode) -> Result<PersonalInflationResult, PersonalInflationError>`

- `inflacao_oficial`: taxa oficial do período (fração).
- `renda`: renda do período (dinheiro).
- `coeficiente`: coeficiente comportamental; `None` omite a simulação.

## Entidades de saída

### `Contribution` (serializável)

| Campo | Tipo | Unidade | Notas |
|-------|------|---------|-------|
| `category` | `String` | — | nome da categoria |
| `weight` | `f64` | fração | peso da categoria; Σ pesos = 1 (tolerância de ponto flutuante) |
| `inflacao` | `f64` | fração | inflação atribuída à categoria |
| `contribuicao` | `f64` | fração | `weight × inflacao`; a lista é **ordenada** por contribuição desc |

### `PersonalInflationResult` (DTO rico, serializável)

| Campo | Tipo | Unidade | Definição |
|-------|------|---------|-----------|
| `gasto_total` | `Decimal` (str) | dinheiro | Σ `gasto` das categorias (>0) |
| `inflacao_pessoal` | `f64` | fração | Σ `contribuicao` |
| `inflacao_pessoal_pct` | `f64` | % | `inflacao_pessoal × 100` |
| `inflacao_oficial` | `f64` | fração | parâmetro de entrada |
| `diferenca_pp` | `f64` | p.p. | `(pessoal − oficial) × 100` |
| `custo_atualizado` | `Decimal` (str) | dinheiro | `gasto_total × (1 + pessoal)` (2 casas) |
| `aumento_cesta` | `Decimal` (str) | dinheiro | `custo_atualizado − gasto_total` |
| `renda_corrigida` | `Decimal` (str) | dinheiro | `renda × (1 + pessoal)` |
| `aumento_renda` | `Decimal` (str) | dinheiro | `renda_corrigida − renda` |
| `renda_corrigida_consumo` | `Decimal` (str) | dinheiro | variante conservadora: `renda + perda_poder_compra` |
| `perda_poder_compra` | `Decimal` (str) | dinheiro | `gasto_total × pessoal` |
| `impacto_comportamental` | `Option<f64>` | fração | `pessoal_pp × coef / 100`; `None` sem coeficiente |
| `impacto_comportamental_pct` | `Option<f64>` | % | `pessoal_pp × coef` (ex.: 7,7×1,4 = 10,78) |
| `consumo_adicional` | `Option<Decimal>` (str) | dinheiro | `gasto_total × impacto_comportamental` |
| `contribuicoes` | `Vec<Contribution>` | — | ordenada desc por `contribuicao` |
| `proveniencias` | `Vec<String>` | — | substituições feitas (fallback de inflação) |
| `aviso` | `String` | — | `METHODOLOGY_NOTE` (aviso metodológico obrigatório) |

### `PersonalInflationError`

| Variante | Quando | Mensagem (`Display`) |
|----------|--------|----------------------|
| `EmptyCategories` | lista vazia | "nenhuma categoria informada" |
| `NonPositiveTotal` | Σ gasto ≤ 0 (ou Σ base ≤ 0 em `Base`) | "gasto total deve ser maior que zero" |
| `NegativeGasto(cat)` | algum `gasto` < 0 | "gasto negativo em '{cat}'" |
| `DuplicateCategory(cat)` | categoria repetida | "categoria duplicada: '{cat}'" |
| `MissingBaseGasto(cat)` | `WeightMode::Base` sem `base_gasto` | "gasto do período-base ausente em '{cat}'" |

## Constantes

- `DEFAULT_BEHAVIORAL_COEFFICIENT: f64 = 1.4` — coeficiente default (simulação).
- `METHODOLOGY_NOTE: &str` — texto do aviso: estimativa; coeficiente de outro contexto;
  não é previsão nem recomendação.

## Regras de validação (resumo)

- `gasto ≥ 0`; `gasto_total > 0`; categorias únicas; em `Base`, `base_gasto` presente e Σ > 0.
- `inflacao` livre (0/negativa válidas). `coeficiente` opcional.
- Σ pesos = 1 e Σ contribuições = `inflacao_pessoal`, ambos dentro de tolerância de
  ponto flutuante (testes usam `EPS = 1e-9`).
- Dinheiro arredondado a 2 casas; resultado determinístico.

## Espelho no frontend (`src/types/api.types.ts`)

```ts
export interface InflationContribution {
  category: string;
  weight: number;        // fração
  inflacao: number;      // fração
  contribuicao: number;  // fração
}

export interface PersonalInflationDetail {
  gasto_total: string;              // dinheiro (Decimal string)
  inflacao_pessoal: number;         // fração
  inflacao_pessoal_pct: number;     // %
  inflacao_oficial: number;         // fração
  diferenca_pp: number;             // pontos percentuais
  custo_atualizado: string;         // dinheiro
  aumento_cesta: string;            // dinheiro
  renda_corrigida: string;          // dinheiro
  aumento_renda: string;            // dinheiro
  renda_corrigida_consumo: string;  // dinheiro (variante conservadora)
  perda_poder_compra: string;       // dinheiro
  impacto_comportamental: number | null;      // fração | null
  impacto_comportamental_pct: number | null;  // % | null
  consumo_adicional: string | null;           // dinheiro | null
  contribuicoes: InflationContribution[];
  proveniencias: string[];
  aviso: string;
}
```

**Regra do frontend**: valores de dinheiro chegam como string; usar `parseFloat`
apenas para exibição. Taxas em `_pct`/`_pp` já vêm em pontos percentuais; as demais
são fração (multiplicar por 100 para exibir como %).
