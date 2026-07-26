---

description: "Task list — adapter de fatura Santander (PDF)"
---

# Tasks: Importar faturas de cartão Santander (PDF)

**Input**: Design documents from `/specs/015-santander-invoice-adapter/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/)

**Tests**: OBRIGATÓRIOS. Constituição (Princípio I, NON-NEGOTIABLE): teste vermelho antes de
implementação, ≥ 90% de cobertura no parser. Toda tarefa de implementação abaixo é precedida
pelo seu teste.

**Organization**: agrupado por user story, na ordem de prioridade do spec.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência pendente)
- **[Story]**: user story a que pertence (US1…US4)

## Path Conventions

Backend Rust em `src-tauri/src/`, testes Rust inline (`#[cfg(test)]`) no próprio módulo,
fixtures em `tests/fixtures/` (raiz do repo). Frontend Vue/TS em `src/`. Arquivos reais só
em `~/Documents/casa/faturas/` (regra "Real files" do CLAUDE.md) — investigação com example
temporário apagado no mesmo turno.

---

## Phase 1: Setup

**Purpose**: fixtures de texto anonimizadas e módulos registrados. Nenhuma dependência nova.

- [X] T001 Criar fixture `tests/fixtures/santander_fatura.txt` a partir da saída real de `pdf_extract` da fatura jul/2026 — titular/CPF/cartões/endereço fictícios, **valores e datas preservados** (compras 29/05–29/06, internacionais com COTAÇÃO DOLAR + IOF, pagamentos de fatura, DESCONTO DO MES, ANUIDADE 0,00, multi-cartão físico+virtual, bloco "Resumo da Fatura" fechando: 4.923,40 + 8.255,19 + 10.783,10 − 21.005,57 − 149,30 = 2.806,82)
- [X] T002 [P] Criar variantes: `tests/fixtures/santander_fatura_cashback.txt` (recorte com pagamento + cashback + 1 internacional + 2 cartões, resumo coerente) e `tests/fixtures/santander_fatura_quebrada.txt` (cópia da principal com um valor de compra adulterado — conferência tem de acusar a diferença exata)
- [X] T003 [P] Registrar módulos vazios com doc-comment: `pub mod santander_invoice;` em `src-tauri/src/domain/mod.rs` e `src-tauri/src/infrastructure/mod.rs`, criando os dois arquivos

**Checkpoint**: `cd src-tauri && cargo build` compila com os módulos vazios.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: senha por banco no keychain — bloqueia a US1 (importar pede senha) e a US4
(pasta usa senha salva). Contrato: [contracts/password_per_bank.md](contracts/password_per_bank.md).

- [X] T004 Escrever testes vermelhos em `src-tauri/src/infrastructure/secrets.rs` (mock keychain já existente): `save/get/clear/has_password_for("Santander")` roundtrip; `get_password_for("BTG")` lê a credencial legada `invoice-password` (senha já salva do usuário continua valendo); credenciais dos dois bancos não colidem
- [X] T005 Implementar em `src-tauri/src/infrastructure/secrets.rs` as funções por banco (`entry_for(bank)` — "BTG" → USER legado `invoice-password`; outros → `invoice-password-<slug minúsculo>`), mantendo as funções antigas como atalhos BTG — T004 verde
- [X] T006 Atualizar os comandos Tauri de senha em `src-tauri/src/commands/` (grep por `secrets::`) para o parâmetro `bank: Option<String>` com default "BTG" — front atual continua funcionando sem mudança; `cargo test secrets` verde

**Checkpoint**: keychain por banco pronto; comportamento BTG intocado.

---

## Phase 3: User Story 1 — Importar a fatura Santander (Priority: P1) 🎯 MVP

**Goal**: selecionar o PDF, informar a senha uma vez, e as compras entram categorizadas com
banco "Santander", somando no dashboard; reimportar substitui.

**Independent Test**: com a fatura real e a senha, importar → transações batem com o PDF;
reimportar → 0 duplicatas; as 4 faturas reais importam sem pedir senha de novo.

### Testes (vermelhos primeiro — `src-tauri/src/domain/santander_invoice.rs`)

- [X] T007 [P] [US1] Teste: `is_santander_invoice` true para a fixture principal; false para texto de contracheque SouGov e para a fixture `banestes_extrato.txt`
- [X] T008 [US1] Teste: `FaturaSantander::parse` da fixture principal — compras de **todas** as subseções de cartão presentes; nenhuma linha `COTAÇÃO DOLAR`/`VALOR TOTAL`/cabeçalho vira compra; nenhuma transação com valor 0,00 (ANUIDADE isenta fora)
- [X] T009 [US1] Teste: compra internacional entra pelo R$ impresso (`7.019,45`, não o US$) e gera par `IOF — ANTHROPIC* TEAM T1` de `245,68` com a mesma data (research R3)
- [X] T010 [US1] Teste: `PAGAMENTO DE FATURA-INTERNET` e `DEB AUTOM DE FATURA` não geram transação e acumulam em `pagamentos_excluidos`; `DESCONTO DO MES` vira transação negativa (`is_reversal` no `Transaction`)
- [X] T011 [US1] Teste: datas — compras `dd/mm` ganham o ano do mês de referência; caso dez→jan (fixture cashback com vencimento em janeiro e compra em dezembro) cai no ano anterior (research R5)
- [X] T012 [P] [US1] Teste: `reference_month` — `Fatura_072026_..._SANTANDER.PDF` → 2026-07; nome fora do padrão → mês/ano do `Vencimento` impresso (research R10)
- [X] T013 [US1] Teste: `into_transactions` — ids determinísticos (mesma fixture duas vezes → mesmos ids, ordem do PDF), categorização pelas regras do app (uma compra casando keyword; sobra em "Outros")

### Implementação (domínio)

- [X] T014 [US1] Implementar `is_santander_invoice` + esqueleto `FaturaSantander`/`Compra`/`ResumoFatura` em `src-tauri/src/domain/santander_invoice.rs` conforme [data-model.md](data-model.md) — T007 verde
- [X] T015 [US1] Implementar o varredor de linhas de `FaturaSantander::parse`: estado de subseção/bloco, âncora de valor no fim da linha, prefixo numérico da coluna "Compra" ignorado, junção de descrição quebrada, descarte explícito de cotação/cabeçalhos/VALOR TOTAL, IOF associado à compra anterior, pagamentos acumulados, extração do `ResumoFatura` e do `Vencimento` — T008–T012 verdes
- [X] T016 [US1] Implementar `reference_month` e `into_transactions` (row_index sequencial, `Transaction::new`, categorizer, parcela melhor-esforço da coluna "Parcela") — T013 verde

### Implementação (infra + fluxo)

- [X] T017 [US1] Implementar `src-tauri/src/infrastructure/santander_invoice.rs`: `extract_text(path, password)` mapeando cifrado-sem-senha → `Encrypted`, senha errada → `WrongPassword` (mensagens do pdf_extract, research R1); `is_encrypted_pdf`; `SantanderInvoiceReader` (strategy, `bank() = "Santander"`, `extensions() = ["pdf"]`) com pipeline extract → detectar → parse → **conferir stub temporário** → into_transactions, conforme [contracts/read_santander_invoice.md](contracts/read_santander_invoice.md)
- [X] T018 [US1] Registrar em `INVOICE_READERS` (`src-tauri/src/infrastructure/invoice_reader.rs`) e estender o teste de despacho: `.pdf` → Santander, `.xlsx` → BTG
- [X] T019 [US1] Resolver o mês de referência não-BTG: `import_invoice` usa o mês informado pelo leitor quando o nome não casa `YYYY-MM-…` (menor atrito escolhido na hora; comportamento BTG intocado) em `src-tauri/src/application/import_invoice.rs`, com teste
- [X] T020 [US1] Senha efetiva por banco em `src-tauri/src/commands/import.rs`: banco do arquivo via `invoice_reader_for`, `get_password_for(banco)` como fallback, `remember` grava com `save_password_for(banco, …)` — senha que falhou nunca é salva
- [X] T021 [P] [US1] Front: seletor de faturas aceita `.pdf` (grep pelo filtro xlsx em `src/components/` / `src/pages/`), textos citam Santander; prompt de senha existente reusado sem mudança de fluxo
- [X] T022 [US1] Rodar `cd src-tauri && cargo test santander && npx vue-tsc --noEmit` — verde

**Checkpoint**: US1 entregue — fatura Santander importa ponta a ponta (conferência ainda stub).

---

## Phase 4: User Story 2 — A fatura confere ou não entra (Priority: P1)

**Goal**: soma lida fecha com o "Resumo da Fatura" ou nada é gravado, com a diferença em R$.

**Independent Test**: fixture íntegra importa; adulterada recusa citando a diferença; sem
resumo recusa dizendo o que faltou.

- [X] T023 [P] [US2] Testes vermelhos em `src-tauri/src/domain/santander_invoice.rs`: `conferir` devolve `Fechou`+`Fechou` na fixture principal e na cashback (Σ despesas == Brasil+Exterior; Σ|créditos|+pagamentos == declarados; identidade do resumo fecha)
- [X] T024 [P] [US2] Testes vermelhos: fixture quebrada → `Divergiu` com a diferença exata e `parse_…`/reader → `Err` contendo a diferença em R$ e "Nada foi importado"; fixture sem o bloco "Resumo da Fatura" → `SemDados` → `Err` dizendo o que faltou
- [X] T025 [US2] Implementar `Conferencia`/`Checagem`/`exigir()` (política estrita da 014) em `src-tauri/src/domain/santander_invoice.rs` e trocar o stub do reader pela conferência real — T023/T024 verdes; T022 continua verde

**Checkpoint**: US2 entregue — nenhuma fatura entra sem fechar com o PDF.

---

## Phase 5: User Story 3 — Senha errada, PDF alheio, regressão BTG (Priority: P2)

**Goal**: erros com mensagens claras; BTG byte-a-byte igual.

**Independent Test**: senha errada → `WRONG_PASSWORD` sem salvar; contracheque/extrato →
recusa clara; suíte BTG 100% verde.

- [X] T026 [P] [US3] Teste em `src-tauri/src/infrastructure/santander_invoice.rs`: PDF inexistente → erro claro; arquivo não-PDF com extensão .pdf → `InvalidFormat`/mensagem pt-BR (sem panic)
- [X] T027 [US3] Teste no domínio: `FaturaSantander::parse` de texto de contracheque e de extrato Banestes → `Err` "não é uma fatura do Santander"; garantir que o reader propaga como `InvalidFormat` (mensagem chega ao usuário via mapError — tem acento)
- [X] T028 [US3] Teste de regressão: `cargo test btg` + teste explícito de que `invoice_reader_for("fatura.xlsx").bank() == "BTG"` e que `import_invoice` de `sample_fatura.xlsx` produz o mesmo `ImportResult` de antes (ids inalterados)
- [X] T029 [US3] Ajustes que os testes T026–T028 exigirem (esperado: nenhum além de mensagens); rodar `cargo test` completo

**Checkpoint**: US3 entregue — caminhos de erro sólidos, zero regressão.

---

## Phase 6: User Story 4 — Pasta de importação automática (Priority: P3)

**Goal**: fatura Santander na pasta importa sozinha com a senha salva; sem senha vira
`ignored` com motivo; extrato/contracheque seguem seus fluxos.

**Independent Test**: pasta com os 3 tipos de PDF → 1 fatura + 1 extrato + contracheque em
silêncio; sem senha → `ENCRYPTED_NO_PASSWORD`; re-varrer não duplica.

- [X] T030 [P] [US4] Testes vermelhos em `src-tauri/src/application/import_folder.rs` (seam de texto, como na 014): fatura Santander (fixture) com senha salva → 1 fatura importada, N transações; sem senha salva + PDF cifrado → `ignored` com `ENCRYPTED_NO_PASSWORD`; contracheque aberto → silêncio; extrato Banestes → extrato (ordem do ramo conforme [contracts/read_santander_invoice.md](contracts/read_santander_invoice.md))
- [X] T031 [US4] Teste: varrer duas vezes → fatura substituída, não duplicada (contagem estável)
- [X] T032 [US4] Implementar o ramo `.pdf` em `import_from_folder`: extrato (recognizes) → senão `is_encrypted_pdf` → fatura Santander via senha salva → senão silêncio — T030/T031 verdes
- [X] T033 [P] [US4] Atualizar o texto da pasta em `src/pages/SettingsPage.vue` (faturas Santander .pdf entram; senha salva necessária) e a entrada de senha por banco nas Configurações (Santander ao lado da BTG), aplicando a skill `nielsen-heuristics` à tela alterada

**Checkpoint**: US4 entregue — pasta cobre os dois bancos de fatura.

---

## Phase 7: Polish & Cross-Cutting

- [X] T034 Rodar a suíte completa: `cd src-tauri && cargo test`, `npx vue-tsc --noEmit`, `npm run test:run`
- [X] T035 Validar contra os **4 PDFs reais** (example temporário via strategy, apagado no mesmo turno; senha nunca em arquivo): 4 importam, totais = "Saldo Desta Fatura" de cada uma; contracheque e extrato reais recusados pelo fluxo de fatura
- [X] T036 [P] Atualizar `docs/ARCHITECTURE.md` (seção 015: segundo InvoiceReader, senha por banco) e `docs/MAINTENANCE.md` (receita "novo banco de fatura" ganha o exemplo Santander + chave de senha)
- [X] T037 [P] Atualizar `README.md`: features citam fatura Santander (PDF cifrado)
- [ ] T038 Validação manual pelo [quickstart.md](quickstart.md) no app rodando (US1 passos 1–5, US3 7–9, US4 10–12)
- [X] T039 Conferir `git status`: nenhum PDF real, nenhuma senha em arquivo; fixtures só anonimizadas

---

## Dependencies

```text
Phase 1 (Setup: T001–T003)
   └─> Phase 2 (Foundational: T004–T006)   ← senha por banco
          └─> Phase 3 US1 (T007–T022)  🎯 MVP
                 ├─> Phase 4 US2 (T023–T025)   conferência real substitui o stub
                 ├─> Phase 5 US3 (T026–T029)   erros/regressão sobre o leitor pronto
                 └─> Phase 6 US4 (T030–T033)   pasta usa leitor + senha salva
                        └─> Phase 7 (Polish: T034–T039)
```

- US2, US3 e US4 são independentes entre si depois da US1.
- Dentro da US1: T014 → T015 → T016 (mesmo arquivo, estado incremental); T017 → T018;
  T019/T020 depois de T018; T021 a qualquer momento.

## Parallel Execution Examples

- **Setup**: T002 e T003 em paralelo após T001.
- **US1 testes**: T007 e T012 em paralelo; T008–T011/T013 tocam o mesmo arquivo (série).
- **US1 fim**: T021 (front) em paralelo com T017–T020 (backend).
- **US2**: T023 e T024 em paralelo (fixtures distintas).
- **Polish**: T036 e T037 em paralelo.

## Implementation Strategy

**MVP = Fases 1+2+3 (US1)**: fatura Santander entrando com senha lembrada já é o valor
central. US2 fecha a rede de integridade (antes de qualquer uso sério), US3 endurece erros,
US4 é conveniência. Ordem recomendada: US1 → US2 → US3 → US4 → Polish.

**Total**: 39 tarefas — Setup 3, Foundational 3, US1 16, US2 3, US3 4, US4 4, Polish 6.
