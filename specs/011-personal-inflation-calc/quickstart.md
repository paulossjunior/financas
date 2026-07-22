# Quickstart: Cálculo rigoroso de inflação pessoal

**Feature**: `011-personal-inflation-calc` | **Plan**: [plan.md](plan.md)

Como validar a feature. O cálculo é domínio puro — a maior parte da validação é por
teste unitário (`cargo test`). Os comandos abaixo assumem a raiz do repositório.

## Comandos

```bash
cd src-tauri && cargo test personal_inflation   # 15 testes do módulo puro
npm run test:run                                # Vitest (tipos/serviço/componente)
npm run tauri dev                               # rodar o app e ver as contribuições no Ano
npx vue-tsc --noEmit                            # type-check do frontend
```

## Cenários de validação (domínio)

Cada cenário corresponde a um teste em `domain/personal_inflation.rs`.

### 1. Exemplo de referência (`reference_example_matches_spec`)

Entradas: Alimentação R$2.000/10%, Transporte R$1.500/8%, Habitação R$1.000/5%,
Outros R$500/3%; oficial 6%; renda R$7.000; coeficiente 1,4; pesos atuais.

Espera **exatamente**:
- inflação pessoal = **7,7%** (0,077)
- diferença = **1,7 p.p.**
- custo da cesta = **R$5.385** (aumento **R$385**)
- renda corrigida = **R$7.539** (aumento **R$539**)
- perda de poder de compra = **R$385**
- impacto comportamental = **10,78%** e consumo adicional = **R$539**

### 2. Categoria única (`single_category`)

Uma categoria com inflação X → inflação pessoal = X e peso = 100%.

### 3. Zero e deflação (`zero_inflation`, `deflation_reduces_personal`)

- Todas com inflação 0 → pessoal = 0; aumento da cesta = R$0,00.
- Categoria com inflação negativa reduz a inflação pessoal (−0,01 + 0,01 = 0).

### 4. Dados ausentes → proveniência (`provenance_is_reported`)

Categoria cuja inflação foi emprestada de um grupo agregador registra a proveniência
em `proveniencias` (ex.: "usou Transportes para Combustível") — **nunca** zero
silencioso.

### 5. Conversão de período composta (`period_conversion_compound_not_divided`)

`annual_to_monthly(0.06) ≈ 0,4868%` (não 6%/12 = 0,5%); estritamente menor que a
divisão ingênua; `monthly_to_annual` faz o round-trip de volta a 6%.

### 6. Acumulação por produto (`accumulate_uses_product_not_sum`)

`accumulate([0.01, 0.02, 0.005]) = 1,01×1,02×1,005 − 1`, diferente da soma.

### 7. Pesos base vs atuais (`base_weights_differ_from_current`)

Mesmas categorias, mesmos gastos: `WeightMode::Current` (gastos atuais) dá 0,09 e
`WeightMode::Base` (cesta fixa via `base_gasto`) dá 0,05 — o modo muda o resultado.

### 8. Erros (`zero_total_is_error`, `negative_and_duplicate_are_errors`, `empty_is_error`, `missing_base_gasto_is_error`)

Total ≤ 0, gasto negativo, categoria duplicada, lista vazia e `base_gasto` ausente em
modo `Base` → erros explícitos (`PersonalInflationError`), nunca resultado silencioso.

### 9. Aviso metodológico (`methodology_note_present`)

Toda saída `Ok` inclui `aviso` não vazio contendo "estimativa".

## Validação no app (comando + UI)

1. `npm run tauri dev`; garanta que os índices IPCA estão em cache (fluxo de 006:
   botão "Atualizar índices" na tela Ano) e que há gastos importados.
2. Invocar `get_personal_inflation_detail` (via a tela Ano quando o componente de
   contribuições estiver integrado) deve retornar o DTO com as contribuições ordenadas,
   a diferença em p.p., os impactos em reais e o aviso.
3. Sem cache de índices ou sem gastos → retorno `null` (UI mostra estado vazio).
4. Offline: nenhuma chamada de rede é feita por este comando.
