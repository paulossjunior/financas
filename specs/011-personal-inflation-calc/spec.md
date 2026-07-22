# Feature Specification: Cálculo rigoroso de inflação pessoal

**Feature Branch**: `011-personal-inflation-calc`

**Created**: 2026-07-22

**Status**: Draft

**Input**: User description: cálculo de inflação pessoal por categoria (pesos × variações), comparação com o oficial, custo da cesta, renda corrigida, perda de poder de compra, impacto comportamental opcional, compatibilidade de período, pesos atuais/base, histórico acumulado, dados ausentes com proveniência, validações e explicação textual — consolidando o que já existe (006/007).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Inflação pessoal e principais contribuições (Priority: P1)

Como usuário, quero ver **a minha inflação pessoal** do período e **quais categorias mais contribuíram**, para entender por que meus preços subiram mais (ou menos) que a média.

**Why this priority**: É o núcleo — sem a inflação pessoal e as contribuições, nada mais tem sentido. Já existe uma versão simples (006); esta a torna correta e explicável.

**Independent Test**: Com os gastos por categoria e a inflação de cada categoria, verificar que a inflação pessoal = Σ(peso×inflação) e que a lista de contribuições (peso, inflação, contribuição) soma exatamente a inflação pessoal, ordenada da maior para a menor.

**Acceptance Scenarios**:

1. **Given** Alimentação R$2.000/10%, Transporte R$1.500/8%, Habitação R$1.000/5%, Outros R$500/3%, **When** calculo, **Then** inflação pessoal = **7,7%** e a maior contribuição é Alimentação (4,0 p.p.).
2. **Given** uma única categoria com inflação X, **When** calculo, **Then** inflação pessoal = X e peso = 100%.
3. **Given** todas as categorias com inflação 0, **When** calculo, **Then** inflação pessoal = 0.
4. **Given** uma categoria com inflação negativa (deflação), **When** calculo, **Then** a contribuição dela reduz a inflação pessoal.

---

### User Story 2 - Comparação com o oficial + impacto em reais (Priority: P1)

Como usuário, quero comparar minha inflação com a **oficial** e ver **em reais** o que isso significa (cesta e renda), para agir.

**Why this priority**: Transforma a taxa em decisão. Depende de US1.

**Independent Test**: Com inflação pessoal 7,7% e oficial 6%, verificar diferença = **1,7 ponto percentual** (não 28%); custo atualizado da cesta de R$5.000 = **R$5.385** (aumento R$385); renda corrigida de R$7.000 = **R$7.539** (aumento R$539); perda de poder de compra sobre o consumo = R$385.

**Acceptance Scenarios**:

1. **Given** pessoal 7,7% e oficial 6%, **When** comparo, **Then** a diferença é apresentada em **pontos percentuais** (1,7 p.p.), com aviso de não confundir p.p. e %.
2. **Given** gastoTotal R$5.000 e pessoal 7,7%, **When** calculo, **Then** custo atualizado = R$5.385 e aumento da cesta = R$385.
3. **Given** renda R$7.000 e pessoal 7,7%, **When** calculo, **Then** renda corrigida = R$7.539 e aumento necessário = R$539; há também a variante conservadora (só sobre o consumo).

---

### User Story 3 - Impacto comportamental (simulação, opcional) (Priority: P3)

Como usuário curioso, quero uma **simulação** de quanto meu consumo poderia variar por causa da inflação pessoal, deixando claro que é só estimativa.

**Why this priority**: Enriquecimento opcional; não bloqueia o resto.

**Independent Test**: Com pessoal 7,7% e coeficiente 1,4, verificar impacto = 7,7×1,4 = **10,78%** e consumo adicional sobre R$5.000 = **R$539**, sempre rotulado como simulação.

**Acceptance Scenarios**:

1. **Given** coeficiente informado (default 1,4), **When** calculo, **Then** impacto = pessoal_pp × coeficiente e o resultado é marcado como **simulação**.
2. **Given** coeficiente ausente, **When** calculo, **Then** os campos comportamentais ficam vazios/omitidos e o resto do cálculo é idêntico.

---

### User Story 4 - Períodos, pesos e histórico corretos (Priority: P2)

Como usuário, quero que taxas de períodos diferentes **não** sejam misturadas e que o histórico acumule corretamente.

**Why this priority**: Corretude metodológica — resultados errados corroem a confiança.

**Independent Test**: Converter 6% ao ano → ~0,4868% ao mês por juro composto (não 0,5%); recusar/avisar mistura de períodos; acumular meses por produto ∏(1+π)−1 (não soma).

**Acceptance Scenarios**:

1. **Given** inflação anual 6%, **When** converto para mensal, **Then** ≈ 0,4868% (composto), nunca 6%/12.
2. **Given** taxas em períodos incompatíveis, **When** calculo sem converter, **Then** erro/aviso explícito.
3. **Given** meses 1%/2%/0,5%, **When** acumulo, **Then** = (1,01×1,02×1,005)−1, não a soma.
4. **Given** pesos de período-base vs pesos atuais, **When** calculo, **Then** o método usado é informado e o resultado reflete o método escolhido.

---

### Edge Cases

- **Gasto total zero** → erro (não dividir por zero).
- **Gasto negativo** → erro.
- **Categoria sem inflação** → erro, **nunca** zero silencioso; se houver categoria agregadora, usa-a e **registra a proveniência** na saída ("usou Transportes para Combustível").
- **Categorias duplicadas** → erro/tratamento explícito, não somar silenciosamente.
- **Inflação 0 ou negativa** → válida.
- **Ponto flutuante** → comparar com tolerância.
- **Coeficiente ausente** → seção comportamental omitida.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST calcular o **peso** de cada categoria = gasto_categoria / gastoTotal, com Σ pesos = 1.
- **FR-002**: O sistema MUST calcular a **contribuição** de cada categoria = peso × inflação_categoria e retornar a lista **ordenada** (maior contribuição primeiro).
- **FR-003**: O sistema MUST calcular a **inflação pessoal** = Σ contribuições.
- **FR-004**: O sistema MUST calcular a **diferença vs inflação oficial em pontos percentuais** e deixar explícita a distinção entre ponto percentual e percentual.
- **FR-005**: O sistema MUST calcular **custo atualizado da cesta** = gastoTotal×(1+pessoal) e o **aumento** da cesta.
- **FR-006**: O sistema MUST calcular **renda corrigida** = renda×(1+pessoal) e o **aumento necessário**, e MUST oferecer a **variante conservadora** (inflação só sobre o consumo).
- **FR-007**: O sistema MUST calcular a **perda de poder de compra** sobre o consumo = gastoTotal×pessoal.
- **FR-008**: O sistema MUST, quando um **coeficiente comportamental** for informado (default 1,4, configurável), calcular impacto = pessoal_pp × coeficiente e o consumo adicional, sempre marcados como **simulação**.
- **FR-009**: O sistema MUST garantir **compatibilidade de período**: helpers de conversão anual↔mensal por juro composto ((1+π)^(1/12)−1), e MUST recusar/avisar mistura de períodos; nunca dividir anual por 12.
- **FR-010**: O sistema MUST suportar **dois modos de peso** — período atual e período-base (cesta fixa) — informando qual foi usado; default atual, base para histórico.
- **FR-011**: O sistema MUST calcular a **inflação pessoal por período** e a **acumulada por produto** ∏(1+π)−1 (nunca soma).
- **FR-012**: O sistema MUST tratar **dados ausentes** de inflação caindo na **categoria agregadora** (mapeamento categoria→grupo já existente) e **registrando a proveniência** na saída; ausência sem agregador → erro (nunca zero silencioso).
- **FR-013**: O sistema MUST **validar**: gasto ≥ 0, gastoTotal > 0, toda categoria com inflação, sem duplicata silenciosa, tolerância de ponto flutuante; inflação pode ser 0/negativa; coeficiente opcional.
- **FR-014**: O sistema MUST retornar um **DTO rico**: gastoTotal, inflaçãoPessoal (decimal e %), oficial, diferençaPP, custoAtualizado, aumentoCesta, rendaCorrigida, aumentoRenda, impactoComportamental, consumoAdicional, contribuições[] e as proveniências usadas.
- **FR-015**: O sistema MUST gerar uma **explicação textual para leigo** e incluir um **aviso metodológico** obrigatório (estimativa; coeficiente de outro contexto; não é previsão nem recomendação).
- **FR-016**: O cálculo MUST usar **aritmética decimal exata para dinheiro** (taxas podem ser ponto flutuante) e ser **determinístico**.
- **FR-017**: A feature MUST **reaproveitar as fontes já existentes** (índices IBGE/cache e mapeamento categoria→grupo) sem duplicar a coleta; e a UI existente (card/explicador) MUST passar a exibir contribuições e comparações.
- **FR-018**: Todo o processamento MUST ser **local**, sem novas chamadas de rede.

### Key Entities *(include if feature involves data)*

- **Entrada por categoria**: nome, gasto (dinheiro), inflação (taxa do período).
- **Parâmetros globais**: renda, inflação oficial, coeficiente comportamental (opcional), modo de peso, periodicidade.
- **Contribuição**: categoria, peso, inflação, contribuição.
- **Proveniência**: categoria solicitada, categoria/grupo efetivamente usado, fonte/período.
- **Resultado**: os campos do DTO rico (FR-014) + explicação textual + aviso.
- **Ponto histórico**: período, inflação pessoal; e a acumulada do intervalo.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Para o exemplo de referência, o sistema produz **exatamente**: inflação pessoal 7,7%, diferença 1,7 p.p., custo da cesta R$5.385 (+R$385), renda corrigida R$7.539 (+R$539), impacto comportamental 10,78% e consumo adicional R$539.
- **SC-002**: Em 100% dos casos, a soma das contribuições é igual à inflação pessoal (dentro da tolerância de ponto flutuante).
- **SC-003**: Nenhuma categoria sem inflação é silenciosamente tratada como zero — sempre erro ou substituição **com proveniência visível**.
- **SC-004**: Conversões de período usam juro composto (anual 6% → mensal ≈ 0,4868%) e o acumulado histórico usa produto, verificável por teste.
- **SC-005**: O resultado inclui explicação textual e o aviso metodológico em toda saída.
- **SC-006**: O cálculo é determinístico: mesma entrada → mesma saída.

## Assumptions

- Coeficiente comportamental default **1,4** (do artigo já citado no README); configurável e apenas simulação.
- Modo de peso default = **período atual**; período-base para análises históricas.
- Dinheiro em Decimal; taxas em ponto flutuante (conversões compostas exigem potência fracionária).
- Fonte de índices e mapeamento categoria→grupo reutilizam a infraestrutura existente (006); sem rede nova.
- O módulo de cálculo é **puro** (sem I/O), consumido pela camada de aplicação/comando e pela UI existente.
- "Renda corrigida" pressupõe toda a renda sujeita à cesta; a variante conservadora aplica só ao consumo (ambas retornadas).
