# Research — Indicadores de inflação

## D1 — Fundamento (por que inflação pessoal)

**Decision**: Além do IPCA oficial, calcular a **inflação pessoal** reponderando as variações dos grupos do IPCA pelos pesos de gasto do usuário.

**Rationale**: Literatura (ECB, BIS, BBVA, NBER, ScienceDirect 2025) mostra que a inflação é heterogênea entre famílias e que o principal fator é a **composição da cesta** (escolha de categorias). O app tem os gastos por categoria → pode personalizar.

## D2 — Endpoints do IBGE (confirmados)

**Decision**:
- **IPCA geral (headline)** — agregado **1737**, período `-1`, localidade `N1[1]`, variáveis: `63` (variação mês), `2265` (acumulado no ano), `69` (acumulado 12 meses).
  `…/agregados/1737/periodos/-1/variaveis/63|2265|69?localidades=N1[1]`
- **IPCA por grupo** — agregado **7060**, período `-1`, variável `63` (mês), `classificacao=315[all]`.
  `…/agregados/7060/periodos/-1/variaveis/63?localidades=N1[1]&classificacao=315[all]`

Confirmado (jun/2026): geral 0,16; Alimentação e bebidas −0,24; Habitação +0,63; Artigos de residência +0,23; Vestuário +0,17; Transportes +0,17; Saúde e cuidados pessoais +0,23; Despesas pessoais +0,25; (Educação, Comunicação).

**Parsing**: da resposta de 7060, manter apenas as 9 categorias cujo nome bate com um dos **9 grupos oficiais** (rótulos únicos como "Alimentação e bebidas"); ignorar subgrupos/itens. `7169` = "Índice geral" (descartado — headline vem do 1737).

**Rationale**: dois agregados cobrem headline + grupos com uma variável cada; nomes de grupo são únicos → filtro por nome é robusto sem hardcode de códigos frágeis.

## D3 — Mapeamento categoria (app) → grupo (IPCA)

**Decision**: mapa por palavra-chave do nome da categoria:
- Alimentação, Lanche, Almoço, Mercado, Cerveja → **Alimentação e bebidas**
- Moradia, Aluguel, Energia, Água, Internet, Condomínio → **Habitação**
- Transporte, Combustível, Carro, Uber, Ônibus → **Transportes**
- Saúde, Farmácia, Remédio, Terapia, Plano → **Saúde e cuidados pessoais**
- Educação, Curso, Escola, Faculdade → **Educação**
- Assinaturas, Serviços de TI, Telefone, Celular → **Comunicação**
- Lazer, Viagem, Cachorros/Pet, Compras Online, Vestuário → **Despesas pessoais** (fallback amplo)
- Sem correspondência → **IPCA geral** (headline mês).

**Rationale**: aproxima a cesta do usuário aos 9 grupos; fallback ao geral evita distorção quando não há grupo.

## D4 — Cálculo da inflação pessoal

**Decision**: `pessoal_mês = Σ (peso_i × var_grupo(cat_i)) / Σ peso_i`, onde `peso_i` = gasto da categoria i (participação), `var_grupo` = variação mensal do grupo mapeado (ou geral no fallback). Reportar também a **diferença** (p.p.) para o IPCA geral do mês.

**Escopo v1**: inflação pessoal **mensal** (grupos trazem a variação do mês). 12 meses fica para v2.

**Rationale**: fiel à literatura; mensal é o dado por grupo disponível de forma simples.

## D5 — Cache local

**Decision**: tabela `inflation_cache` (linha única) com `payload` (JSON: headline + grupos + ref_month) e `fetched_at`. `save` faz upsert; `load` devolve o último. O `get_inflation` lê o cache e calcula o pessoal com as categorias atuais.

**Rationale**: offline por padrão (FR-005); exibe a data da última atualização (FR-006).

## D6 — Exceção de rede (local-first)

**Decision**: o fetch roda **no backend Rust** (reqwest, rustls-tls), disparado **só** pelo comando `fetch_ipca` (botão). Sem esse clique, nenhuma rede. Nenhum dado do usuário é enviado (apenas GET de índice público). Erro de rede → mensagem clara, cache preservado.

**Rationale**: única exceção ao Princípio V, minimizada e explícita. reqwest no backend evita mexer no CSP/capabilities do webview.

## Resolved unknowns

Todos resolvidos. Sem NEEDS CLARIFICATION pendente.
