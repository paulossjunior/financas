---

description: "Task list — adapter de extrato Banestes (PDF)"
---

# Tasks: Adapter de extrato Banestes (PDF)

**Input**: Design documents from `/specs/014-banestes-statement-adapter/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/)

**Tests**: OBRIGATÓRIOS. A Constituição do projeto (Princípio I, NON-NEGOTIABLE) exige teste
antes de implementação, com ≥ 90% de cobertura em parsers. Toda tarefa de implementação abaixo
é precedida pelo seu teste vermelho.

**Organization**: agrupado por user story, na ordem de prioridade do spec.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência pendente)
- **[Story]**: user story a que pertence (US1…US4)

## Path Conventions

Backend Rust em `src-tauri/src/`, testes Rust inline (`#[cfg(test)]`) no próprio módulo,
fixtures em `tests/fixtures/` (raiz do repo — mesma convenção do `sample_fatura.xlsx`; as
tarefas abaixo dizem `src-tauri/tests/fixtures/`, corrigido na execução).
Frontend Vue/TS em `src/`.

## Status de verificação (2026-07-26)

O bloqueio de disco de 2026-07-25 caiu (57 GiB livres) e o crate Tauri completo compila.

| Verificação | Resultado |
|---|---|
| `cd src-tauri && cargo test` (crate completo) | **159 passaram, 0 falharam** (153 unit + 6 integração xlsx) |
| `cargo test banestes` | 18 passaram (17 de domínio + `import_folder::imports_banestes_statement_text_from_folder`) |
| `cargo test bank_statement` | 7 passaram — inclui `entry_id_key_format_is_frozen` e `dedup_id_is_stable` (regressão de id BTG) |
| `npx vue-tsc --noEmit` | verde |
| `npm run test:run` | 85 passaram (15 arquivos) |
| `infrastructure/banestes_statement.rs` contra o **PDF real** do usuário | 9 lançamentos, total `-7106.11`, conta `12/1234567-8`, titular correto, descrições limpas |
| Contracheque real (`contracheque_6_2026.pdf`) pelo mesmo leitor | rejeitado: "Este PDF não é um extrato do Banestes." |
| Validação manual no app rodando | **feita em 2026-07-26** — T048 (usuário validou importar/reimportar/pasta no app; "funcionou") |

## Revisão pós-tasks (2026-07-26): strategy por banco + classe de domínio

A pedido do usuário, a leitura de documentos virou **padrão strategy** e o domínio Banestes
virou classe (fora do escopo original das tasks acima; documentado em
`docs/ARCHITECTURE.md` §014 e nas receitas de `docs/MAINTENANCE.md`):

- `infrastructure/statement_reader.rs::StatementReader` (impls Banestes/BTG, registro
  `STATEMENT_READERS`) e `infrastructure/invoice_reader.rs::InvoiceReader` (impl BTG) —
  `commands/bank.rs`, `import_folder.rs` e `import_invoice.rs` só falam com o registro.
- Persistência genérica por banco: coluna `invoices.bank` (migração default `'BTG'`),
  `Invoice.bank` carimbado pelo strategy; `bank_entries.bank` já existia.
- `domain/banestes_statement.rs::ExtratoBanestes` — `parse` → `conferir` → `into_parsed`;
  `Conferencia`/`Checagem` explícitas; política **estrita** (`Divergiu` **e** `SemDados`
  recusam — antes `SemDados` passava em silêncio); consolidado confere por `Saldo Conta`
  (fixture nova `banestes_extrato_consolidado.txt`).
- Verificação: `cargo test` **162 + 6** verdes; `vue-tsc` verde; Vitest 85; PDF real via
  strategy → 9 lançamentos, `-7106.11`; contracheque real → `recognizes=false` (pulado).

---

## Phase 1: Setup

**Purpose**: fixture de teste e registro dos módulos novos. Nenhuma dependência nova a instalar
(`pdf-extract` já está no `Cargo.toml`).

- [X] T001 Criar fixture de texto anonimizada `src-tauri/tests/fixtures/banestes_extrato.txt` com a saída de `pdf_extract::extract_text` do extrato real de jul/2026 — substituir titular, contrapartes e agência/conta por valores fictícios, **preservando datas e valores** (9 lançamentos, saldo anterior 7.337,41, saldo final 231,30, entradas 0,00, saídas 7.106,11) e todas as particularidades: linha quebrada em duas, linhas `Saldo`, dia sem `JUL/26` repetido, lançamento do dia 20 com operação em 19/07
- [X] T002 [P] Criar variantes da fixture no mesmo diretório: `banestes_extrato_credito.txt` (1 lançamento sem sinal = entrada, mais um pagamento de fatura de cartão, um crédito de salário e uma transferência para o próprio titular, com saldos coerentes) e `banestes_extrato_quebrado.txt` (cópia do original com um valor alterado para a conferência falhar)
- [X] T003 [P] Registrar módulos vazios: `pub mod banestes_statement;` em `src-tauri/src/domain/mod.rs` e em `src-tauri/src/infrastructure/mod.rs`, criando `src-tauri/src/domain/banestes_statement.rs` e `src-tauri/src/infrastructure/banestes_statement.rs` com o doc-comment do módulo

**Checkpoint**: `cd src-tauri && cargo build` compila com os módulos vazios registrados.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: preparar o código compartilhado do extrato BTG para receber um segundo banco **sem
quebrar nada já gravado**. Bloqueia todas as user stories.

**⚠️ CRITICAL**: os ids de `bank_entries` já persistidos NÃO podem mudar (senão a próxima
importação BTG duplica tudo).

- [X] T004 Escrever teste vermelho em `src-tauri/src/domain/bank_statement.rs` que fixa o id atual de um lançamento BTG conhecido (valor literal do UUID gerado hoje por `entry_id`) — trava de regressão para as mudanças seguintes
- [X] T005 Escrever teste vermelho em `src-tauri/src/domain/bank_statement.rs`: dois `RawEntry` idênticos (mesma data, descrição e valor) na mesma lista devem produzir **ids diferentes**, e o primeiro deve manter o id do teste T004
- [X] T006 Implementar em `src-tauri/src/domain/bank_statement.rs` a atribuição de id por ocorrência (índice 0 mantém a chave `bank:{account}:{date}:{norm(desc)}:{amount}`; ocorrências repetidas ganham sufixo determinístico), mantendo `entry_id` como está para uso pontual — T004 e T005 verdes
- [X] T007 Escrever teste vermelho em `src-tauri/src/domain/bank_statement.rs`: lançamento com `transaction = "Pagamento Fatura Cartao"` (grafia Banestes, sem "do") é excluído com `reason = "fatura"`, e o texto BTG "Pagamento de fatura do cartão" continua excluído
- [X] T008 Ajustar `classify_entry` em `src-tauri/src/domain/bank_statement.rs` para detectar fatura por `FATURA` + `CART` (em `transaction` ou `description`) — T007 verde
- [X] T009 Adicionar campo `bank: String` (com `#[serde(default)]`) a `ParsedStatement` em `src-tauri/src/domain/bank_statement.rs` e expor os helpers `norm` e `parse_amount` como `pub(crate)` para reuso pelo parser Banestes
- [X] T010 Preencher `bank: "BTG"` em `src-tauri/src/infrastructure/btg_statement.rs::read_statement` e ajustar o teste existente de `parse_statement_rows` para o novo campo
- [X] T011 Rodar `cd src-tauri && cargo test bank_statement` — toda a suíte do extrato BTG verde antes de qualquer código Banestes

**Checkpoint**: base pronta; ids BTG preservados; `ParsedStatement` carrega o banco.

---

## Phase 3: User Story 1 — Importar o extrato do Banestes (Priority: P1) 🎯 MVP

**Goal**: selecionar o PDF do Banestes e ter os lançamentos (entradas e saídas) lidos, conferidos
e gravados, sem duplicar em reimportação.

**Independent Test**: com o PDF real, a prévia lista os 9 lançamentos de jul/2026 com valores
idênticos ao extrato; confirmar grava; reimportar grava 0.

### Testes (vermelhos primeiro)

- [X] T012 [P] [US1] Teste em `src-tauri/src/domain/banestes_statement.rs`: `is_banestes_statement` é `true` para a fixture `banestes_extrato.txt` e `false` para um texto de contracheque SouGov.br (usar as constantes de teste já existentes em `src-tauri/src/domain/payslip.rs` como referência de texto não-Banestes)
- [X] T013 [US1] Teste em `src-tauri/src/domain/banestes_statement.rs`: `parse_banestes_text` da fixture devolve `bank = "Banestes"`, `account = "12/1234567-8"` (agência/conta fictícias da fixture), `holder` sem o sufixo `Período: …` e **9** entradas/saídas
- [X] T014 [US1] Teste em `src-tauri/src/domain/banestes_statement.rs`: soma dos `amount` = `-7106.11`; nenhum `RawEntry` com descrição contendo `Saldo`; nenhum vindo do bloco de totais do topo nem do rodapé
- [X] T015 [US1] Teste em `src-tauri/src/domain/banestes_statement.rs`: o lançamento quebrado em duas linhas vira **um** registro com a descrição completa e `amount = -2729.78`
- [X] T016 [US1] Teste em `src-tauri/src/domain/banestes_statement.rs`: datas — todos os `month` são `2026-07`; o lançamento da linha do dia `20` tem `date = 2026-07-19` (data da operação vence o dia da coluna); um lançamento sem data de operação no texto cai para dia da coluna + mês do grupo
- [X] T017 [US1] Teste em `src-tauri/src/domain/banestes_statement.rs`: `transaction = "Pix Enviado"`, `description` = contraparte (sem data, sem número de documento), `btg_category` vazio
- [X] T018 [P] [US1] Teste em `src-tauri/src/domain/banestes_statement.rs`: fixture `banestes_extrato_credito.txt` — valor sem sinal vira `amount > 0` (entrada) e a conferência de entradas declaradas fecha
- [X] T019 [P] [US1] Teste em `src-tauri/src/domain/banestes_statement.rs`: fixture `banestes_extrato_quebrado.txt` devolve `Err` mencionando a diferença; texto sem cabeçalho de tabela devolve `Err` de formato; texto Banestes sem lançamentos devolve `Err` informativo

### Implementação

- [X] T020 [US1] Implementar `is_banestes_statement` em `src-tauri/src/domain/banestes_statement.rs` conforme [contracts/read_banestes_statement.md](contracts/read_banestes_statement.md) — T012 verde
- [X] T021 [US1] Implementar em `src-tauri/src/domain/banestes_statement.rs` a extração de metadados (agência, conta, titular cortado em `Período:`) e do bloco de totais do topo (saldo total, entradas, saídas) — T013 verde
- [X] T022 [US1] Implementar em `src-tauri/src/domain/banestes_statement.rs` o varredor de linhas: estado dia/mês, junção de linha de continuação, âncora de valor no fim da linha, descarte de linhas de saldo/cabeçalho/rodapé, montagem de `RawEntry` (tipo de operação, contraparte, data da operação com fallback, sinal) — T014 a T018 verdes
- [X] T023 [US1] Implementar em `src-tauri/src/domain/banestes_statement.rs` a conferência de integridade (saldo anterior + Σ = saldo final; Σ créditos = entradas declaradas; Σ |débitos| = saídas declaradas) devolvendo `Err` com a diferença em reais — T019 verde
- [X] T024 [US1] Implementar `read_statement` em `src-tauri/src/infrastructure/banestes_statement.rs`: `pdf_extract::extract_text` → erro claro se falhar ou vier sem texto útil → delega ao domínio
- [X] T025 [US1] Em `src-tauri/src/commands/bank.rs`, despachar por extensão em `classify_all` (`.pdf` → leitor Banestes, `.xls`/`.xlsx` → leitor BTG, outra → erro claro) e propagar `parsed.bank` para `BankEntry::from_classified` em `import_bank_statement`
- [X] T026 [US1] Em `src-tauri/src/commands/bank.rs`, adicionar `bank` a `StatementPreview` e o parâmetro `bank: String` a `save_bank_statement` (substituindo o literal `"BTG"`)
- [X] T027 [P] [US1] Em `src/types/api.types.ts`, acrescentar `bank` a `StatementPreview`; em `src/services/tauri.service.ts`, passar `bank` em `saveBankStatement`
- [X] T028 [US1] Em `src/pages/ExtratoPage.vue`, aceitar `pdf` no filtro do seletor de arquivo, enviar `preview.bank` no `saveBankStatement` e atualizar o texto de apoio para citar Banestes (.pdf) e BTG (.xls/.xlsx)
- [X] T029 [US1] Rodar `cd src-tauri && cargo test banestes && npx vue-tsc --noEmit` — verde

**Checkpoint**: US1 entregue — importar o PDF do Banestes funciona ponta a ponta (MVP).

---

## Phase 4: User Story 2 — Mesmas regras de exclusão e categorização do BTG (Priority: P1)

**Goal**: garantir, por teste, que o caminho do Banestes passa pelas **mesmas** regras do BTG:
fatura de cartão, salário com contracheque, transferência interna, categorização por palavra-chave
com "Outros" como sobra.

**Independent Test**: fixture com os três casos de exclusão → prévia mostra os três em "excluídos"
com o motivo correto.

- [X] T030 [US2] Teste em `src-tauri/src/domain/banestes_statement.rs` (integração de domínio): a fixture `banestes_extrato_credito.txt` passada por `parse_banestes_text` + `classify_statement` produz exclusão `fatura` para o pagamento de fatura de cartão
- [X] T031 [US2] Teste no mesmo arquivo: crédito de salário com o mês presente em `payslip_months` é excluído com `reason = "salario"`; sem contracheque no mês, é incluído como `kind = "income"`
- [X] T032 [US2] Teste no mesmo arquivo: transferência cuja contraparte é o titular do extrato é excluída com `reason = "interno"` (garante que `description` = contraparte pura, condição para o casamento por tokens do titular funcionar)
- [X] T033 [US2] Teste no mesmo arquivo: lançamento cuja contraparte casa com uma regra do `Categorizer` entra categorizado; sem regra, entra como `Outros` (sem fallback de categoria do banco, pois `btg_category` é vazio)
- [X] T034 [US2] Ajustar o parser/classificador conforme os testes T030–T033 exigirem (esperado: nenhum ajuste em `classify_entry` além do já feito em T008; se algum for necessário, manter compatível com os testes BTG de T011)

**Checkpoint**: US2 entregue — uma única lógica de exclusão/categorização para os dois bancos.

---

## Phase 5: User Story 3 — Duas contas convivendo (Priority: P2)

**Goal**: o usuário vê de qual banco veio cada lançamento e os totais somam as duas contas.

**Independent Test**: com extratos BTG e Banestes importados, a lista mostra o banco de cada
lançamento e Movimentações soma os dois.

- [X] T035 [P] [US3] Teste em `src-tauri/src/infrastructure/db.rs`: gravar `BankEntry` com `bank = "BTG"` e `bank = "Banestes"` na mesma conta-mês e verificar que `load_bank_entries` devolve os dois, sem colisão de id; e que gravar **duas vezes** o mesmo conjunto de lançamentos não aumenta a contagem (dedup de reimportação — SC-004)
- [X] T036 [US3] Em `src/pages/ExtratoPage.vue`, exibir banco e conta de cada lançamento na lista de importados (coluna/etiqueta) e no cabeçalho da prévia, mantendo remover/limpar/recategorizar
- [X] T037 [P] [US3] Revisar `src/pages/MovimentacoesPage.vue` e `src/pages/TransactionsPage.vue`: confirmar que agregam por `BankEntry` independente do banco (esperado: sim, já agregam por lançamento) e ajustar rótulos que digam "extrato do BTG" para o texto neutro
- [X] T038 [US3] Aplicar a skill `nielsen-heuristics` à tela de Extrato alterada (visibilidade do estado, prevenção de erro no seletor de arquivo, mensagens de erro em linguagem do usuário, reconhecimento em vez de memorização do banco) e corrigir o que a checagem apontar

**Checkpoint**: US3 entregue — dois bancos visíveis e somados corretamente.

---

## Phase 6: User Story 4 — Pasta de importação automática reconhece o Banestes (Priority: P3)

**Goal**: PDF de extrato Banestes na pasta configurada é importado sozinho; contracheque não.

**Independent Test**: pasta com extrato + contracheque → resumo informa 1 extrato; re-varrer não duplica.

- [X] T039 [US4] Extrair de `import_from_folder` uma função interna que recebe o **texto** já extraído (`&str`) + nome do arquivo e faz classificar/gravar, e escrever teste em `src-tauri/src/application/import_folder.rs` que passa a fixture `banestes_extrato.txt` por ela: 1 extrato, N lançamentos gravados (sem PDF binário no repositório — a chamada a `pdf_extract` fica na casca não testada, conforme research.md R7)
- [X] T040 [US4] Teste em `src-tauri/src/application/import_folder.rs`: `.pdf` que não é extrato Banestes **não** cria lançamento e é pulado **em silêncio**, fora de `ignored` (revisado em 2026-07-26 — antes reportava `NOT_RECOGNIZED`; contracheque mora legitimamente nessa pasta e a varredura roda a cada abertura do app, então o aviso repetido parecia erro); `.pdf` Banestes que não fecha a conferência entra em `ignored` com `ERROR: …`
- [X] T041 [US4] Teste em `src-tauri/src/application/import_folder.rs`: varrer duas vezes a mesma pasta não duplica lançamentos
- [X] T042 [US4] Implementar o ramo `Some("pdf")` em `import_from_folder` em `src-tauri/src/application/import_folder.rs`, reusando `try_import_extrato` com o leitor Banestes e o `bank` do `ParsedStatement` — T039–T041 verdes
- [X] T043 [P] [US4] Atualizar o texto da pasta de importação em `src/pages/SettingsPage.vue` para incluir extratos `.pdf` do Banestes

**Checkpoint**: US4 entregue — importação automática cobre o Banestes.

---

## Phase 7: Polish & Cross-Cutting

- [X] T044 Rodar a suíte completa: `cd src-tauri && cargo test`, `npx vue-tsc --noEmit`, `npm run test:run`
- [X] T045 [P] Atualizar `docs/ARCHITECTURE.md`: seção de extratos passa a listar dois leitores (BTG `.xls`/`.xlsx` via calamine, Banestes `.pdf` via pdf-extract) convergindo no mesmo domínio
- [X] T046 [P] Atualizar `docs/MAINTENANCE.md` com a receita "adicionar um novo banco" (leitor de infraestrutura + parser de domínio + despacho) e o invariante "a primeira ocorrência de um lançamento mantém o id antigo"
- [X] T047 [P] Atualizar `README.md`: features mencionam extrato Banestes (PDF)
- [X] T048 Validação manual pelo [quickstart.md](quickstart.md) com o PDF real do usuário (importar, reimportar, conferir totais contra o app do banco) — validada pelo usuário em 2026-07-26 (`npm run tauri dev`; migração `invoices.bank` confirmada no banco real: 10 faturas BTG, 80+9 bank_entries BTG/Banestes)
- [X] T049 Remover artefatos temporários de investigação (nenhum dentro do repo; conferir `git status` limpo além dos arquivos da feature) e garantir que nenhum extrato real foi adicionado ao repositório

---

## Dependencies

```text
Phase 1 (Setup: T001–T003)
   └─> Phase 2 (Foundational: T004–T011)   ← bloqueia todas as stories
          ├─> Phase 3 US1 (T012–T029)  🎯 MVP
          │      ├─> Phase 4 US2 (T030–T034)   depende do parser de US1
          │      ├─> Phase 5 US3 (T035–T038)   depende do `bank` propagado em US1
          │      └─> Phase 6 US4 (T039–T043)   depende do leitor de US1
          └─> Phase 7 (Polish: T044–T049)
```

- **US2, US3 e US4 são independentes entre si** — depois de US1, podem ser feitas em qualquer ordem.
- Dentro da Fase 2 a ordem é estrita: T004 → T005 → T006 (a trava de id precisa existir antes da
  mudança de id).

## Parallel Execution Examples

- **Setup**: T002 e T003 em paralelo (arquivos distintos), depois de T001.
- **US1 testes**: T012, T018 e T019 em paralelo (fixtures distintas); T013–T017 tocam o mesmo
  arquivo de teste, então em série.
- **US1 fim**: T027 (front types/service) em paralelo com T025/T026 (backend).
- **US3**: T035 (backend db) em paralelo com T037 (revisão de páginas).
- **Polish**: T045, T046 e T047 em paralelo.

## Implementation Strategy

**MVP = Fase 1 + Fase 2 + Fase 3 (US1)**: já entrega o valor central — o extrato Banestes entra no
app com entradas e saídas conferidas. As fases seguintes são incrementos verificáveis isoladamente.

Ordem recomendada de entrega: US1 → US2 (garante que os totais não inflam) → US3 (visibilidade dos
dois bancos) → US4 (conveniência) → Polish.

**Total**: 49 tarefas — Setup 3, Foundational 8, US1 18, US2 5, US3 4, US4 5, Polish 6.
