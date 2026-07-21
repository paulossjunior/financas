# Feature Specification: Categorias Personalizadas de Despesas

**Feature Branch**: `003-custom-categories`

**Created**: 2026-06-08

**Status**: Draft

**Input**: User description: "eu como usuario quero criar categorias e associar as despesas."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Gerenciar Categorias (Priority: P1)

O usuário pode criar novas categorias de despesas com um nome personalizado (ex.: "Saúde", "Lazer", "Educação") e pode editar ou remover categorias existentes. As categorias aparecem no dashboard e nos relatórios substituindo as categorias padrão do sistema.

**Why this priority**: Sem categorias personalizadas o usuário fica preso às categorias fixas do sistema e não consegue organizar os gastos da forma que faz sentido para sua vida financeira.

**Independent Test**: Criar uma categoria "Saúde", editar seu nome para "Saúde & Bem-Estar" e deletá-la. O resultado é verificável na lista de categorias disponíveis, sem necessidade de nenhuma outra funcionalidade.

**Acceptance Scenarios**:

1. **Given** nenhuma categoria personalizada existe, **When** o usuário digita o nome "Saúde" e confirma, **Then** a categoria "Saúde" aparece na lista de categorias disponíveis.
2. **Given** a categoria "Saúde" existe, **When** o usuário renomeia para "Saúde & Bem-Estar", **Then** o nome atualizado aparece em toda a interface (lista, dashboard, histórico).
3. **Given** a categoria "Lazer" existe com regras associadas, **When** o usuário tenta deletar, **Then** o sistema exibe confirmação informando quantas regras serão removidas junto.
4. **Given** o usuário confirma a deleção, **When** a categoria é removida, **Then** as transações que eram dessa categoria passam a ser classificadas como "Outros".

---

### User Story 2 - Criar Regras de Categorização por Palavras-Chave (Priority: P2)

O usuário pode associar palavras-chave a uma categoria. Quando uma transação contém a palavra-chave na descrição, ela é automaticamente classificada nessa categoria. O usuário pode adicionar várias palavras-chave por categoria e remover regras existentes.

**Why this priority**: Criar a categoria sozinha não resolve o problema — o usuário precisa que as transações sejam automaticamente classificadas nela para que o dashboard reflita a nova organização.

**Independent Test**: Com a categoria "Saúde" criada, adicionar a palavra-chave "DROGARIA". Importar uma fatura que contenha a transação "DROGARIA SAO PAULO". A transação deve aparecer no dashboard sob "Saúde".

**Acceptance Scenarios**:

1. **Given** a categoria "Saúde" existe, **When** o usuário adiciona a palavra-chave "DROGARIA", **Then** a regra é salva e aparece na lista de regras de "Saúde".
2. **Given** a regra "DROGARIA → Saúde" existe, **When** uma fatura é importada com a transação "DROGARIA PACHECO SP", **Then** essa transação aparece na categoria "Saúde" no dashboard.
3. **Given** a regra existe, **When** o usuário remove a palavra-chave "DROGARIA" de "Saúde", **Then** transações com "DROGARIA" voltam a ser classificadas conforme as regras padrão (ou "Outros" se não houver outra regra).
4. **Given** uma palavra-chave conflita com outra regra existente, **When** o usuário tenta adicionar, **Then** o sistema exibe aviso indicando qual categoria já usa essa palavra-chave.

---

### User Story 3 - Reclassificação Manual de Transação Individual (Priority: P3)

O usuário pode alterar manualmente a categoria de uma transação específica, sobrepondo a classificação automática. Essa escolha manual persiste para futuras visualizações da mesma transação.

**Why this priority**: Existem casos pontuais em que nenhuma regra de palavra-chave consegue capturar corretamente — o usuário precisa de um escape hatch para corrigir casos específicos sem criar regras genéricas que causariam falsos positivos.

**Independent Test**: Localizar uma transação "AMAZON MKTPL" classificada como "Compras", alterar manualmente para "Educação" (ex.: compra de livro). A transação deve exibir "Educação" no dashboard, histórico e relatório.

**Acceptance Scenarios**:

1. **Given** a transação "AMAZON MKTPL" está classificada como "Compras", **When** o usuário seleciona "Educação" como categoria, **Then** a transação exibe "Educação" na interface.
2. **Given** a reclassificação manual foi feita, **When** o usuário reimporta a fatura ou filtra por mês, **Then** a reclassificação manual persiste.
3. **Given** a transação tem reclassificação manual, **When** o usuário remove o override, **Then** a classificação automática por palavras-chave é restaurada.

---

### Edge Cases

- O que acontece quando o usuário cria uma categoria com nome duplicado?
- Como o sistema trata palavras-chave com acentos (ex.: "Alimentação" vs "Alimentacao")?
- O que acontece com as categorias padrão existentes (Alimentação, Transporte, etc.) — elas coexistem ou são substituídas?
- O que acontece se o usuário deleta todas as categorias?
- Como ficam as transações já importadas quando uma nova regra é adicionada — elas são reclassificadas automaticamente?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema DEVE permitir ao usuário criar categorias com nome único (case-insensitive).
- **FR-002**: O sistema DEVE permitir ao usuário renomear e deletar categorias existentes.
- **FR-003**: Ao deletar uma categoria, o sistema DEVE exibir confirmação com o número de regras e transações afetadas.
- **FR-004**: O sistema DEVE permitir associar múltiplas palavras-chave a uma categoria (matching case-insensitive, substring).
- **FR-005**: O sistema DEVE aplicar as regras de palavras-chave na classificação de todas as transações de todas as faturas importadas.
- **FR-006**: O sistema DEVE detectar conflito de palavra-chave entre categorias e alertar o usuário antes de salvar.
- **FR-007**: O sistema DEVE permitir reclassificação manual de transações individuais, com persistência.
- **FR-008**: O sistema DEVE reprocessar automaticamente as categorias das transações existentes quando regras são adicionadas ou removidas.
- **FR-009**: As categorias e regras criadas pelo usuário DEVEM persistir entre sessões do aplicativo.
- **FR-010**: O dashboard e o histórico DEVEM refletir as categorias personalizadas do usuário.

### Key Entities

- **Categoria**: nome único, pode ser padrão (sistema) ou personalizada (usuário). Atributos: id, nome, origem (padrão/personalizada).
- **Regra de Categorização**: associação entre palavra-chave e categoria. Atributos: palavra-chave, categoria_id, prioridade.
- **Override de Transação**: reclassificação manual de uma transação específica. Atributos: transaction_id, categoria_id.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Criar uma nova categoria e adicionar a primeira regra de palavra-chave leva menos de 60 segundos do início ao fim.
- **SC-002**: 100% das transações importadas são classificadas em alguma categoria (nenhuma fica sem categoria).
- **SC-003**: A reclassificação automática por novas regras ocorre sem necessidade de reimportar faturas.
- **SC-004**: Todas as categorias criadas pelo usuário sobrevivem ao fechamento e reabertura do aplicativo.
- **SC-005**: O dashboard exibe corretamente os totais agrupados pelas novas categorias personalizadas.

## Assumptions

- O aplicativo é de uso individual — não há controle de acesso ou múltiplos usuários.
- As categorias padrão do sistema (Alimentação, Transporte, etc.) coexistem com as personalizadas; o usuário pode editar ou remover as padrão se quiser.
- O matching de palavras-chave é feito na descrição da transação (campo de texto da fatura).
- Quando uma transação não corresponde a nenhuma regra, ela permanece classificada como "Outros".
- A reclassificação manual tem prioridade sobre qualquer regra de palavras-chave.
- Novas regras disparam reprocessamento das categorias nas faturas já importadas (sem reimportar o arquivo).
- Gerenciamento de categorias e regras é feito na página de Configurações.
- Não há limite artificial no número de categorias ou regras que o usuário pode criar.
