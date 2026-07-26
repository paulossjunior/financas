# Feature Specification: Importar faturas de cartão Santander (PDF)

**Feature Branch**: `015-santander-invoice-adapter`

**Created**: 2026-07-26

**Status**: Draft

**Input**: User description: "Ler faturas de cartão Santander (PDF cifrado com senha) e importá-las no app com a mesma experiência das faturas BTG: prévia, categorização, dedup, dashboard, com o banco identificado."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Importar a fatura Santander (Priority: P1) 🎯 MVP

O usuário tem faturas do cartão Santander em PDF (baixadas do app/site do banco, protegidas
por senha — o CPF do titular). Ele seleciona um PDF na tela de Faturas, informa a senha uma
única vez, e as compras entram no app como qualquer fatura: categorizadas pelas regras
existentes, somadas no dashboard do mês, listadas na tela de faturas com o banco "Santander".

**Why this priority**: é o valor central da feature — sem a leitura do PDF nada mais existe.
O usuário hoje só consegue acompanhar o cartão BTG; o gasto do Santander (que inclui as
maiores despesas recorrentes dele, como serviços de nuvem e assinaturas) fica invisível.

**Independent Test**: com uma fatura real e a senha, importar → a lista de transações da
prévia/fatura bate linha a linha com o PDF (datas, descrições, valores em R$); o total do
mês no dashboard sobe exatamente o valor da fatura; reimportar o mesmo arquivo substitui em
vez de duplicar.

**Acceptance Scenarios**:

1. **Given** um PDF de fatura Santander e a senha correta salva, **When** o usuário importa
   o arquivo, **Then** todas as compras do "Detalhamento da Fatura" viram transações com
   data, descrição e valor em reais idênticos ao PDF, e a fatura aparece na lista com o
   banco "Santander" e o mês de referência correto.
2. **Given** a mesma fatura já importada, **When** o usuário importa o mesmo arquivo de
   novo, **Then** a fatura é substituída (mesma identidade) e nenhuma transação duplica.
3. **Given** um PDF cifrado e nenhuma senha salva, **When** o usuário importa, **Then** o
   app pede a senha, valida contra o arquivo e oferece guardá-la para as próximas
   importações (como já faz com a fatura BTG cifrada).
4. **Given** uma compra internacional no PDF (valor em US$ + cotação + IOF), **When**
   importada, **Then** a compra entra pelo valor em reais impresso e o IOF da compra entra
   como lançamento próprio — a soma da fatura fecha com o total impresso no PDF.
5. **Given** as linhas "PAGAMENTO DE FATURA"/"DEB AUTOM DE FATURA" (pagamento da fatura
   anterior) e "DESCONTO DO MES" (cashback), **When** a fatura é importada, **Then** os
   pagamentos de fatura NÃO viram despesa nem receita (são transferências já contadas no
   extrato), e o cashback entra como crédito abatendo o mês.

---

### User Story 2 - Confiança: a fatura confere ou não entra (Priority: P1)

Antes de gravar qualquer coisa, o app soma o que leu e confere com o bloco "Resumo da
Fatura" impresso no próprio PDF (Saldo Anterior + Despesas no Brasil + Despesas no Exterior
− Pagamentos − Créditos = Saldo Desta Fatura). Qualquer diferença — ou um PDF sem o resumo —
interrompe a importação com uma mensagem que diz a diferença em reais.

**Why this priority**: PDF é entrada frágil (linhas quebradas, layout que muda). Uma compra
perdida vira um mês silenciosamente mais barato — exatamente o tipo de erro que destrói a
confiança num app de finanças. Mesma política já adotada no extrato Banestes (014).

**Independent Test**: fixture com um valor adulterado → a importação recusa citando a
diferença; fixture íntegra → importa; fixture sem o bloco de resumo → recusa explicando o
que faltou.

**Acceptance Scenarios**:

1. **Given** uma fatura cuja soma das transações lidas fecha com o resumo impresso, **When**
   importada, **Then** grava e informa quantas transações entraram.
2. **Given** uma fatura em que a leitura perdeu ou distorceu uma linha, **When** importada,
   **Then** nada é gravado e a mensagem informa a diferença em reais.
3. **Given** um PDF sem o bloco "Resumo da Fatura" reconhecível, **When** importado,
   **Then** nada é gravado e a mensagem diz que a fatura não pôde ser conferida.

---

### User Story 3 - Senha errada, PDF alheio e outros formatos (Priority: P2)

O app trata os caminhos de erro com mensagens claras: senha incorreta pede de novo (sem
gravar a senha errada), PDF que não é fatura Santander (contracheque, extrato Banestes) é
recusado com explicação, e a fatura BTG continua funcionando exatamente como antes.

**Why this priority**: os PDFs convivem na mesma pasta (contracheques, extratos); um
contracheque interpretado como fatura inventaria despesas. Robustez aqui protege os dados,
mas depende da US1 existir.

**Acceptance Scenarios**:

1. **Given** um PDF Santander e uma senha errada, **When** o usuário importa, **Then** a
   mensagem diz que a senha não confere e o app pede de novo; a senha errada não é salva.
2. **Given** um PDF de contracheque ou de extrato Banestes, **When** selecionado na
   importação de faturas, **Then** o app recusa dizendo que o arquivo não é uma fatura
   reconhecida.
3. **Given** uma fatura BTG (.xlsx), **When** importada após esta feature, **Then** o
   comportamento é idêntico ao anterior (regressão zero).

---

### User Story 4 - Pasta de importação automática (Priority: P3)

PDF de fatura Santander na pasta configurada é importado sozinho na abertura do app, usando
a senha salva. Sem senha salva, o arquivo é listado como ignorado com um motivo claro (uma
vez, não como erro repetido). Contracheques e extratos na mesma pasta continuam indo para os
seus fluxos (extrato importa como extrato; contracheque é ignorado em silêncio).

**Why this priority**: conveniência sobre a US1; o usuário já usa a pasta automática para
BTG e Banestes.

**Acceptance Scenarios**:

1. **Given** a pasta com uma fatura Santander e a senha salva, **When** o app abre, **Then**
   o resumo informa a fatura importada; re-abrir não duplica.
2. **Given** a pasta com uma fatura Santander e nenhuma senha salva, **When** o app abre,
   **Then** a fatura aparece em "ignorados" com motivo de senha ausente e nada é gravado.
3. **Given** a pasta com extrato Banestes + contracheque + fatura Santander, **When** o app
   abre, **Then** cada arquivo vai para o seu fluxo: extrato → lançamentos bancários,
   fatura → transações de cartão, contracheque → ignorado em silêncio.

---

### Edge Cases

- Fatura com múltiplos cartões (físico + virtuais) no mesmo PDF: todas as subseções entram;
  nenhuma transação é atribuída ao cartão errado a ponto de mudar valores.
- Linha de despesa com valor 0,00 (ex.: "ANUIDADE DIFERENCIADA" isenta): não vira transação.
- Sub-linhas informativas ("COTAÇÃO DOLAR"): descartadas sem afetar a compra a que se referem.
- IOF de compra internacional: lançamento próprio com data da compra — sem ele o resumo não
  fecha; com ele duplicado, também não (a conferência pega os dois casos).
- Descrição de compra quebrada em mais de uma linha pelo extrator de texto: vira uma
  transação única (mesma técnica da 014); se não juntar, a conferência recusa.
- PDF não cifrado (banco pode mudar): importa sem pedir senha.
- Fatura de mês sem compras (só pagamento e anuidade zerada): importa com zero transações e
  informa isso — não é erro.
- Nome de arquivo fora do padrão `Fatura_MMYYYY_*`: mês de referência cai para a data de
  vencimento impressa no PDF.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST importar faturas de cartão Santander em PDF pela mesma tela e
  fluxo das faturas BTG, identificando o banco de cada fatura como "Santander".
- **FR-002**: O sistema MUST decifrar PDFs protegidos usando senha fornecida pelo usuário,
  oferecendo guardá-la com a mesma segurança da senha BTG (armazenamento de segredos do
  sistema operacional), em chave separada por banco.
- **FR-003**: O sistema MUST extrair de cada compra: data (dia/mês, com ano inferido do mês
  de referência da fatura), descrição e valor em reais; compras de todas as subseções de
  cartão (físico e virtuais) do "Detalhamento da Fatura".
- **FR-004**: O sistema MUST criar lançamento próprio para o IOF de cada compra
  internacional e descartar linhas informativas de cotação, de modo que a soma lida feche
  com os totais impressos.
- **FR-005**: O sistema MUST excluir das transações os pagamentos de fatura (transferências
  bancárias já visíveis no extrato) e MUST registrar cashback ("DESCONTO DO MES") como
  crédito que abate o mês.
- **FR-006**: O sistema MUST NOT criar transações de valor zero.
- **FR-007**: O sistema MUST conferir, antes de gravar, a identidade contábil do "Resumo da
  Fatura" (Saldo Anterior + Despesas Brasil + Despesas Exterior − Pagamentos − Créditos =
  Saldo Desta Fatura) contra os valores lidos; divergência ou resumo ausente MUST abortar a
  importação sem gravar nada, com mensagem em português informando a diferença em reais ou o
  que faltou.
- **FR-008**: O sistema MUST determinar o mês de referência pelo nome do arquivo
  (`Fatura_MMYYYY_…`) e, na ausência do padrão, pela data de vencimento impressa.
- **FR-009**: Reimportar o mesmo arquivo MUST substituir a fatura existente (mesma
  identidade derivada do nome do arquivo) sem duplicar transações — mesmo comportamento do
  BTG.
- **FR-010**: As transações importadas MUST passar pela mesma categorização por regras do
  app (com "Outros" como sobra) e aparecer no dashboard mensal, na lista de faturas, na
  previsão e nas telas agregadas, com o banco visível onde a fatura é listada.
- **FR-011**: O sistema MUST recusar, com mensagem clara, PDFs que não sejam fatura
  Santander (contracheques, extratos) e senhas incorretas (sem persistir a senha errada).
- **FR-012**: A pasta de importação automática MUST reconhecer e importar faturas Santander
  usando a senha salva; sem senha salva, MUST listar o arquivo como ignorado com motivo
  claro; MUST NOT quebrar os fluxos existentes (fatura BTG, extrato Banestes, contracheque
  ignorado em silêncio).
- **FR-013**: A importação da fatura BTG MUST permanecer byte-a-byte compatível: mesmas
  identidades de fatura/transação, mesmo comportamento.
- **FR-014**: Nenhum arquivo real de fatura (com dados pessoais) MUST entrar no
  repositório; testes usam texto anonimizado com valores e datas preservados.

### Key Entities

- **Fatura (Invoice)**: já existe; ganha origem "Santander" além de "BTG". Identidade
  derivada do nome do arquivo; carrega mês de referência e transações.
- **Transação (Transaction)**: já existe; uma compra, um IOF de compra internacional ou um
  crédito de cashback. Sem novos campos.
- **Resumo da fatura (novo, interno)**: os totais declarados no PDF usados na conferência —
  saldo anterior, despesas Brasil, despesas exterior, pagamentos, créditos, saldo da fatura.
- **Senha por banco (segredo)**: senha de PDF do Santander, guardada no armazenamento de
  segredos do SO sob chave própria, independente da senha BTG.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: As 4 faturas reais do usuário (fev, mai, jun e jul/2026, dois cartões)
  importam com 100% das compras do PDF presentes e com o total de cada fatura igual ao
  "Saldo Desta Fatura" impresso.
- **SC-002**: Reimportar qualquer fatura já importada resulta em 0 transações duplicadas.
- **SC-003**: Uma fatura com qualquer linha perdida ou valor distorcido na leitura é
  recusada em 100% dos casos testados (nenhum mês "mais barato" silencioso).
- **SC-004**: Contracheques e extratos Banestes apresentados ao fluxo de fatura são
  recusados em 100% dos casos testados.
- **SC-005**: Depois de salvar a senha uma vez, importar as demais faturas não pede senha de
  novo (zero prompts adicionais nas importações seguintes, manual e automática).
- **SC-006**: A suíte de faturas BTG existente permanece 100% verde (regressão zero).

## Assumptions

- A senha dos PDFs Santander do usuário é o CPF do titular (11 dígitos) e é a mesma para
  todos os cartões — mas o sistema não assume isso: aceita qualquer senha e guarda a que
  funcionar.
- O layout investigado (4 faturas reais, cartões "ELITE CASHBACK VISA SIGNATURE", fev–jul
  2026) é representativo; variações futuras de layout são protegidas pela conferência
  obrigatória (falham visivelmente, nunca importam errado).
- Parcelamentos ("Parcela" na tabela do PDF) não aparecem em nenhuma das 4 faturas reais;
  o formato impresso da coluna é tratado como melhor esforço e a conferência garante que
  uma parcela mal lida não passe.
- O valor da compra internacional considerado é o valor em reais impresso na fatura (já
  convertido pelo banco); o valor em US$ é ignorado.
- "Melhor dia de compra", limites, encargos, boleto e propaganda de parcelamento do PDF
  estão fora do escopo — só o detalhamento de transações e o resumo entram no app.
- A fatura Santander não traz categoria por transação (diferente do BTG): toda categorização
  vem das regras do app; sem regra, a transação cai em "Outros".
