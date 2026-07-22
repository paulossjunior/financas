# Research: Cálculo rigoroso de inflação pessoal

**Feature**: `011-personal-inflation-calc` | **Plan**: [plan.md](plan.md)

Decisões de Fase 0 que sustentam o design. Cada item traz **decisão**,
**justificativa** e **alternativas rejeitadas**. Todas confirmadas pelo código
implementado em `domain/personal_inflation.rs` e `commands/inflation.rs`.

## 1. Tipos numéricos: dinheiro em `Decimal`, taxas em `f64`

- **Decisão**: valores monetários (`gasto`, `gasto_total`, `custo_atualizado`,
  `renda_corrigida`, `perda_poder_compra`, `consumo_adicional`) usam
  `rust_decimal::Decimal`, arredondados a 2 casas e serializados como **string**.
  As taxas (`inflacao`, `inflacao_pessoal`, `diferenca_pp`, `weight`, coeficiente)
  são `f64`.
- **Justificativa**: dinheiro exige aritmética decimal exata (Constituição IV) e não
  pode acumular erro binário. Já as taxas precisam de **potência fracionária** para a
  conversão de período por juro composto ((1+π)^(1/12)), operação que `Decimal` não
  oferece nativamente — daí `f64`. O cálculo em reais reconverte a taxa para `Decimal`
  apenas no momento de multiplicar pelo dinheiro e arredonda a 2 casas.
- **Alternativas rejeitadas**: (a) tudo em `f64` — viola a exigência de dinheiro exato;
  (b) tudo em `Decimal` — impossível/complicado para potências fracionárias e sem ganho
  de precisão em taxas percentuais.

## 2. Reuso da fonte de índices (006) vs coleta própria

- **Decisão**: o novo módulo faz **apenas o cálculo rico** e é puro; a coleta e o
  mapeamento categoria→grupo continuam em `domain/inflation` (grupos/headline em cache
  SQLite) e `map_category_to_group`. O comando `get_personal_inflation_detail` lê o
  cache existente e monta as `CategoryInput`.
- **Justificativa**: DRY (Constituição III) e local-first (V) — não duplicar a coleta
  nem abrir nova chamada de rede. 006 já baixa e persiste o IPCA (opt-in).
- **Alternativas rejeitadas**: duplicar fetch/parse no módulo novo — duplicação e novo
  ponto de rede; reprocessar faturas para pesos — o dashboard já agrega por categoria.

## 3. Diferença vs oficial: pontos percentuais, não percentual

- **Decisão**: `diferenca_pp = (inflacao_pessoal − inflacao_oficial) × 100`, rotulada
  como **ponto percentual (p.p.)**. Ex.: 7,7% vs 6% → **1,7 p.p.** (não 28%).
- **Justificativa**: comparar duas taxas é subtração em pontos percentuais; a razão
  (28%) seria enganosa. O `aviso` reforça a distinção p.p. × %.
- **Alternativas rejeitadas**: diferença relativa ((pessoal−oficial)/oficial) — induz
  a leitura errada; omitir o rótulo — ambíguo para leigo.

## 4. Modos de peso: atual vs base (cesta fixa)

- **Decisão**: `WeightMode::Current` (default) usa os gastos do período atual como
  numerador/denominador dos pesos; `WeightMode::Base` usa `base_gasto` (cesta fixa do
  período-base), exigindo `base_gasto` em todas as categorias (erro `MissingBaseGasto`
  caso falte). O modo usado é implícito na chamada.
- **Justificativa**: índice tipo Paasche (atual) reflete o consumo de hoje; Laspeyres
  (base fixa) é o correto para **comparação histórica** sem contaminar a variação com
  mudança de hábitos. Oferecer ambos cobre US4.
- **Alternativas rejeitadas**: só pesos atuais — impossibilita histórico consistente;
  escolher automaticamente — esconde a metodologia do usuário.

## 5. Acumulação por produto, não por soma

- **Decisão**: `accumulate(rates) = ∏(1+π) − 1`; nunca a soma das taxas. A inflação
  pessoal de um período é Σ(peso×inflação), mas a **série** de períodos acumula por
  produto.
- **Justificativa**: inflação compõe (ganho sobre ganho). Somar superestima/subestima
  conforme o sinal e não fecha com o custo real acumulado.
- **Alternativas rejeitadas**: soma simples — incorreta e verificável por teste
  (`accumulate_uses_product_not_sum`).

## 6. Categoria sem inflação: fallback com proveniência, nunca zero silencioso

- **Decisão**: quando a categoria não tem grupo do IPCA mapeado, usa-se o **IPCA geral**
  como taxa e registra-se a **proveniência** (ex.: "Sem grupo do IPCA para «X» — usou o
  IPCA geral.") no vetor `proveniencias` do resultado. Ausência total sem agregador seria
  erro — jamais assumir zero.
- **Justificativa**: SC-003 e FR-012 — zero silencioso falsearia a inflação pessoal para
  baixo sem o usuário saber. A substituição é explícita e auditável.
- **Alternativas rejeitadas**: assumir 0% — engana; descartar a categoria — distorce os
  pesos (Σ pesos ≠ 1) e some com gasto real.

## 7. Coeficiente comportamental: default 1,4, apenas simulação

- **Decisão**: `DEFAULT_BEHAVIORAL_COEFFICIENT = 1.4`; quando informado, `impacto =
  pessoal_pp × coeficiente` e `consumo_adicional = gasto_total × impacto`, sempre
  marcados como **simulação** e cobertos pelo `aviso` metodológico obrigatório. Ausente
  → campos comportamentais omitidos (None), resto idêntico.
- **Justificativa**: o coeficiente vem de estudo econométrico de outro contexto; é
  enriquecimento opcional (US3, P3) e não pode ser lido como previsão nem recomendação.
- **Alternativas rejeitadas**: embutir o comportamental no número principal — misturaria
  estimativa frágil com o cálculo determinístico; coeficiente fixo não configurável —
  menos flexível.

## 8. Periodicidade: mensal (per-categoria só existe mensal no cache)

- **Decisão**: o comando calcula sobre a **variação mensal** dos grupos do IPCA (único
  recorte por grupo disponível no cache de 006), oficial = headline do mês. Os helpers
  de conversão (`annual_to_monthly` ((1+a)^(1/12)−1), `monthly_to_annual` ((1+m)^12−1),
  `quarterly_to_monthly`) existem para não misturar períodos e nunca dividir anual por
  12; a acumulação por produto cobre a agregação de meses.
- **Justificativa**: FR-009/SC-004 — corretude metodológica. O cache expõe variação
  mensal por grupo; anualizar exigiria série por grupo, fora do escopo atual.
- **Alternativas rejeitadas**: dividir taxa anual por 12 — matematicamente errado
  (0,4868%/mês ≠ 0,5%); assumir mesma taxa em períodos diferentes — mistura inválida.

## Resumo

| Tema | Decisão |
|------|---------|
| Dinheiro / taxas | `Decimal` (2 casas, string) / `f64` |
| Fonte de índices | reuso de 006 (cache), módulo novo só calcula |
| Comparação | pontos percentuais (p.p.), com aviso p.p. × % |
| Pesos | atual (default) e base fixa (histórico) |
| Acumulação | produto ∏(1+π)−1, nunca soma |
| Dados ausentes | fallback IPCA geral + proveniência (nunca zero) |
| Comportamental | coeficiente default 1,4, simulação, opcional |
| Período | mensal; helpers compostos; sem dividir anual/12 |
