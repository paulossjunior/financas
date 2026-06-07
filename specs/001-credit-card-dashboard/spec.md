# Feature Specification: Gestor Financeiro — Dashboard de Faturas BTG

**Feature Branch**: `001-credit-card-dashboard`

**Created**: 2026-06-07

**Status**: Draft

**Input**: User description: "quero criar um gestor financeiro que leia as @faturas/ do meu cartão e crie um dashboard categorizando as despesas e mostre aonde está o maior gasto"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Importar Fatura BTG (Priority: P1)

O usuário coloca um arquivo de fatura do cartão BTG (formato XLSX) na pasta `faturas/` e o sistema lê automaticamente, extraindo todas as transações com valor, data, descrição e categoria.

**Why this priority**: Sem importação de dados não há nada para visualizar. É o bloco fundamental de todo o sistema.

**Independent Test**: Basta adicionar um arquivo XLSX do BTG na pasta `faturas/` e verificar se o sistema lista as transações corretamente, com valores e datas.

**Acceptance Scenarios**:

1. **Given** um arquivo XLSX de fatura BTG válido na pasta `faturas/`, **When** o sistema processa o arquivo, **Then** todas as transações são extraídas com data, descrição, valor e categoria identificados.
2. **Given** múltiplos arquivos XLSX na pasta `faturas/`, **When** o sistema processa os arquivos, **Then** todas as faturas são consolidadas sem duplicatas.
3. **Given** um arquivo XLSX com formato inesperado ou corrompido, **When** o sistema tenta processar, **Then** exibe mensagem clara de erro e continua processando os demais arquivos.

---

### User Story 2 - Dashboard de Categorias de Gastos (Priority: P2)

O usuário visualiza um dashboard com os gastos agrupados por categoria (ex: Alimentação, Transporte, Saúde, Lazer, etc.), mostrando o total gasto em cada categoria e o percentual do total.

**Why this priority**: Categorização é o núcleo do valor entregue — transforma dados brutos em inteligência financeira acionável.

**Independent Test**: Com transações importadas, o dashboard exibe gráfico ou tabela de categorias com valores totais e percentuais verificáveis manualmente.

**Acceptance Scenarios**:

1. **Given** transações importadas, **When** o usuário acessa o dashboard, **Then** vê lista de categorias com total gasto e percentual de cada uma em relação ao total da fatura.
2. **Given** transações sem categoria definida na fatura, **When** o sistema processa, **Then** agrupa automaticamente em "Outros" ou infere categoria pela descrição da transação.
3. **Given** múltiplas faturas importadas, **When** o usuário visualiza o dashboard, **Then** pode filtrar por período ou fatura específica.

---

### User Story 3 - Identificar Maior Gasto (Priority: P2)

O usuário vê de forma destacada onde está seu maior gasto — seja a categoria com mais despesas, seja a transação individual de maior valor.

**Why this priority**: Responde diretamente à necessidade do usuário: "onde está o maior gasto". Alto valor com baixo esforço adicional.

**Independent Test**: O dashboard exibe claramente o destaque do maior gasto, verificável contra os dados brutos da fatura.

**Acceptance Scenarios**:

1. **Given** transações importadas, **When** o usuário visualiza o dashboard, **Then** a categoria com maior gasto total aparece destacada visualmente com valor e percentual.
2. **Given** transações importadas, **When** o usuário acessa detalhes, **Then** vê as 5 maiores transações individuais em ordem decrescente de valor.
3. **Given** variação entre meses, **When** há múltiplas faturas, **Then** o sistema indica qual categoria cresceu mais em relação ao período anterior.

---

### User Story 4 - Visão de Evolução Mensal (Priority: P3)

O usuário com múltiplas faturas carregadas consegue ver a evolução dos gastos mês a mês por categoria.

**Why this priority**: Agrega valor quando há histórico acumulado. Depende de P1 e P2 estarem funcionando.

**Independent Test**: Com 2+ faturas de meses distintos, o dashboard exibe gráfico de tendência por categoria.

**Acceptance Scenarios**:

1. **Given** faturas de 2 ou mais meses distintos, **When** o usuário acessa o histórico, **Then** vê gráfico de linha ou barras comparando gastos por categoria ao longo dos meses.

---

### Edge Cases

- O que acontece quando o arquivo XLSX está vazio ou sem transações?
- Como o sistema lida com transações parceladas (mesma compra aparece em múltiplas faturas)?
- O que acontece quando a descrição da transação não permite inferir categoria?
- Como tratar estornos (valores negativos) — subtraem do total da categoria ou são exibidos separados?
- O que acontece quando dois arquivos XLSX cobrem o mesmo período (duplicata de fatura)?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema DEVE ler arquivos XLSX de faturas do cartão de crédito BTG localizados na pasta `faturas/` e extrair todas as transações.
- **FR-002**: O sistema DEVE identificar para cada transação: data, descrição do estabelecimento, valor e categoria (quando disponível na fatura).
- **FR-003**: O sistema DEVE agrupar transações por categoria e calcular o total gasto em cada categoria.
- **FR-004**: O sistema DEVE exibir um dashboard com as categorias de gasto ordenadas por valor total decrescente.
- **FR-005**: O sistema DEVE destacar visualmente a categoria com maior gasto e o valor correspondente.
- **FR-006**: O sistema DEVE exibir o percentual de cada categoria em relação ao total da fatura.
- **FR-007**: O sistema DEVE listar as 5 maiores transações individuais em destaque.
- **FR-008**: O sistema DEVE consolidar múltiplos arquivos XLSX de faturas distintas sem duplicar transações.
- **FR-009**: O sistema DEVE exibir mensagem de erro compreensível quando um arquivo XLSX não puder ser processado.
- **FR-010**: O sistema DEVE suportar estornos (valores negativos) e tratá-los separadamente ou subtraindo da categoria correspondente.
- **FR-011**: O sistema DEVE permitir filtrar o dashboard por fatura específica ou período de datas.
- **FR-012**: Quando há 2 ou mais faturas de meses distintos, o sistema DEVE exibir comparação de gastos por categoria entre períodos.

### Key Entities

- **Fatura**: Arquivo XLSX do BTG representando um ciclo de cobrança; contém data de referência, vencimento e lista de transações.
- **Transação**: Item individual de gasto; atributos: data, descrição do estabelecimento, valor (positivo = gasto, negativo = estorno), categoria.
- **Categoria**: Agrupamento lógico de transações similares (ex: Alimentação, Transporte, Saúde, Lazer, Compras, Outros); pode ser extraída da fatura ou inferida.
- **Dashboard**: Visão consolidada das transações de uma ou mais faturas com agregações por categoria e destaques de maior gasto.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% das transações de um arquivo XLSX BTG válido são importadas sem perda de dados.
- **SC-002**: O usuário identifica sua categoria de maior gasto em menos de 10 segundos ao acessar o dashboard.
- **SC-003**: O dashboard carrega e exibe os dados de uma fatura em menos de 5 segundos.
- **SC-004**: O agrupamento por categoria cobre 90% ou mais das transações (máximo 10% classificado como "Outros" em faturas típicas).
- **SC-005**: O sistema processa corretamente múltiplas faturas BTG sem gerar duplicatas, validado manualmente contra os arquivos originais.

## Assumptions

- O formato XLSX do BTG mantém estrutura consistente de colunas entre faturas (data, descrição, valor, categoria).
- O sistema é de uso pessoal/local — não há necessidade de autenticação ou múltiplos usuários.
- A pasta `faturas/` é o único ponto de entrada de dados; sem integração direta com APIs bancárias nesta versão.
- Suporte mobile está fora do escopo — interface desktop/web local é suficiente.
- Transações parceladas que aparecem em múltiplas faturas são tratadas como transações independentes por fatura (sem agrupamento cross-fatura de parcelas).
- As categorias presentes na fatura BTG são usadas como base; para transações sem categoria, o sistema infere pelo nome do estabelecimento usando mapeamento simples.
