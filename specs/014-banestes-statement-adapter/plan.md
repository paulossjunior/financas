# Implementation Plan: Adapter de extrato Banestes (PDF)

**Branch**: `014-banestes-statement-adapter` | **Date**: 2026-07-25 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/014-banestes-statement-adapter/spec.md`

## Summary

Ler o extrato de conta corrente do Banestes (PDF) e transformá-lo em **entradas e saídas** que
entram no app pelo mesmo caminho do extrato BTG. A única parte nova é um **adapter exclusivo do
Banestes**: texto do PDF → `ParsedStatement`/`RawEntry` (as estruturas que o BTG já produz). Daí
para frente é código existente: `classify_statement` (excluir fatura/salário/transferência interna
+ categorizar pelas palavras-chave), `BankEntry` (gravação com dedup) e as telas atuais.

Nenhuma entidade nova, nenhuma tabela nova, nenhuma tela nova, nenhuma dependência nova
(`pdf-extract` já é usada pelo contracheque).

Antes de gravar, o adapter confere as **entradas e saídas** extraídas contra os totais que o próprio
extrato declara (e contra saldo anterior/final). Não fecha ⇒ erro explícito, nada gravado.

## Technical Context

**Language/Version**: Rust 1.75+ (backend), TypeScript 5.8 / Vue 3.5 (frontend)

**Primary Dependencies**: Tauri v2, `pdf-extract 0.7` (já presente), `regex`, `rust_decimal`,
rusqlite, `@tauri-apps/plugin-dialog` (seletor de arquivo), Pinia

**Storage**: SQLite `financas.db` — tabela `bank_entries` existente, **sem migração** (o campo
`bank` já existe e hoje é sempre `"BTG"`)

**Testing**: `cargo test` (parser puro com fixture de texto anonimizado), `npx vue-tsc --noEmit`,
`npm run test:run`

**Target Platform**: App desktop (macOS/Windows/Linux via Tauri)

**Performance Goals**: extrato pessoal (dezenas a centenas de linhas); leitura + prévia sem
percepção de espera (< 1 s)

**Constraints**: 100% offline; dinheiro em `Decimal` (nunca float); dedup determinístico; ids de
lançamentos BTG já gravados **não podem mudar**; falha de leitura nunca grava dado parcial

**Scale/Scope**: usuário único; 1 módulo de domínio novo, 1 módulo de infraestrutura novo, ajustes
pontuais em `commands/bank.rs`, `application/import_folder.rs`, `domain/bank_statement.rs` e 2
arquivos de front

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. TDD (NON-NEGOTIABLE)**: o parser Banestes é escrito teste-primeiro a partir da fixture de
  texto anonimizado — 9 lançamentos, linha quebrada, linhas de saldo, dia sem mês repetido, operação
  em dia diferente do lançamento, conferência de totais, entrada (crédito) sem sinal. Testes de
  regressão do BTG rodam antes e depois dos ajustes em `bank_statement.rs`. ✅ PASS
- **II. Clean Architecture**: `domain/banestes_statement.rs` é puro (texto → estruturas, zero I/O);
  `infrastructure/banestes_statement.rs` faz só PDF→texto e delega; `commands` só despacha;
  frontend só via `services/tauri.service.ts`. Espelha exatamente o par BTG existente. ✅ PASS
- **III. Simplicidade/YAGNI**: sem trait `StatementReader`, sem registry de bancos, sem
  configuração de layout — dois leitores concretos e um despacho por extensão. Helpers de
  normalização são **reusados** (`pub(crate)`), não duplicados. ✅ PASS
- **IV. Integridade de dados**: `Decimal` em tudo; estrutura validada antes de processar as linhas
  (cabeçalho da tabela obrigatório); conferência entradas/saídas + saldos antes de gravar; erro
  explícito em vez de default silencioso; dedup determinístico com ids BTG preservados. ✅ PASS
- **V. Local-First & Privacy**: só leitura de arquivo local; nenhuma rede; fixture de teste
  anonimizada (o extrato real, com nome/agência/conta e nomes de terceiros, não entra no repo). ✅ PASS

**Resultado**: PASS — sem violações. Re-checado após o desenho da Fase 1: sem mudanças.

## Project Structure

### Documentation (this feature)

```text
specs/014-banestes-statement-adapter/
├── plan.md                       # este arquivo
├── research.md                   # Fase 0 — gramática do extrato, extração de PDF, detecção
├── data-model.md                 # Fase 1 — reuso das entidades; único campo novo
├── quickstart.md                 # Fase 1 — como validar ponta a ponta
├── contracts/
│   ├── read_banestes_statement.md   # contrato do adapter (texto → ParsedStatement)
│   └── bank_statement_commands.md   # contratos Tauri afetados
├── checklists/requirements.md
└── tasks.md                      # Fase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
src-tauri/
├── src/domain/
│   ├── banestes_statement.rs     # NOVO (puro): texto do PDF → ParsedStatement + conferência
│   ├── bank_statement.rs         # + campo `bank` em ParsedStatement; norm/parse_amount pub(crate);
│   │                             #   fatura de cartão = FATURA+CART; entry_id com índice de ocorrência
│   └── mod.rs                    # + pub mod banestes_statement
├── src/infrastructure/
│   ├── banestes_statement.rs     # NOVO (I/O): pdf_extract::extract_text → domínio
│   └── mod.rs                    # + pub mod banestes_statement
├── src/commands/bank.rs          # despacho por extensão (.pdf → Banestes); bank em preview/save
└── src/application/import_folder.rs  # ramo .pdf na varredura da pasta

tests/fixtures/                    # raiz do repo (convenção do sample_fatura.xlsx)
├── banestes_extrato.txt           # NOVO: texto anonimizado do extrato real (jul/2026)
├── banestes_extrato_credito.txt   # NOVO: entrada, fatura, salário, transferência interna, tarifa
└── banestes_extrato_quebrado.txt  # NOVO: valor alterado → conferência falha

src/
├── pages/ExtratoPage.vue         # aceitar .pdf no seletor; mostrar banco/conta; textos
├── services/tauri.service.ts     # (sem assinatura nova — só tipos)
└── types/api.types.ts            # + bank em StatementPreview / ParsedStatement
```

**Structure Decision**: reuso da estrutura em camadas atual. O adapter é dividido em
domínio (gramática do extrato, testável sem arquivo) + infraestrutura (PDF→texto), igual ao par
`domain/bank_statement.rs` + `infrastructure/btg_statement.rs` que já existe para o BTG.

## Phase 2 — Ordem de implementação (resumo; detalhe em tasks.md)

1. Fixture de texto anonimizada + testes vermelhos do parser Banestes (domínio).
2. Parser Banestes até verde: metadados, dia/mês, continuação, saldos, sinal, conferência.
3. Ajustes de reuso em `domain/bank_statement.rs` (campo `bank`, helpers, fatura+cart, entry_id) com
   testes de regressão do BTG.
4. Leitor de infraestrutura (PDF → texto → domínio) + detecção "é Banestes?".
5. Despacho por extensão em `commands/bank.rs`; `bank` propagado até `BankEntry`.
6. Ramo `.pdf` em `import_folder.rs` (contracheque não vira extrato).
7. Front: seletor aceita `.pdf`, coluna de banco na lista, textos revisados pelas heurísticas de
   usabilidade (`nielsen-heuristics`).

## Complexity Tracking

> Sem violações constitucionais. Tabela não aplicável.
