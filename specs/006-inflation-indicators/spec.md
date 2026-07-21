# Feature Specification: Indicadores de inflação (IPCA + inflação pessoal)

**Feature Branch**: `006-inflation-indicators`

**Created**: 2026-07-21

**Status**: Draft

**Input**: User description: "Indicadores de inflação no dashboard, com inflação pessoal… IPCA oficial + inflação pessoal reponderada pelos gastos do usuário… dados da API do IBGE, busca opt-in (botão 'Atualizar índices'), guardando em cache local… salvar os dados do IBGE localmente."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Ver o IPCA oficial no dashboard (Priority: P1)

Como usuário, quero ver o IPCA (variação do mês, no ano e em 12 meses) no meu dashboard, para saber a inflação oficial sem sair do app.

**Why this priority**: É a base e a referência de comparação. Entrega valor sozinho.

**Independent Test**: Com índices atualizados, o dashboard mostra IPCA do mês, no ano e 12 meses, com o mês de referência.

**Acceptance Scenarios**:

1. **Given** os índices já foram atualizados, **When** abro o dashboard, **Then** vejo IPCA do mês, acumulado no ano e em 12 meses, com o mês de referência.
2. **Given** nunca atualizei os índices, **When** abro o dashboard, **Then** vejo um estado vazio convidando a clicar em "Atualizar índices".

### User Story 2 - Ver minha inflação pessoal (Priority: P1)

Como usuário, quero ver minha "inflação pessoal" — as variações dos grupos do IPCA reponderadas pelo peso real dos meus gastos — comparada ao IPCA oficial, para saber se minha cesta sobe mais ou menos que a média.

**Why this priority**: É o diferencial apoiado pela literatura (a inflação sentida depende da cesta de cada família).

**Independent Test**: Com gastos categorizados e índices atualizados, o app mostra um número de inflação pessoal e a diferença para o IPCA oficial.

**Acceptance Scenarios**:

1. **Given** tenho gastos categorizados e índices atualizados, **When** vejo o card de inflação, **Then** aparece minha inflação pessoal e a diferença (em p.p.) para o IPCA oficial.
2. **Given** uma categoria minha não tem grupo do IPCA correspondente, **When** a inflação pessoal é calculada, **Then** essa parcela usa o IPCA geral como referência (sem quebrar o cálculo).
3. **Given** não tenho gastos no período, **When** vejo o card, **Then** a inflação pessoal iguala o IPCA geral (sem pesos próprios).

### User Story 3 - Atualizar índices sob demanda (Priority: P2)

Como usuário, quero atualizar os índices quando eu quiser (o app é offline por padrão), clicando em um botão que busca os dados públicos do IBGE.

**Why this priority**: Preserva o princípio local-first; a rede só é usada quando eu autorizo.

**Acceptance Scenarios**:

1. **Given** estou online, **When** clico em "Atualizar índices", **Then** o app busca o IPCA (geral e por grupo) e mostra os valores mais recentes com a data da atualização.
2. **Given** estou offline (ou o IBGE falha), **When** clico em "Atualizar", **Then** vejo uma mensagem de erro clara e o último cache é mantido.

### User Story 4 - Guardar os índices localmente (Priority: P1)

Como usuário, quero que os índices baixados fiquem salvos no meu computador, para continuar vendo a inflação mesmo offline e sem precisar rebuscar toda vez.

**Why this priority**: Coerente com o app local-first; sem isso, cada abertura exigiria internet.

**Acceptance Scenarios**:

1. **Given** atualizei os índices uma vez, **When** fecho e reabro o app offline, **Then** os últimos índices continuam visíveis, com a data em que foram baixados.
2. **Given** existe cache salvo, **When** abro o dashboard, **Then** os indicadores aparecem imediatamente, sem rede.

### Edge Cases

- **Sem cache** (nunca atualizou): estado vazio com convite a atualizar; sem erro.
- **Offline / IBGE indisponível** no clique: erro claro; cache anterior preservado.
- **Categoria sem grupo IPCA**: usa o IPCA geral para essa fração.
- **Cache defasado** (mês antigo): mostrar a data/mês de referência para o usuário perceber que está velho.
- **Sem gastos categorizados**: inflação pessoal = IPCA geral.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST exibir o IPCA oficial: variação do mês, acumulado no ano e acumulado em 12 meses, com o mês de referência.
- **FR-002**: O sistema MUST calcular a "inflação pessoal" reponderando as variações dos grupos do IPCA pelos pesos reais de gasto do usuário (participação de cada categoria), e mostrar a diferença para o IPCA oficial.
- **FR-003**: O sistema MUST mapear as categorias do usuário aos grupos do IPCA; categorias sem grupo correspondente usam o IPCA geral.
- **FR-004**: O sistema MUST buscar os índices apenas sob demanda (ação explícita "Atualizar índices"), nunca automaticamente.
- **FR-005**: O sistema MUST **salvar os índices localmente** e usá-los offline nas aberturas seguintes, sem nova busca.
- **FR-006**: O sistema MUST exibir a data/mês da última atualização dos índices.
- **FR-007**: O sistema MUST tratar falha de rede/fonte com mensagem clara, preservando o cache anterior.
- **FR-008**: O sistema MUST funcionar sem índices (estado vazio) e sem gastos (inflação pessoal = IPCA geral), sem erro.
- **FR-009**: O sistema MUST enviar nenhum dado pessoal na atualização (somente leitura de índices públicos).

### Key Entities *(include if feature involves data)*

- **Índice IPCA (headline)**: mês de referência, variação do mês, acumulado no ano, acumulado em 12 meses, data da busca.
- **Grupo do IPCA**: nome do grupo (Alimentação, Habitação, Transportes, Saúde…) e sua variação no mês.
- **Cache de índices**: conjunto persistido localmente (headline + grupos + data), disponível offline.
- **Inflação pessoal**: valor calculado a partir dos pesos de gasto do usuário × variações dos grupos, mais a diferença para o IPCA oficial.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: O usuário vê sua inflação pessoal e a compara com o IPCA oficial em uma única tela, sem cálculo manual.
- **SC-002**: Após atualizar uma vez, os indicadores continuam visíveis **offline** em 100% das aberturas seguintes.
- **SC-003**: O usuário sempre sabe de quando é o índice exibido (mês de referência / data da busca).
- **SC-004**: A soma dos pesos usados na inflação pessoal corresponde a 100% dos gastos considerados (cálculo consistente).
- **SC-005**: Uma atualização de índices conclui em até 5 segundos em conexão comum, ou falha com mensagem clara.

## Assumptions

- Fonte dos índices: dados públicos do IBGE (IPCA geral e por grupo). O primeiro carregamento exige internet; depois funciona offline via cache.
- O peso de cada categoria vem dos gastos já categorizados no app (participação no total).
- Mapeamento categoria → grupo do IPCA definido no app; categorias sem correspondência caem no IPCA geral.
- Exibição no dashboard (tela Ano e/ou Mês) — definição final no planejamento.
- Local-first: nenhuma busca automática; nenhum dado pessoal transmitido.
