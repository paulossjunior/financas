# Feature Specification: Ler extrato bancário do Banestes (adapter)

**Feature Branch**: `014-banestes-statement-adapter`

**Created**: 2026-07-25

**Status**: Draft

**Input**: User description: "quero uma nova funcionalidade que ler o extrato bancário do banestes. Para isso faça um adapter .. não precisa criar novas entidades. Apenas um adapter que reusa o que foi feito com o BTG."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Importar o extrato do Banestes (Priority: P1)

Como usuário, quero importar o extrato de conta corrente do Banestes (o PDF que baixo do
internet banking) e ter os lançamentos reais entrando no app do mesmo jeito que já entram
os do BTG, para não precisar digitar nada nem converter arquivo.

**Why this priority**: É o valor central da feature. Sem isso a conta Banestes fica invisível
no app e os totais do mês ficam incompletos.

**Independent Test**: Selecionar o PDF do extrato Banestes na tela de Extrato; o app mostra a
prévia com os lançamentos (data, descrição, valor, débito/crédito) e, ao confirmar, eles
aparecem na lista de lançamentos importados e nos totais do mês.

**Acceptance Scenarios**:

1. **Given** o extrato Banestes de 01/07/2026 a 25/07/2026 (9 lançamentos; entradas R$ 0,00,
   saídas R$ 7.106,11, saldo anterior R$ 7.337,41, saldo final R$ 231,30), **When** importo,
   **Then** os 9 lançamentos entram com data, descrição, valor e tipo corretos, as saídas somam
   R$ 7.106,11 e as entradas somam R$ 0,00.
2. **Given** o extrato traz linhas de saldo ("Saldo Anterior", "Saldo" do dia, "Saldo Conta",
   "Saldo Total"), o resumo do topo e o rodapé de emissão, **When** importo, **Then** nada
   disso entra como lançamento.
3. **Given** um lançamento cuja descrição ocupa duas linhas no PDF (ex.: "ALFA COMERCIO E
   REPRESENTACOES LTDA"), **When** importo, **Then** ele entra como **um** lançamento com a
   descrição completa e o valor de R$ 2.729,78.
4. **Given** um lançamento lançado no dia 20 mas com data de operação 19/07/2026, **When**
   importo, **Then** a data usada é a data da operação mostrada no lançamento.
5. **Given** importo o mesmo extrato de novo, **When** confirmo, **Then** nenhum lançamento
   novo é criado.
6. **Given** um PDF que não é extrato Banestes (ex.: contracheque, PDF digitalizado sem
   texto), **When** tento importar, **Then** vejo uma mensagem clara e nada é gravado.

---

### User Story 2 - Mesmas regras de exclusão e categorização do BTG (Priority: P1)

Como usuário, quero que o extrato Banestes obedeça exatamente às regras que já valem para o
BTG — não contar duas vezes o que já vem de outra fonte e categorizar pelas minhas palavras-chave
— para meus totais continuarem coerentes com uma só lógica.

**Why this priority**: Sem isso a nova fonte infla os totais e cria uma segunda lógica de
categorização para manter. A reutilização é o pedido explícito da feature.

**Independent Test**: Importar um extrato Banestes que contenha pagamento de fatura de cartão,
crédito de salário (num mês com contracheque importado) e uma transferência para conta do
próprio titular; a prévia mostra os três como excluídos, com o motivo.

**Acceptance Scenarios**:

1. **Given** um pagamento de fatura de cartão no extrato, **When** importo, **Then** ele é
   excluído com motivo "já vem da fatura".
2. **Given** um crédito de salário e existe contracheque no mesmo mês, **When** importo,
   **Then** ele é excluído com motivo "já vem do contracheque"; sem contracheque no mês, entra
   como renda.
3. **Given** uma transferência cuja contraparte é o próprio titular do extrato, **When**
   importo, **Then** ela é excluída com motivo "transferência interna".
4. **Given** um débito cuja descrição casa com uma palavra-chave minha (ex.: "GIGA MAIS FIBRA"
   → Internet), **When** importo, **Then** ele já entra categorizado.
5. **Given** um débito sem palavra-chave correspondente, **When** importo, **Then** ele entra
   como "Outros" e aparece na fila de categorização, junto com os "Outros" do cartão e do BTG.

---

### User Story 3 - Duas contas convivendo (Priority: P2)

Como usuário, quero ver de qual banco veio cada lançamento importado e ter os dois extratos
somando nos mesmos totais, para acompanhar tudo numa visão só.

**Why this priority**: Passa a existir mais de uma origem de extrato; sem identificar o banco
o usuário não sabe o que remover nem confere com o app do banco.

**Independent Test**: Importar um extrato BTG e um Banestes; a lista mostra o banco de cada
lançamento e as telas de movimentações/despesas somam os dois.

**Acceptance Scenarios**:

1. **Given** lançamentos importados dos dois bancos, **When** abro a lista de extrato, **Then**
   vejo o banco (e a conta) de cada lançamento e consigo remover ou recategorizar qualquer um.
2. **Given** lançamentos dos dois bancos no mesmo mês, **When** abro as movimentações do mês,
   **Then** entradas e saídas somam as duas contas, sem dupla contagem.

---

### User Story 4 - Pasta de importação automática reconhece o Banestes (Priority: P3)

Como usuário que já configurou uma pasta de importação automática, quero que o extrato Banestes
que eu jogar nessa pasta seja importado sozinho, como já acontece com as faturas e os extratos
do BTG.

**Why this priority**: Conveniência; a importação manual (US1) já entrega o valor completo.

**Independent Test**: Colocar o PDF do extrato Banestes na pasta configurada, abrir o app e ver
o resumo informando 1 extrato importado com N lançamentos.

**Acceptance Scenarios**:

1. **Given** a pasta configurada com um PDF de extrato Banestes, **When** o app varre a pasta,
   **Then** o extrato é importado e contado no resumo.
2. **Given** a pasta também contém um PDF de contracheque, **When** o app varre a pasta,
   **Then** o contracheque **não** é tratado como extrato (nenhum lançamento errado é criado).
3. **Given** a mesma pasta é varrida de novo, **When** o app varre, **Then** nada é duplicado.

---

### Edge Cases

- **Texto do PDF fora de ordem**: o PDF do Banestes é um relatório em colunas; a extração de
  texto pode devolver os valores separados dos lançamentos a que pertencem. O valor de cada
  lançamento precisa continuar sendo o valor daquela linha — nunca o de outra linha nem um saldo.
- **Lançamento em duas linhas**: contraparte com nome longo quebra em duas linhas e não pode
  virar dois lançamentos.
- **Coluna de dia sem mês repetido**: o mês ("JUL/26") aparece só na primeira linha do grupo de
  dias; as linhas seguintes herdam.
- **Data da operação ≠ dia da coluna** (operação em 19, lançada em 20): vale a data da operação.
- **Lançamento sem data de operação no texto**: usa o dia da coluna com o mês/ano do grupo.
- **Créditos**: valor sem sinal negativo = crédito (renda); com sinal = débito (despesa).
- **Extrato que atravessa meses**: cada lançamento entra no seu próprio mês.
- **Dois lançamentos idênticos no mesmo dia** (mesma contraparte, mesmo valor): devem permanecer
  dois lançamentos distintos.
- **Saldos não fecham** (soma dos lançamentos ≠ saldo anterior − saldo final): sinal de que a
  leitura perdeu ou trocou algo → falha explícita, nada é gravado.
- **PDF protegido por senha, digitalizado (imagem) ou de outro banco**: mensagem clara, nada
  importado.
- **Extrato com zero lançamentos no período**: mensagem informando que não há lançamentos, sem erro.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST reconhecer um extrato de conta corrente do Banestes em PDF e extrair
  titular, conta (agência + número) e período.
- **FR-002**: O sistema MUST extrair cada lançamento com data da operação, tipo de operação,
  contraparte e valor com sinal (negativo = débito/saída, positivo = crédito/entrada). Ruído da
  linha que não descreve o lançamento (hora, número de documento do banco) MUST ficar fora da
  descrição mostrada ao usuário.
- **FR-003**: O sistema MUST ignorar linhas de saldo ("Saldo Anterior", "Saldo", "Saldo Conta",
  "Saldo Total"), cabeçalhos de coluna, o resumo do topo e o rodapé de emissão.
- **FR-004**: O sistema MUST manter cada valor associado ao lançamento correto mesmo quando o texto
  extraído do PDF vem fora da ordem visual, e MUST unir descrições quebradas em mais de uma linha
  num único lançamento.
- **FR-005**: O sistema MUST conferir os lançamentos extraídos contra os totais de **entradas e
  saídas** declarados no próprio extrato (e, quando presentes, contra saldo anterior e saldo final):
  a soma dos créditos MUST igualar as entradas declaradas e a soma dos débitos MUST igualar as saídas
  declaradas. Se não fechar, MUST recusar a importação com mensagem clara e não gravar nada.
- **FR-006**: O sistema MUST aplicar aos lançamentos do Banestes as **mesmas regras de exclusão** já
  usadas no extrato BTG — pagamento de fatura de cartão, salário quando há contracheque no mês,
  transferência entre contas do próprio titular — reconhecendo também os termos que o Banestes usa.
- **FR-007**: O sistema MUST categorizar os lançamentos incluídos pelas regras (palavras-chave) do
  app; sem correspondência, o lançamento fica "Outros" e entra na fila de categorização existente
  (o Banestes não informa categoria própria, logo não há fallback de categoria do banco).
- **FR-008**: O sistema MUST gravar os lançamentos incluídos no **mesmo modelo de dados** já usado
  pelo extrato BTG, identificando o banco de origem como Banestes e a conta do extrato — **sem criar
  novas entidades de dados**.
- **FR-009**: O sistema MUST deduplicar reimportações do mesmo extrato (reimportar não cria
  lançamento novo) e, ao mesmo tempo, MUST preservar como distintos dois lançamentos diferentes que
  compartilhem data, descrição e valor. Lançamentos do BTG já importados MUST continuar sendo
  reconhecidos como os mesmos (nenhuma reimportação passada volta a duplicar).
- **FR-010**: O sistema MUST oferecer a mesma prévia do extrato BTG antes de gravar: incluídos com
  categoria (editável) e excluídos com o motivo.
- **FR-011**: Usuários MUST conseguir selecionar um PDF na tela de importação de extrato, e o sistema
  MUST identificar o banco pelo conteúdo do arquivo — o usuário não informa o banco.
- **FR-012**: O sistema MUST mostrar o banco (e a conta) de origem na lista de lançamentos
  importados, mantendo remover, limpar e recategorizar como hoje.
- **FR-013**: O sistema MUST somar os lançamentos do Banestes nos mesmos totais e telas em que já
  entram os do BTG (painel do mês, movimentações, despesas e receitas, recorrentes derivados), sem
  dupla contagem.
- **FR-014**: O sistema MUST tratar PDF ilegível, sem texto extraível, protegido por senha ou de
  outro banco com mensagem clara, sem importar nada.
- **FR-015**: A varredura da pasta de importação automática MUST reconhecer PDFs de extrato Banestes
  e MUST não confundi-los com contracheques; arquivos não reconhecidos continuam apenas reportados
  como ignorados, sem abortar a varredura.
- **FR-016**: Todo o processamento MUST ser local; nenhum dado do extrato sai da máquina.

### Key Entities *(include if feature involves data)*

Nenhuma entidade nova. A feature reutiliza as existentes do extrato BTG:

- **Lançamento de extrato**: data, descrição, valor com sinal, tipo (crédito/débito), categoria,
  mês, banco e conta de origem, marca de inclusão + motivo de exclusão. O que muda é apenas o valor
  do campo de banco ("Banestes") e o fato de a categoria informada pelo banco vir vazia.
- **Extrato**: arquivo importado (titular, conta, período) que agrupa lançamentos.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Importando o extrato Banestes de julho/2026, o usuário obtém **9 lançamentos** com
  valores idênticos aos do PDF, somando **R$ 7.106,11** de saídas e reconciliando com o saldo final
  de **R$ 231,30**.
- **SC-002**: 100% dos extratos válidos importados têm a soma dos lançamentos batendo com os saldos
  declarados; nenhum extrato é gravado com valor trocado, perdido ou duplicado.
- **SC-003**: O usuário importa o extrato sem informar de qual banco é o arquivo e sem converter
  formato — mesmo número de passos do fluxo BTG atual (selecionar arquivo → revisar prévia → confirmar).
- **SC-004**: Reimportar o mesmo extrato cria **zero** lançamentos novos.
- **SC-005**: Com extratos dos dois bancos no mesmo mês, os totais das telas de movimentações e de
  despesas/receitas equivalem à soma das duas contas, e nenhum item já contado por fatura ou
  contracheque aparece nos totais.
- **SC-006**: Arquivo inválido ou de outro tipo resulta em mensagem que diz o que está errado e zero
  registros gravados.
- **SC-007**: Todo o fluxo funciona sem rede.

## Assumptions

- **Formato de entrada**: PDF de "Extrato de Conta Corrente" gerado pelo Banestes, com cabeçalho
  Agência/Conta/Cliente/Período, colunas Data · Lançamento · Valor (R$) e bloco final "Saldos".
  O Banestes não classifica os lançamentos por categoria — diferente do BTG, que traz uma coluna de
  categoria usada como fallback.
- **Reuso**: a leitura do arquivo é a única parte nova (um adapter de leitura por banco); classificação,
  exclusão, categorização, deduplicação, gravação e telas são as já existentes do extrato BTG. Nenhuma
  entidade nova, nenhuma tabela nova, nenhuma tela nova.
- **Data do lançamento**: vale a data da operação impressa no próprio lançamento; a coluna de dia é
  usada apenas como fallback (com o mês/ano do grupo).
- **Débitos** entram como despesas avulsas e **créditos** como renda extra, por mês — igual ao BTG.
- **Detecção do banco** é feita pelo conteúdo do arquivo, não pela extensão nem pelo nome: um `.pdf`
  pode ser extrato Banestes ou contracheque.
- **Testes** usam um extrato de exemplo **anonimizado** (titular, contrapartes e número de conta
  substituídos), preservando datas e valores — o extrato real não vai para o repositório.
- **Dependências**: depende do fluxo de extrato existente (feature 008), do contracheque (feature 004,
  para a regra de salário) e da varredura de pasta (feature 013, apenas para a US4).

## Out of Scope

- Outros bancos além de BTG e Banestes.
- Outros formatos do Banestes (OFX, CSV, .xls) e extratos de poupança, cartão ou investimento.
- Integração com API/Open Finance do banco.
- Qualquer mudança nas regras de categorização, no painel ou nos relatórios além de passar a incluir
  a nova origem.
