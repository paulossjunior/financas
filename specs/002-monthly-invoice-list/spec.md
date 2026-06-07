# Feature Specification: Listagem Mensal de Faturas

**Feature Branch**: `002-monthly-invoice-list`

**Created**: 2026-06-07

**Status**: Draft

**Input**: User description: "quero separar as contas por mes e listar todas."

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Ver todas as faturas agrupadas por mês (Priority: P1)

O usuário importou múltiplas faturas BTG de meses diferentes e quer ver todas elas organizadas por mês, com o total gasto em cada mês visível de forma clara. Hoje, só é possível ver o dashboard agregado; não há forma de navegar entre meses individualmente.

**Why this priority**: É o núcleo da funcionalidade. Sem a listagem por mês, o usuário não consegue separar e entender os gastos mês a mês.

**Independent Test**: Importar 3 faturas de meses diferentes → página de listagem mostra 3 grupos, um por mês, cada um com total correto.

**Acceptance Scenarios**:

1. **Given** nenhuma fatura importada, **When** usuário acessa a listagem, **Then** mensagem "Nenhuma fatura importada" é exibida.
2. **Given** 2 faturas do mesmo mês, **When** usuário acessa a listagem, **Then** as 2 faturas aparecem no mesmo grupo mensal com total somado das duas.
3. **Given** faturas de meses diferentes, **When** usuário acessa a listagem, **Then** grupos aparecem em ordem cronológica decrescente (mais recente primeiro).
4. **Given** listagem visível, **When** usuário visualiza um grupo mensal, **Then** vê: mês/ano, número de faturas, total líquido do período.

---

### User Story 2 — Ver detalhes de um mês específico (Priority: P2)

O usuário quer clicar em um mês na listagem e ver o dashboard filtrado apenas para aquele mês — categorias, transações e totais referentes somente ao período selecionado.

**Why this priority**: Complementa a listagem. O usuário precisa não só ver os totais mas conseguir "entrar" em um mês para análise detalhada.

**Independent Test**: Clicar em um mês na listagem → dashboard exibe apenas dados daquele mês (categorias e transações daquele período, total corresponde ao mês).

**Acceptance Scenarios**:

1. **Given** listagem com 3 meses, **When** usuário clica em "Maio/2026", **Then** é redirecionado ao dashboard com dados filtrados para Maio/2026.
2. **Given** dashboard filtrado por mês, **When** usuário remove o filtro, **Then** dashboard volta a exibir todos os dados agregados.
3. **Given** dashboard filtrado por mês, **When** usuário visualiza o banner de maior gasto, **Then** o banner reflete o maior gasto daquele mês especificamente.

---

### User Story 3 — Remover uma fatura da listagem (Priority: P3)

O usuário importou uma fatura errada ou duplicada e quer removê-la da listagem sem precisar reiniciar o aplicativo.

**Why this priority**: Operação de manutenção — necessária mas não bloqueia o valor principal da listagem.

**Independent Test**: Clicar em "remover" em uma fatura → fatura some da listagem e totais do mês são recalculados. Se era a única fatura do mês, o grupo mensal desaparece.

**Acceptance Scenarios**:

1. **Given** um grupo mensal com 2 faturas, **When** usuário remove uma, **Then** grupo continua existindo com total recalculado (apenas 1 fatura).
2. **Given** um grupo mensal com 1 fatura, **When** usuário remove essa fatura, **Then** o grupo mensal desaparece da listagem.
3. **Given** remoção solicitada, **When** confirmação do usuário aceita, **Then** ação é irreversível — fatura removida permanentemente.

---

### Edge Cases

- O que acontece se duas faturas do mesmo mês tiverem nomes de arquivo idênticos? → A segunda substitui a primeira (comportamento de deduplicação já existente).
- Como exibir faturas com mês desconhecido (nome de arquivo não segue padrão `YYYY-MM-*`)? → Agrupadas em "Mês desconhecido" no final da lista.
- O que acontece se o usuário filtra por um mês e depois remove a única fatura daquele mês? → Redirecionado automaticamente à listagem geral com todos os dados.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema DEVE listar todas as faturas importadas agrupadas por mês/ano.
- **FR-002**: Os grupos mensais DEVEM ser ordenados cronologicamente decrescente (mês mais recente primeiro).
- **FR-003**: Cada grupo mensal DEVE exibir: mês e ano por extenso (ex: "Maio 2026"), quantidade de faturas importadas naquele mês, e total líquido do período.
- **FR-004**: O usuário DEVE conseguir clicar em um mês para filtrar o dashboard e ver apenas os dados daquele período.
- **FR-005**: O usuário DEVE conseguir remover uma fatura individualmente da listagem.
- **FR-006**: Após remoção de uma fatura, os totais do grupo mensal DEVEM ser recalculados automaticamente.
- **FR-007**: Se um grupo mensal ficar sem faturas após remoção, o grupo DEVE desaparecer da listagem.
- **FR-008**: O usuário DEVE conseguir limpar o filtro de mês e voltar ao dashboard com todos os dados.
- **FR-009**: Faturas cujo mês não pôde ser inferido do nome do arquivo DEVEM ser agrupadas em categoria "Mês desconhecido".

### Key Entities

- **GrupoMensal**: Agrupamento de uma ou mais faturas pertencentes ao mesmo mês/ano. Atributos: mês/ano, lista de faturas, total líquido agregado, número de transações total.
- **Fatura (Invoice)**: Entidade já existente. Atributos relevantes: id, nome do arquivo, mês de referência, total de transações, data de importação.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Usuário visualiza todas as faturas agrupadas por mês em menos de 2 segundos após abrir a listagem.
- **SC-002**: Ao clicar em um mês, o dashboard filtrado carrega em menos de 1 segundo.
- **SC-003**: 100% das faturas importadas aparecem na listagem — nenhuma fatura é omitida, independente do nome do arquivo.
- **SC-004**: Após remover uma fatura, a listagem e os totais são atualizados em menos de 1 segundo, sem necessidade de recarregar o aplicativo.
- **SC-005**: O usuário consegue identificar rapidamente qual mês teve maior gasto apenas olhando a listagem (total visível por grupo).

## Assumptions

- Cada fatura corresponde a um único mês (não existem faturas que abranjam múltiplos meses).
- O mês de referência de uma fatura é inferido do nome do arquivo (padrão `YYYY-MM-*`) — comportamento já implementado.
- A listagem mensal substitui ou complementa a aba "Histórico" já existente no aplicativo.
- Não há paginação para a listagem de faturas — número de faturas por usuário pessoal é tipicamente ≤ 24 (2 anos de faturas mensais).
- A remoção de uma fatura requer confirmação do usuário para evitar exclusão acidental.
- Múltiplas faturas do mesmo mês são suportadas (ex: titular + dependente na mesma conta).
