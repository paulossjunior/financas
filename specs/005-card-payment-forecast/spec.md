# Feature Specification: Previsão de pagamento do cartão (parcelamentos)

**Feature Branch**: `005-card-payment-forecast`

**Created**: 2026-07-21

**Status**: Draft

**Input**: User description: "Gráfico de previsão de pagamento do cartão baseado nos parcelamentos. O usuário quer ver, num gráfico, quanto ainda vai pagar de cartão nos próximos meses por causa das compras parceladas já feitas."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Ver quanto ainda vou pagar de cartão (Priority: P1)

Como usuário, quero ver num gráfico quanto do meu cartão já está comprometido em cada um dos próximos meses por causa das compras que parcelei. Assim eu sei o "peso" que já carrego antes mesmo de fazer qualquer compra nova.

**Why this priority**: É o coração da feature. Sem isso, o usuário só descobre o tamanho do parcelamento quando a fatura chega. Ver a projeção mês a mês permite planejar e evitar sufoco.

**Independent Test**: Com faturas que contêm compras parceladas importadas, abrir a visão de previsão e conferir que cada mês futuro mostra a soma das parcelas que ainda vão cair. Entrega valor sozinho, sem depender das outras histórias.

**Acceptance Scenarios**:

1. **Given** existem compras parceladas em aberto (ex.: uma compra em 3x com 1 parcela já paga), **When** o usuário abre a previsão, **Then** o gráfico mostra uma barra por mês futuro com a soma das parcelas que caem naquele mês.
2. **Given** não há nenhum parcelamento em aberto, **When** o usuário abre a previsão, **Then** o gráfico mostra estado vazio com uma mensagem clara ("sem parcelas futuras").
3. **Given** dois parcelamentos diferentes têm parcelas no mesmo mês, **When** o usuário vê aquele mês, **Then** o valor exibido é a soma das duas parcelas.

### User Story 2 - Entender a composição de um mês (Priority: P2)

Como usuário, quero ver quais compras compõem o compromisso de um mês específico, para saber de onde vem o valor.

**Why this priority**: Transforma o número em ação — o usuário identifica a compra pesada e decide.

**Independent Test**: Selecionar/passar o mouse em um mês da previsão e ver a lista de parcelamentos que caem nele (descrição, qual parcela, valor).

**Acceptance Scenarios**:

1. **Given** um mês futuro com mais de um parcelamento, **When** o usuário interage com aquele mês, **Then** vê a lista das parcelas daquele mês (descrição, "parcela X de Y", valor).

### User Story 3 - Saber o total comprometido e quando zera (Priority: P3)

Como usuário, quero ver o total que ainda vou pagar em parcelamentos e em que mês esse compromisso termina.

**Why this priority**: Dá o horizonte — "faltam R$ X e acaba em novembro" — útil para decisões de médio prazo.

**Acceptance Scenarios**:

1. **Given** parcelamentos em aberto, **When** o usuário abre a previsão, **Then** vê o total ainda a pagar (soma de todas as parcelas futuras) e o mês da última parcela.

### Edge Cases

- **Sem parcelamentos**: estado vazio com mensagem, sem gráfico quebrado.
- **Última parcela (parcela atual = total)**: não gera meses futuros (já é a última a cair).
- **Parcelamento já quitado**: não aparece na projeção.
- **Compras à vista**: nunca entram na previsão (não são parcelamento).
- **Estorno de uma compra parcelada**: não deve inflar a projeção.
- **Parcelamentos muito longos** (ex.: 24x): o horizonte se estende até a última parcela sem travar a leitura do gráfico.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST projetar, para cada mês futuro, a soma das parcelas ainda não pagas dos parcelamentos já existentes.
- **FR-002**: O sistema MUST derivar as parcelas restantes de cada parcelamento a partir de "parcela atual de total" e do valor da parcela.
- **FR-003**: Usuários MUST conseguir ver a projeção como um gráfico com um ponto/barra por mês futuro e o valor de cada mês.
- **FR-004**: O sistema MUST considerar apenas compromissos já assumidos (parcelamentos existentes); não projeta compras novas nem estima gastos futuros à vista.
- **FR-005**: O sistema MUST apresentar o total ainda a pagar em parcelamentos e o mês em que o compromisso termina (última parcela).
- **FR-006**: Usuários MUST conseguir ver a composição de um mês (quais parcelamentos e valores caem nele).
- **FR-007**: O sistema MUST definir o horizonte da projeção do próximo mês até a última parcela pendente.
- **FR-008**: O sistema MUST tratar estados vazios (nenhum parcelamento) com mensagem clara, sem erro.
- **FR-009**: O sistema MUST manter todos os dados e cálculos locais, sem qualquer chamada de rede.

### Key Entities *(include if feature involves data)*

- **Parcelamento**: uma compra parcelada — descrição, número da parcela atual, total de parcelas, valor da parcela e o mês de referência em que a parcela atual foi cobrada.
- **Ponto de previsão (mês futuro)**: um mês adiante com o valor total das parcelas que caem nele e a lista das parcelas que o compõem.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A partir da tela, o usuário vê em até 1 interação quanto pagará de cartão em cada um dos próximos meses.
- **SC-002**: A soma dos valores projetados é igual à soma de todas as parcelas pendentes dos parcelamentos existentes (consistência de 100%).
- **SC-003**: O usuário identifica em até 5 segundos o mês de maior compromisso futuro.
- **SC-004**: O usuário descobre, sem cálculo manual, em que mês o compromisso de parcelamentos do cartão zera.

## Assumptions

- Reutiliza os dados de parcelamento que o app já extrai das faturas (parcela atual/total, valor por parcela, mês de referência).
- A projeção começa no mês **seguinte** ao mês de referência mais recente — a parcela atual já está na fatura corrente.
- O valor de cada parcela é considerado constante (o app não dispõe de juros/variações por parcela).
- Horizonte da projeção = até a última parcela pendente entre todos os parcelamentos.
- Exibição principal na tela **Ano** (visão temporal para frente); um resumo pode aparecer na tela **Mês**. Definição final na fase de planejamento.
- Tudo local, sem rede (coerente com o restante do app).
