# Feature Specification: Categorias recorrentes + baseline + anti-duplicação

**Feature Branch**: `010-recurring-categories`

**Created**: 2026-07-21

**Status**: Draft

**Input**: User description: "Categorias recorrentes + baseline + anti-duplicação. Marcar categorias como recorrentes (aluguel, água, luz, internet); contas fixas do mês derivadas automaticamente dos lançamentos já categorizados (extrato + fatura); categoria recorrente = fonte de verdade; manual = fallback/override; anti-duplicação (extrato supersede manual); baseline = média dos últimos 3 meses para o Teto/projeção antes de importar; detecção inteligente opt-in (sugere, não marca sozinho); recorrência finita com vigência (ex.: psicólogo por 3 meses)."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Contas fixas derivadas do dado real (Priority: P1)

Hoje o usuário digita cada conta fixa (aluguel, água, luz) na aba **Fixos & Renda**. Como agora o **extrato bancário** e a **fatura** já trazem esses lançamentos, digitar de novo é retrabalho e arrisca contar em dobro. O usuário marca as categorias que se repetem todo mês como **recorrentes**; a partir daí, as **contas fixas do mês são derivadas automaticamente** dos lançamentos importados nessas categorias — a categoria recorrente vira a **fonte de verdade**.

**Why this priority**: É o núcleo da feature — elimina digitação, elimina o duplo-cont e mantém as fixas sempre com o valor real. Sem isso, nada mais faz sentido.

**Independent Test**: Marcar "Aluguel" como recorrente, importar um extrato com um débito de aluguel no mês, e verificar que ele aparece nas **contas fixas** do mês com a origem "Extrato", sem precisar de cadastro manual, e que o total das fixas reflete o valor do extrato.

**Acceptance Scenarios**:

1. **Given** a categoria "Aluguel" marcada como recorrente e um extrato importado com um débito de R$ 2.000 categorizado como Aluguel, **When** o usuário abre o painel do mês, **Then** "Aluguel" aparece nas contas fixas com valor R$ 2.000 e origem "Extrato".
2. **Given** uma categoria **não** marcada como recorrente (ex.: "Restaurantes") com lançamentos no extrato, **When** o painel calcula as contas fixas, **Then** esses lançamentos **não** entram nas contas fixas (continuam como despesa variável/avulsa).
3. **Given** uma categoria recorrente com lançamentos vindos tanto do extrato quanto da fatura no mesmo mês, **When** as fixas são derivadas, **Then** cada lançamento distinto é contado uma vez, com a respectiva origem (Extrato/Fatura).

---

### User Story 2 - Anti-duplicação: extrato supersede o fixo manual (Priority: P1)

O usuário já tinha um fixo manual de "Aluguel R$ 2.000" cadastrado. Ao importar o extrato, o mesmo aluguel aparece. O sistema deve **usar o lançamento importado e suprimir o fixo manual equivalente** no mês — nunca contar os dois — do mesmo jeito que o contracheque já supersede o salário manual.

**Why this priority**: Sem isso, quem já usava fixos manuais passaria a contar tudo em dobro ao importar o extrato. É condição para a feature não regredir os dados de quem já usa o app.

**Independent Test**: Cadastrar um fixo manual de Aluguel; importar um extrato com o mesmo aluguel; verificar que o total de despesas conta o aluguel **uma vez só** e que a UI indica que o manual foi suprimido pelo importado.

**Acceptance Scenarios**:

1. **Given** um fixo manual "Aluguel R$ 2.000" e um débito de aluguel no extrato do mesmo mês na categoria recorrente Aluguel, **When** o painel é calculado, **Then** o aluguel entra uma única vez (valor do extrato) e o total de despesas não soma os dois.
2. **Given** o cenário acima, **When** o usuário olha a lista de contas fixas, **Then** vê uma indicação de que o fixo manual foi substituído pelo lançamento do extrato (com opção de ver detalhe).
3. **Given** um fixo manual em uma categoria recorrente **sem** lançamento importado correspondente naquele mês (ex.: seguro do carro em débito automático não importado), **When** o painel é calculado, **Then** o fixo manual é mantido (fallback) e contado normalmente.

---

### User Story 3 - Baseline para o Teto antes de importar (Priority: P2)

No começo do mês o extrato ainda não foi importado, mas o usuário quer saber o **Teto do cartão** (renda recorrente − contas fixas). O sistema usa a **média dos últimos 3 meses** das categorias recorrentes como **baseline** das fixas, e sinaliza que o valor é estimado. Quando o extrato/fatura do mês entra, o **valor real substitui a média** automaticamente.

**Why this priority**: Mantém o Teto e as projeções úteis mesmo sem dados do mês corrente — hoje as fixas manuais cumpriam esse papel. É importante, mas depende de US1/US2 já existirem.

**Independent Test**: Com 3 meses de histórico de recorrentes importados e o mês corrente sem import, verificar que o Teto do cartão usa a média dos 3 meses das categorias recorrentes e exibe um selo "base: média". Depois importar o mês e verificar que o valor real substitui a média.

**Acceptance Scenarios**:

1. **Given** 3 meses de histórico com Aluguel/Água/Luz recorrentes e o mês corrente sem lançamentos importados, **When** o Teto do cartão é calculado, **Then** ele usa a média dos 3 meses dessas categorias como contas fixas e a UI mostra um indicador de "baseline/estimado".
2. **Given** o mês corrente com o extrato já importado, **When** o Teto é calculado, **Then** ele usa o valor **realizado** do mês (não a média) e o selo de baseline some.
3. **Given** menos de 3 meses de histórico, **When** o baseline é calculado, **Then** usa a média dos meses disponíveis (≥1) e deixa claro que a base é parcial; sem histórico, o baseline é zero.

---

### User Story 4 - Detecção inteligente (sugestão, opt-in) (Priority: P3)

O sistema percebe que uma categoria/descrição aparece de forma **aproximadamente mensal com valor semelhante** (ex.: "Academia Smart Fit ~R$ 110 nos últimos 4 meses") e **sugere** marcá-la como recorrente. O usuário **confirma ou ignora** — o sistema **nunca marca sozinho**.

**Why this priority**: Conveniência que acelera a configuração, mas o usuário consegue marcar manualmente sem ela. Puro incremento.

**Independent Test**: Importar histórico com uma despesa que se repete ~mensal com valor parecido; verificar que aparece uma sugestão "marcar como recorrente" e que aceitar liga o flag, ignorar dispensa a sugestão sem alterar nada.

**Acceptance Scenarios**:

1. **Given** uma descrição/categoria que ocorre em ≥3 dos últimos 4 meses com variação de valor pequena, **When** o usuário abre o Mapeamento, **Then** vê uma sugestão para marcá-la como recorrente com o valor médio observado.
2. **Given** uma sugestão exibida, **When** o usuário clica "Ignorar", **Then** a sugestão some e não reaparece para o mesmo item, e nada é marcado.
3. **Given** uma sugestão exibida, **When** o usuário clica "Marcar", **Then** a categoria/regra passa a recorrente e as contas fixas passam a considerá-la.

---

### User Story 5 - Recorrência finita com vigência (Priority: P2)

Algumas despesas são fixas **por um período** e depois acabam (ex.: psicólogo por 3 meses, um curso). O usuário define uma **vigência** (mês início → mês fim, ou nº de meses) para essa recorrência. Dentro da vigência ela conta como fixa; **depois do fim, sai das contas fixas, do baseline e do Teto automaticamente**. A vigência é do **compromisso específico** (regra/lançamento, ex.: "psicólogo"), não da categoria inteira (Saúde continua existindo).

**Why this priority**: Sem vigência, uma fixa temporária ficaria projetada para sempre pelo baseline, distorcendo o Teto por meses. É necessário para a corretude do baseline, mas construído sobre US1–US3.

**Independent Test**: Definir "Psicólogo" como recorrente com vigência jan–mar; verificar que conta nas fixas de jan/fev/mar e some de abr em diante, inclusive não entrando no baseline de abr.

**Acceptance Scenarios**:

1. **Given** um compromisso recorrente "Psicólogo" com vigência jan–mar, **When** o painel de fevereiro é calculado, **Then** o psicólogo entra nas contas fixas.
2. **Given** o mesmo compromisso, **When** o painel de abril é calculado, **Then** o psicólogo **não** entra nas contas fixas nem no baseline/Teto.
3. **Given** um compromisso recorrente contínuo (sem vigência) que **deixou de aparecer** no mês mais recente importado, **When** o Mapeamento é aberto, **Then** o sistema **sugere** marcá-lo como encerrado (para removê-lo do baseline), sem removê-lo sozinho.

---

### Edge Cases

- **Valor variável (água/luz)**: o valor real do mês vem do extrato; o baseline usa a média — a UI diferencia "valor fixo" de "média/varia".
- **Múltiplos lançamentos na mesma categoria recorrente no mês** (ex.: dois débitos de energia): somam-se; a fixa da categoria no mês é a soma.
- **Categoria recorrente sem nenhum lançamento no mês nem histórico**: fixa = 0 naquele mês; não inventa valor.
- **Reimportar o mesmo extrato**: a dedup já existente evita duplicar lançamentos; as fixas derivadas não mudam.
- **Fixo manual em categoria recorrente com vários lançamentos importados**: o manual é suprimido inteiro quando há qualquer lançamento importado equivalente no mês/categoria; documentar o critério de equivalência (mesma categoria recorrente + mesmo mês).
- **Vigência com fim no passado**: nunca aparece nos meses após o fim, mesmo em recálculo histórico.
- **Estornos** dentro de uma categoria recorrente: reduzem o total da fixa daquele mês (aritmética exata).
- **Mudança de flag recorrente**: marcar/desmarcar recalcula fixas e baseline de forma determinística, sem duplicar.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST permitir marcar/desmarcar uma categoria como **recorrente** (despesa fixa mensal), persistindo o estado localmente.
- **FR-002**: O sistema MUST **derivar as contas fixas do mês** a partir dos lançamentos já categorizados (extrato + fatura) cujas categorias estão marcadas como recorrentes, sem exigir cadastro manual.
- **FR-003**: O sistema MUST tratar a categoria recorrente como **fonte de verdade**: o valor da fixa no mês é o realizado dos lançamentos importados daquela categoria naquele mês.
- **FR-004**: O sistema MUST manter o **cadastro manual de fixos** como fallback/override, usado quando não há lançamento importado equivalente no mês (débito automático, dinheiro) ou quando o mês ainda não foi importado.
- **FR-005**: O sistema MUST aplicar **anti-duplicação**: quando existe lançamento importado equivalente (mesma categoria recorrente, mesmo mês) a um fixo manual, o importado **supersede** o manual — o valor é contado uma única vez.
- **FR-006**: O sistema MUST indicar na UI a **origem** de cada conta fixa (Extrato, Fatura ou Manual) e sinalizar quando um fixo manual foi suprimido pelo importado.
- **FR-007**: O sistema MUST calcular um **baseline** por categoria recorrente = **média dos últimos 3 meses** (ou dos meses disponíveis, se menos de 3) dessa categoria.
- **FR-008**: O sistema MUST usar o **baseline** como valor das contas fixas de um mês **ainda não importado** (para o Teto do cartão e projeções) e MUST **substituir pelo valor realizado** assim que houver lançamentos importados daquele mês.
- **FR-009**: O sistema MUST sinalizar na UI quando o valor exibido (fixas/Teto) é **baseline (estimado)** em vez de realizado.
- **FR-010**: O sistema MUST oferecer **detecção inteligente opt-in**: identificar categorias/descrições que ocorrem de forma aproximadamente mensal com valor semelhante e **sugerir** marcá-las como recorrentes, exigindo confirmação do usuário (nunca marcar automaticamente).
- **FR-011**: O sistema MUST permitir **dispensar (ignorar)** uma sugestão de recorrência, e a mesma sugestão NÃO deve reaparecer para o mesmo item.
- **FR-012**: O sistema MUST suportar **recorrência finita com vigência** (mês início → mês fim, ou número de meses), aplicada a um compromisso específico (regra/lançamento), não à categoria inteira.
- **FR-013**: O sistema MUST **excluir automaticamente** uma recorrência finita das contas fixas, do baseline e do Teto **após o fim da vigência**, de forma determinística inclusive em recálculo histórico.
- **FR-014**: O sistema MUST, para recorrências contínuas que **deixaram de aparecer** no mês mais recente importado, **sugerir encerrá-las** (removê-las do baseline), sem removê-las automaticamente.
- **FR-015**: Todos os cálculos monetários (fixas, baseline, Teto) MUST usar aritmética decimal exata e ser **determinísticos** dado o mesmo conjunto de dados.
- **FR-016**: O sistema MUST tratar **estornos** dentro de categorias recorrentes reduzindo o total da fixa do respectivo mês.
- **FR-017**: O sistema MUST manter todo o processamento **local**, sem chamadas de rede para esta feature.
- **FR-018**: O sistema MUST recalcular fixas e baseline de forma **idempotente** ao marcar/desmarcar recorrência, reimportar dados ou reabrir o app (sem duplicação).

### Key Entities *(include if feature involves data)*

- **Categoria recorrente**: uma categoria marcada como despesa fixa mensal. Atributos: nome da categoria, flag recorrente, (opcional) vigência quando finita, baseline calculado (derivado).
- **Compromisso recorrente finito**: uma recorrência com prazo (ex.: psicólogo). Atributos: identificação (regra/descrição/categoria), mês início, mês fim (ou nº de meses), valor esperado/observado. Relaciona-se a uma categoria.
- **Conta fixa do mês (derivada)**: resultado do cálculo para um mês. Atributos: categoria, valor, origem (Extrato/Fatura/Manual/Baseline), status (realizado/estimado/suprimido).
- **Sugestão de recorrência**: item detectado como candidato a recorrente. Atributos: alvo (categoria/descrição), valor médio observado, meses observados, estado (pendente/aceita/ignorada).
- **Baseline**: por categoria recorrente, a média dos últimos N meses usada como estimativa. Derivado, não editável diretamente.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Depois de marcar suas categorias fixas como recorrentes e importar o extrato do mês, o usuário obtém as contas fixas **sem digitar nenhum lançamento manual** para essas categorias.
- **SC-002**: Em cenários com fixo manual + lançamento importado equivalente, o valor da despesa é contado **exatamente uma vez** em 100% dos casos (zero duplicação).
- **SC-003**: O Teto do cartão fica disponível em um mês ainda não importado usando o baseline, e passa a refletir o valor **realizado** automaticamente após a importação, sem ação manual.
- **SC-004**: Uma recorrência finita (ex.: 3 meses) deixa de contar nas fixas e no Teto a partir do primeiro mês após o fim, em 100% dos recálculos.
- **SC-005**: A configuração inicial de recorrências fica mais rápida com as sugestões: o usuário consegue marcar as fixas sugeridas com um clique cada, sem digitar valores.
- **SC-006**: Os totais de fixas, baseline e Teto são **determinísticos**: reabrir o app ou reimportar os mesmos dados produz os mesmos números.

## Assumptions

- **Baseline padrão = média dos últimos 3 meses** (decidido com o usuário; pode ser revisto).
- **Detecção é sempre opt-in**: o app sugere, o usuário confirma; nada é marcado automaticamente.
- **Fixos & Renda** passa a mostrar as fixas **derivadas** (somente leitura por categoria) com um botão para **adicionar fixo manual** (para o que não passa em conta).
- **Equivalência para anti-duplicação** = mesma categoria recorrente no mesmo mês (não casamento por valor exato), consistente com o mecanismo payslip→salário já existente.
- **Vigência** é mensal (granularidade de mês), coerente com o resto do app.
- Reaproveita a infraestrutura existente: categorias/regras (003), lançamentos manuais recorrentes, extrato (008), fatura, e o padrão de "supersede" do contracheque (004).
- Sem mudança no modelo de dinheiro (Decimal como string) nem no princípio local-first.
