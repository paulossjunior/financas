# Feature Specification: Importar extrato bancário do BTG

**Feature Branch**: `008-bank-statement-import`

**Created**: 2026-07-21

**Status**: Draft

**Input**: User description: "Ler o extrato bancário do BTG (.xls) e inserir dados de crédito e débito categorizados; usar os extratos das contas como entrada de dados. Excluir automaticamente o que já é contado (fatura do cartão, salário quando há contracheque, transferências internas). Categorizar pelas regras do app com fallback na categoria do BTG."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Importar o extrato e ver crédito/débito categorizados (Priority: P1)

Como usuário, quero importar o extrato do BTG (.xls) e ter meus lançamentos reais (pix, boletos, recebimentos) inseridos e categorizados automaticamente, para não precisar digitar cada gasto.

**Why this priority**: É o valor central — o extrato vira fonte de dados de gasto/renda com pouco esforço.

**Independent Test**: Selecionar o arquivo .xls; o app lista os lançamentos importados (data, descrição, valor, categoria, crédito/débito) e os soma no painel.

**Acceptance Scenarios**:

1. **Given** um extrato .xls do BTG, **When** importo, **Then** cada lançamento válido entra com data, descrição, valor, tipo (crédito/débito) e categoria.
2. **Given** o extrato tem linhas "Saldo Diário" e cabeçalho, **When** importo, **Then** essas linhas são ignoradas.
3. **Given** importo o mesmo extrato de novo, **When** confirmo, **Then** não duplica (lançamentos idênticos são reconhecidos).

### User Story 2 - Não duplicar o que o app já conta (Priority: P1)

Como usuário, quero que o app ignore automaticamente o que já é contado por outras fontes, para meus totais não inflarem.

**Acceptance Scenarios**:

1. **Given** um "Pagamento de fatura do cartão", **When** importo, **Then** ele é excluído (o cartão já vem da fatura).
2. **Given** um crédito de salário e existe contracheque no mês, **When** importo, **Then** o salário do extrato é excluído (a renda já vem do contracheque).
3. **Given** uma transferência entre minhas próprias contas (descrição = meu nome), **When** importo, **Then** ela é excluída (não é gasto nem receita real).

### User Story 3 - Revisar antes de salvar (Priority: P2)

Como usuário, quero revisar o que será importado e o que foi excluído (e por quê) antes de confirmar.

**Acceptance Scenarios**:

1. **Given** o extrato lido, **When** vejo a prévia, **Then** vejo os incluídos (com categoria) e os excluídos (com o motivo: fatura, salário, transferência interna) e posso confirmar.

### Edge Cases

- **Valor**: positivo = crédito (entra como renda extra); negativo = débito (entra como despesa avulsa).
- **Estornos** (valor que reverte outro): mantidos como lançamento (o líquido resolve), salvo se for transferência interna.
- **Descrição = nome do titular** (do cabeçalho do extrato): tratada como transferência interna → excluída.
- **Categoria sem regra do app**: usa a categoria informada pelo BTG.
- **Arquivo inválido / não-BTG**: erro claro, nada é importado.
- **Vários meses no mesmo arquivo**: cada lançamento entra no seu mês.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST ler o extrato .xls do BTG e extrair os lançamentos (data/hora, categoria BTG, tipo, descrição, valor), ignorando cabeçalho, metadados e linhas "Saldo Diário".
- **FR-002**: O sistema MUST classificar cada lançamento como **crédito** (valor > 0) ou **débito** (valor < 0).
- **FR-003**: O sistema MUST **excluir automaticamente**: pagamento de fatura do cartão, salário (quando houver contracheque no mês) e transferências entre as contas do próprio titular (descrição = nome do titular do extrato).
- **FR-004**: O sistema MUST categorizar cada lançamento incluído pelas **regras do app** (descrição); sem correspondência, usar a **categoria do BTG**.
- **FR-005**: O sistema MUST **salvar os lançamentos localmente** e **deduplicar** reimportações (mesmo lançamento não entra duas vezes).
- **FR-006**: O sistema MUST somar os lançamentos importados nos totais do painel — débitos como despesa, créditos como renda extra — por mês.
- **FR-007**: O sistema MUST oferecer uma **prévia** com incluídos (categoria) e excluídos (motivo) antes de confirmar.
- **FR-008**: Usuários MUST conseguir **remover** os lançamentos importados de um extrato.
- **FR-009**: O sistema MUST tratar arquivo inválido com mensagem clara, sem importar nada.
- **FR-010**: Tudo local; nenhum dado enviado.

### Key Entities *(include if feature involves data)*

- **Lançamento de extrato**: data, descrição, valor, tipo (crédito/débito), categoria (app ou BTG), categoria BTG original, mês, conta/origem, marca de exclusão + motivo.
- **Extrato**: arquivo importado (titular, conta, período) que agrupa lançamentos.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: O usuário importa um extrato e vê seus gastos/recebimentos reais categorizados sem digitar lançamento a lançamento.
- **SC-002**: Nenhum item já contado (fatura do cartão, salário com contracheque, transferência interna) infla os totais.
- **SC-003**: Reimportar o mesmo extrato não cria duplicatas.
- **SC-004**: A prévia mostra claramente o que entra e o que foi excluído (e por quê).
- **SC-005**: Tudo funciona offline/local.

## Assumptions

- Formato do extrato: planilha do BTG (aba "Extrato") com colunas Data e hora, Categoria, Transação, Descrição, Valor; metadados no topo incluem o nome do titular.
- "Transferência interna" é detectada quando a descrição corresponde ao nome do titular do extrato.
- Débitos entram como despesas avulsas; créditos como renda extra (não-recorrentes), por mês.
- Dedup por identidade do lançamento (data+descrição+valor+conta).
- Integra com o modelo atual (cartão + contracheque + manuais) sem dupla contagem.
