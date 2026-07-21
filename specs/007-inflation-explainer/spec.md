# Feature Specification: Explicador do impacto da inflação (para leigo)

**Feature Branch**: `007-inflation-explainer`

**Created**: 2026-07-21

**Status**: Draft

**Input**: User description: "Explicador do impacto da inflação para leigo… traduz IPCA e inflação pessoal em frases simples e números concretos, baseados nos gastos e recebíveis — no mês e nos anos… reaproveita índices em cache + totais de gasto/renda; local/offline."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Entender em uma frase o que a inflação faz comigo (Priority: P1)

Como usuário leigo, quero ler frases simples que traduzam a inflação para o meu bolso — quanto meus gastos vão custar mais e quanto minha renda perde de poder de compra — sem jargão.

**Why this priority**: É o coração da feature: transformar % abstrato em reais concretos que a pessoa entende.

**Independent Test**: Com índices atualizados e gastos/renda no app, o painel mostra frases-resumo com valores em reais (ex.: "seus gastos custarão R$ X a mais em 12 meses").

**Acceptance Scenarios**:

1. **Given** há índice em cache e gastos/renda, **When** abro o explicador, **Then** vejo, para cada indicador, uma frase clara + o número em reais.
2. **Given** não há índice em cache, **When** abro, **Then** vejo um convite para atualizar os índices (sem erro).

### User Story 2 - Projeção dos gastos no futuro (Priority: P1)

Como usuário, quero ver quanto meus gastos de hoje custarão em 12 meses, 3 e 5 anos se a inflação continuar, e quanto a mais em reais.

**Acceptance Scenarios**:

1. **Given** gasto mensal G e inflação anual observada, **When** vejo a projeção, **Then** aparece o custo futuro estimado (12m/3a/5a) e a diferença em reais, marcado como estimativa "se continuar assim".

### User Story 3 - Poder de compra da renda (Priority: P2)

Como usuário, quero saber quanto minha renda perde de poder de compra em 12 meses se não for reajustada, e quanto ela precisaria subir para empatar com a inflação.

**Acceptance Scenarios**:

1. **Given** renda mensal R e inflação anual, **When** vejo o card, **Then** aparece a perda em reais (R × inflação) e o reajuste necessário (%).

### User Story 4 - Minha inflação vs oficial e o que R$ 100 valem (Priority: P2)

Como usuário, quero ver se minha cesta subiu mais ou menos que o IPCA (e qual categoria puxa) e o que R$ 100 de hoje valerão no futuro / valeriam no passado.

**Acceptance Scenarios**:

1. **Given** inflação pessoal e IPCA, **When** vejo o card, **Then** a frase diz se subi mais/menos que a média (diferença em p.p.) e a categoria de maior peso.
2. **Given** a inflação anual, **When** vejo "valor do dinheiro", **Then** aparece o poder de compra de R$ 100 em 12 meses e em 5 anos.

### Edge Cases

- **Sem índice em cache**: convite a atualizar; sem números.
- **Sem gastos / sem renda**: mostrar só os cards possíveis (ex.: valor do dinheiro), ocultar os que dependem do dado faltante.
- **Inflação negativa (deflação)**: frases se invertem corretamente ("ganho de poder de compra").
- **Projeção**: sempre rotulada como estimativa "se a inflação continuar neste ritmo" — não é previsão oficial.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST apresentar, para cada indicador, uma **frase-resumo em linguagem simples** + o valor em reais/percentual.
- **FR-002**: O sistema MUST projetar o custo dos gastos atuais em **12 meses, 3 anos e 5 anos** usando a inflação observada, mostrando o custo futuro e a diferença em reais.
- **FR-003**: O sistema MUST mostrar a **perda de poder de compra da renda** em 12 meses (renda × inflação) e o reajuste necessário para empatar.
- **FR-004**: O sistema MUST comparar a **inflação pessoal com o IPCA** (diferença em p.p.) e indicar a **categoria de maior peso** na diferença.
- **FR-005**: O sistema MUST mostrar o **poder de compra de R$ 100** em horizontes futuros (e opcionalmente passado).
- **FR-006**: O sistema MUST **rotular projeções como estimativas** ("se a inflação continuar assim"), deixando claro que não é previsão oficial.
- **FR-007**: O sistema MUST reutilizar os **índices em cache** (IPCA + inflação pessoal) e os **totais de gasto/renda** já calculados; funcionar **offline**.
- **FR-008**: O sistema MUST ocultar graciosamente cards sem dado suficiente (sem índice, sem gasto ou sem renda).

### Key Entities *(include if feature involves data)*

- **Cenário de inflação**: taxa anual usada (IPCA 12 meses e/ou inflação pessoal anualizada).
- **Projeção de gasto**: custo futuro estimado do gasto mensal em cada horizonte + diferença.
- **Poder de compra**: efeito da inflação sobre renda e sobre R$ 100.
- **Frase-resumo**: texto simples derivado dos números para cada indicador.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Um usuário leigo entende, em uma frase, o impacto de cada indicador — sem termos técnicos.
- **SC-002**: Todo número aparece em **reais ou %**, nunca só índice abstrato.
- **SC-003**: As projeções deixam explícito que são estimativas "se continuar assim".
- **SC-004**: Funciona **offline** com o último índice salvo; sem índice, orienta a atualizar.
- **SC-005**: Os cálculos são consistentes (a projeção de 12 meses = gasto × inflação anual observada).

## Assumptions

- A "inflação anual observada" padrão é o **IPCA 12 meses**; a versão pessoal usa a **inflação pessoal anualizada** (mensal composta por 12), ambas rotuladas.
- Gasto e renda mensais vêm dos totais que o app já calcula (mês selecionado / média do período).
- Projeções são estimativas de juros compostos sobre a inflação observada — não previsão oficial.
- Exibição no dashboard (mês e/ou ano) — definição no planejamento.
- Local/offline; nenhum dado enviado.
