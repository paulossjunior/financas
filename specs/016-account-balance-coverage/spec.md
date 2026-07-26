# Feature Specification: Saldo de conta, cobertura de dados e conferência por segmento

**Feature Branch**: `016-account-balance-coverage`

**Created**: 2026-07-26

**Status**: Draft

**Input**: User description: "Dar ao app o conceito de saldo (estoque) e de cobertura de
dados, usando o que o extrato bancário já imprime e hoje é descartado: snapshot de saldo
por conta a cada importação, período coberto por importação (mês parcial, buracos,
encadeamento), e conferência por segmento com os saldos diários do extrato."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Ver o saldo da conta no painel (Priority: P1) 🎯 MVP

Hoje o app responde "quanto saiu em julho" mas não responde a pergunta mais básica de
finanças pessoais: **"quanto eu tenho agora?"**. O extrato que o usuário já importa traz
essa resposta impressa (saldo final da conta, e o da poupança no extrato consolidado) — e o
app a joga fora. Depois desta feature, cada importação de extrato registra a posição da
conta, e o painel mostra "Saldo em conta: R$ 231,30 · extrato de 25/07" por conta, com o
total somado quando houver mais de uma conta/produto.

**Why this priority**: é o conceito de *estoque* nascendo no app com custo mínimo — o dado
já é lido e conferido durante a importação; falta só guardá-lo e mostrá-lo. Sem ele,
qualquer projeção futura de caixa não tem âncora.

**Independent Test**: importar o extrato real de julho → o painel mostra o saldo da conta
igual ao impresso no PDF, com a data do fim do período; reimportar o mesmo extrato não
duplica nem altera; importar um extrato mais novo substitui a posição corrente.

**Acceptance Scenarios**:

1. **Given** um extrato Banestes importado (saldo final R$ 231,30, período até 25/07),
   **When** o usuário abre o painel, **Then** vê "Saldo em conta" da conta Banestes com
   R$ 231,30 e a data 25/07/2026 como referência.
2. **Given** o mesmo extrato importado de novo, **When** o painel recarrega, **Then** o
   saldo continua um só (mesma posição, sem duplicata).
3. **Given** um extrato mais novo importado (período até 31/08), **When** o painel
   recarrega, **Then** a posição corrente passa a ser a do extrato mais novo; a anterior
   permanece registrada como histórico.
4. **Given** um extrato consolidado que imprime também o saldo da poupança, **When**
   importado, **Then** a posição da poupança é registrada em separado e o total do painel
   soma conta corrente + poupança.
5. **Given** um extrato antigo importado depois de um mais novo (chegou atrasado),
   **When** o painel recarrega, **Then** a posição corrente continua sendo a do período
   mais recente (não a do último import).

---

### User Story 2 - Saber se o mês está completo (Priority: P1)

Números de mês parcial parecem mês barato — o silêncio mente. Cada importação de extrato
passa a registrar o período que cobre (impresso no cabeçalho: "Período: 01/07/2026 à
25/07/2026"). Com isso o app: marca o mês como **parcial** nas telas de resumo quando a
cobertura não alcança o fim do mês; aponta **buracos** (meses sem nenhuma cobertura entre o
primeiro e o último extrato importados); e confere o **encadeamento** — o "Saldo Anterior"
de um extrato novo deve bater com o saldo final do período imediatamente anterior já
importado, senão o app avisa (sem bloquear: pode ser só um extrato faltando no meio).

**Why this priority**: mesma filosofia da conferência das features 014/015 — dado
incompleto tem de se declarar incompleto. Protege a confiança do usuário nos totais.

**Independent Test**: importar só o extrato de 01–25/07 → o mês de julho aparece marcado
como parcial na tela de extrato/movimentações; importar extratos de maio e julho (sem
junho) → aviso de buraco em junho; importar extrato cujo saldo anterior não bate com o
final do anterior → aviso de encadeamento.

**Acceptance Scenarios**:

1. **Given** cobertura 01–25/07 registrada, **When** o usuário vê julho nas telas que somam
   extrato, **Then** há indicação visível de "dados até 25/07" (mês parcial).
2. **Given** extratos cobrindo maio e julho, sem junho, **When** o usuário abre a tela de
   extrato, **Then** vê aviso de que junho não tem cobertura.
3. **Given** um extrato novo cujo "Saldo Anterior" difere do saldo final do período
   anterior registrado, **When** importado, **Then** a importação conclui e o usuário
   recebe aviso do desencontro com os dois valores (provável extrato faltando entre eles).
4. **Given** dois extratos com períodos sobrepostos (01–25/07 e 20/07–10/08), **When**
   importados, **Then** a cobertura resultante é a união (sem dupla contagem de dias) e os
   lançamentos duplicados continuam protegidos pelo dedup existente.

---

### User Story 3 - Conferência por segmento no parser Banestes (Priority: P2)

O extrato imprime o saldo após cada dia de movimentação ("JUL/26 Saldo 6.637,41"). A
conferência atual fecha o total do período; a por segmento fecha **cada trecho** — pegando
o erro que se auto-cancela (uma linha perdida de +100 e outra de −100 somam zero e passam
hoje). Segmento que não fecha recusa a importação com a mensagem dizendo o dia e a
diferença, igual à política das conferências existentes.

**Why this priority**: robustez pura, invisível quando tudo vai bem. Depende só do parser;
não muda tela nenhuma.

**Independent Test**: fixture com duas linhas adulteradas que se cancelam (+100/−100) →
recusada citando o primeiro segmento que não fecha; fixtures íntegras existentes →
continuam importando.

**Acceptance Scenarios**:

1. **Given** a fixture íntegra de julho, **When** importada, **Then** todos os segmentos
   diários fecham e a importação segue como hoje.
2. **Given** uma fixture com +100 numa linha e −100 em outra (soma total intacta),
   **When** importada, **Then** a importação é recusada citando o dia do primeiro segmento
   divergente e a diferença em reais; nada é gravado.
3. **Given** um extrato sem os saldos intermediários impressos (layout futuro), **When**
   importado, **Then** a conferência por segmento não roda (sem dados), a conferência
   total continua obrigatória, e nada muda para o usuário.

---

### Edge Cases

- Extrato de período que cruza meses (20/07–10/08): a cobertura marca julho parcial a
  partir de 20/07 e agosto parcial até 10/08; o "mês completo" exige união de coberturas.
- Primeira importação de todas (sem posição anterior): encadeamento não tem com o que
  comparar — sem aviso.
- Extrato antigo importado após o novo: histórico de posições ordena por fim de período,
  não por ordem de importação.
- Conta nova (numeração diferente): posições e coberturas são por conta — contas não se
  misturam.
- Extrato do mesmo período reimportado com conteúdo idêntico: posição, cobertura e
  lançamentos idênticos (idempotência total).
- Extrato BTG (.xls) sem saldos/período impressos: nada é registrado de posição/cobertura
  para ele; as telas mostram posição só das contas que têm dado (sem inventar).
- Dia sem movimentação não imprime saldo: segmentos são delimitados pelos saldos que
  existem; ausência de um dia não é erro.
- Remoção de todos os lançamentos de um extrato (limpar extrato): as posições/coberturas
  daquela importação também saem, para não sobrar saldo órfão.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Cada importação de extrato que imprimir saldo final MUST registrar uma
  posição de conta: banco, identificação da conta, saldo, data-base (fim do período
  coberto) e, quando o extrato consolidado imprimir, a posição da poupança como produto
  separado da mesma conta.
- **FR-002**: Reimportar o mesmo extrato MUST resultar na mesma posição (idempotência);
  posições MUST ser identificadas deterministicamente por conta/produto + data-base.
- **FR-003**: O painel MUST exibir a posição corrente por conta (valor + data-base) e o
  total somado; a posição corrente é a de **data-base mais recente**, independente da
  ordem de importação.
- **FR-004**: Cada importação de extrato que imprimir o período MUST registrar a cobertura
  (data inicial e final) por conta; coberturas sobrepostas MUST ser tratadas como união.
- **FR-005**: Telas que somam lançamentos de extrato por mês MUST indicar quando o mês tem
  cobertura parcial (não alcança o fim do mês) e MUST permitir ver até que dia há dados.
- **FR-006**: A tela de extrato MUST apontar meses sem nenhuma cobertura entre o primeiro
  e o último período registrados da conta.
- **FR-007**: Ao importar um extrato cujo "Saldo Anterior" divergir do saldo final da
  posição imediatamente anterior da mesma conta, o sistema MUST concluir a importação e
  MUST avisar o desencontro citando os dois valores; ausência de posição anterior não gera
  aviso.
- **FR-008**: O parser Banestes MUST conferir cada segmento delimitado pelos saldos
  intermediários impressos; segmento divergente MUST abortar a importação citando o dia e
  a diferença em reais; extrato sem saldos intermediários MUST cair apenas na conferência
  total existente.
- **FR-009**: Nada nesta feature MUST alterar a semântica dos lançamentos (exclusões,
  categorização, dedup, ids) nem os fluxos de fatura/contracheque.
- **FR-010**: Valores monetários MUST usar aritmética decimal exata; dados MUST persistir
  localmente; nenhum arquivo real de extrato MUST entrar no repositório (fixtures
  anonimizadas — as existentes já contêm os saldos diários e o período).
- **FR-011**: Limpar os dados de extrato de uma conta MUST remover também as posições e
  coberturas associadas (sem saldo órfão).

### Key Entities

- **Posição de conta (Account Position/Snapshot — novo)**: banco, conta, produto (conta
  corrente | poupança), saldo, data-base, origem (arquivo importado). Identidade
  determinística por (conta, produto, data-base).
- **Cobertura (Coverage — novo)**: por conta: período coberto (início, fim) de cada
  importação; derivadas: meses parciais, buracos, encadeamento.
- **Extrato/lançamentos (existentes)**: intactos; a importação passa a *também* produzir
  posição + cobertura.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Após importar o extrato real de julho, o painel mostra o saldo idêntico ao
  impresso no PDF (R$ 231,30) com a data-base 25/07/2026; reimportar mantém exatamente 1
  posição para essa data-base.
- **SC-002**: Com extratos de maio e julho importados (sem junho), o app aponta junho como
  buraco em 100% dos casos testados; com cobertura 01–25/07, julho aparece como parcial.
- **SC-003**: Extrato com saldo anterior divergente da posição anterior gera aviso com os
  dois valores em 100% dos casos testados, sem bloquear a importação.
- **SC-004**: Fixture com erros que se auto-cancelam (+100/−100) é recusada em 100% dos
  casos, citando o primeiro dia divergente; as fixtures íntegras existentes continuam
  importando sem mudança.
- **SC-005**: Suítes existentes de extrato/fatura permanecem 100% verdes (regressão zero
  nos fluxos).

## Assumptions

- O extrato Banestes é a fonte inicial de posição/cobertura (imprime tudo). O extrato BTG
  (.xls) só participa se os dados existirem no arquivo; ausência = sem posição/cobertura,
  sem erro.
- "Posição corrente" = maior data-base registrada da conta/produto; não há edição manual
  de posições (fora de escopo: CRUD de contas).
- O aviso de encadeamento é informativo (não bloqueia): o caso comum é extrato faltando
  no meio, que o usuário resolve importando o período que falta.
- Fatura de cartão não gera posição (dívida de cartão fora do escopo desta feature).
- Projeção de caixa, orçamento e metas são features futuras que consumirão estas
  entidades; nada delas entra aqui.
