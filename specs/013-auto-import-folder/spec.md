# Feature Specification: Pasta de Importação Automática

**Feature Branch**: `013-auto-import-folder`

**Created**: 2026-07-24

**Status**: Draft

**Input**: User description: "especifique uma pasta para ler sempre as faturas e os extratos" — uma pasta única, importação automática ao abrir o app.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Definir a pasta de importação (Priority: P1)

Na tela de Configurações o usuário escolhe uma pasta do computador onde guarda
as faturas do cartão (.xlsx) e os extratos bancários (.xls/.xlsx), tudo junto.
Ao definir a pasta, o app já varre o que está lá e importa, identificando
sozinho o que é fatura e o que é extrato. O usuário passa a ter um único lugar
para "jogar" os arquivos, sem escolher arquivo por arquivo a cada importação.

**Why this priority**: É o coração do pedido ("ler sempre de uma pasta") e
entrega valor imediato: definiu a pasta, os dados entram. Independe da execução
automática no boot (US2) — pode ser testado só definindo a pasta.

**Independent Test**: Definir uma pasta contendo uma fatura e um extrato;
confirmar que ambos foram importados e aparecem no painel, sem seleção manual
de arquivos.

**Acceptance Scenarios**:

1. **Given** uma pasta com faturas e extratos, **When** o usuário a define em
   Configurações, **Then** o app importa os arquivos válidos, identificando o
   tipo de cada um, e mostra um resumo (quantas faturas, quantos extratos,
   quantos ignorados).
2. **Given** a pasta já foi importada antes, **When** o usuário reimporta (ou o
   app varre de novo), **Then** nada é duplicado — só arquivos/lançamentos novos
   entram.
3. **Given** um arquivo na pasta que não é fatura nem extrato reconhecível,
   **When** a varredura ocorre, **Then** esse arquivo é ignorado com aviso, sem
   interromper a importação dos demais.
4. **Given** o usuário quer parar de usar a pasta, **When** ele limpa a
   configuração, **Then** o app deixa de varrer a pasta ao abrir.

---

### User Story 2 - Importar automaticamente ao abrir o app (Priority: P2)

Com a pasta configurada, toda vez que o usuário abre o app ele varre a pasta e
importa automaticamente os arquivos novos, sem nenhum clique. Assim, basta
salvar a fatura/extrato na pasta e abrir o app para os dados estarem lá.

**Why this priority**: É a conveniência "sempre" do pedido. Depende de US1 (a
pasta precisa estar definida), por isso vem depois.

**Independent Test**: Com a pasta definida, adicionar um arquivo novo à pasta,
abrir o app e verificar que o arquivo foi importado sem ação manual.

**Acceptance Scenarios**:

1. **Given** a pasta configurada com arquivos novos desde a última abertura,
   **When** o app é aberto, **Then** os novos são importados automaticamente e o
   painel já reflete os dados.
2. **Given** a abertura do app com a pasta configurada, **When** a importação
   automática termina, **Then** o app exibe um resumo discreto do que foi
   importado/ignorado (visibilidade de status), sem bloquear o uso.
3. **Given** nenhuma pasta configurada, **When** o app abre, **Then** nada é
   varrido e o comportamento atual (importação manual) permanece.
4. **Given** a pasta configurada não existe mais (movida/apagada), **When** o
   app abre, **Then** ele não trava; avisa que a pasta não foi encontrada.

---

### Edge Cases

- **Fatura protegida por senha**: se houver senha salva no dispositivo, o app a
  usa; senão, ignora o arquivo com aviso (não trava esperando digitação).
- **Pasta grande / muitos arquivos**: a abertura não congela indefinidamente; o
  usuário percebe que uma importação está em curso.
- **Mesmo arquivo já importado**: dedup evita recontar (fatura por nome/ível;
  extrato por lançamento).
- **Arquivo corrompido ou de outro formato**: ignorado com aviso; os demais
  seguem importando.
- **Pasta sem permissão de leitura**: aviso claro; sem travar o app.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema DEVE permitir configurar, na tela de Configurações, uma
  pasta única de onde faturas e extratos são lidos, escolhida via diálogo do SO.
- **FR-002**: O sistema DEVE identificar automaticamente, para cada arquivo da
  pasta, se é fatura de cartão ou extrato bancário, e importá-lo pelo fluxo
  correto.
- **FR-003**: Ao definir/alterar a pasta, o sistema DEVE varrer e importar seu
  conteúdo imediatamente, mostrando um resumo do resultado.
- **FR-004**: Ao abrir o app, se houver pasta configurada, o sistema DEVE
  varrê-la e importar automaticamente os arquivos, sem ação do usuário.
- **FR-005**: A importação (manual ou automática) NÃO DEVE duplicar dados já
  importados (dedup de faturas e de lançamentos de extrato).
- **FR-006**: Arquivos não reconhecidos, corrompidos ou que exijam senha
  indisponível DEVEM ser ignorados com aviso, sem interromper os demais.
- **FR-007**: O sistema DEVE exibir um resumo do que foi importado e ignorado
  (contagens por tipo), tanto na importação ao definir a pasta quanto na
  automática ao abrir.
- **FR-008**: O usuário DEVE poder limpar/desativar a pasta; sem pasta
  configurada, o app não varre nada e mantém a importação manual existente.
- **FR-009**: Se a pasta configurada não existir ou não puder ser lida na
  abertura, o app NÃO DEVE travar; DEVE avisar e seguir normalmente.
- **FR-010**: Toda a leitura/importação DEVE ocorrer localmente, sem rede.
- **FR-011**: A importação manual arquivo-a-arquivo existente DEVE continuar
  funcionando (a pasta é um complemento, não substitui).

### Key Entities *(include if feature involves data)*

- **Pasta de importação**: caminho único (opcional) configurado pelo usuário;
  fonte de faturas e extratos. Vazio = recurso desligado.
- **Resumo de importação**: resultado de uma varredura — quantidades de faturas
  importadas, extratos importados, arquivos ignorados (com motivo), erros.
- **Fatura** / **Extrato**: entidades já existentes; esta feature apenas as
  alimenta a partir da pasta.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Após definir a pasta com faturas e extratos, 100% dos arquivos
  válidos são importados sem seleção manual arquivo-a-arquivo.
- **SC-002**: Abrir o app com arquivos novos na pasta resulta nesses dados no
  painel sem nenhum clique de importação.
- **SC-003**: Reabrir o app N vezes com a mesma pasta não cria nenhuma
  duplicata (0% de dados duplicados).
- **SC-004**: Um arquivo inválido na pasta nunca impede a importação dos válidos
  (os válidos entram; o inválido é reportado).
- **SC-005**: Com pasta inexistente/ilegível, o app abre normalmente em 100% das
  vezes (sem travar) e informa o problema.

## Assumptions

- **Uma pasta só** para faturas e extratos (decisão do usuário); o app distingue
  o tipo pelo conteúdo/formato do arquivo.
- Importação **automática ao abrir** (decisão do usuário); não há agendamento em
  segundo plano enquanto o app está aberto.
- Faturas de cartão são `.xlsx` (BTG); extratos são `.xls`/`.xlsx` (BTG). Outros
  bancos/formatos ficam fora do escopo (ignorados com aviso).
- Reutiliza os fluxos e parsers de importação existentes (fatura e extrato) e o
  mecanismo de dedup determinístico já presente.
- A pasta é um caminho absoluto no computador do usuário; substitui, na prática,
  o antigo campo relativo "Pasta das Faturas" (que não era usado para leitura).
- Público-alvo: usuário único, uso pessoal.
