# Implementation Plan: Importar faturas de cartão Santander (PDF)

**Branch**: `015-santander-invoice-adapter` | **Date**: 2026-07-26 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/015-santander-invoice-adapter/spec.md`

## Summary

Segundo banco de **fatura** do app (o BTG veio em `.xlsx`; o Santander vem em **PDF
cifrado**, senha = CPF do titular). A feature 014 já deixou a costura pronta: o leitor entra
como nova strategy `InvoiceReader` registrada em `INVOICE_READERS`, o domínio ganha uma
classe `FaturaSantander` no molde de `ExtratoBanestes` (`parse` → `conferir` → transações), e
a persistência já é genérica por banco (`Invoice.bank`). Conferência obrigatória contra o
"Resumo da Fatura" impresso no PDF — não fechou, nada grava. Senha ganha chave própria no
keychain (`invoice-password-santander`), reusando o fluxo de prompt/remember do BTG.

## Technical Context

**Language/Version**: Rust 1.75+ (backend Tauri v2), TypeScript 5 + Vue 3 (frontend)

**Primary Dependencies**: `pdf_extract` 0.7 (`extract_text_encrypted` — já no Cargo.toml,
usada pelo contracheque e pelo extrato Banestes), `rust_decimal`, `regex`, `keyring` 3
(keychain), `uuid` v5. Zero dependências novas.

**Storage**: SQLite `financas.db` (tabelas `invoices`/`transactions`, já genéricas por banco
desde a 014); keychain do SO para a senha.

**Testing**: `cargo test` (testes inline `#[cfg(test)]` no domínio + fixtures de texto
anonimizado em `tests/fixtures/`), Vitest para o front, `vue-tsc`.

**Target Platform**: desktop macOS/Windows/Linux (Tauri v2), 100% local.

**Project Type**: desktop-app (backend `commands → application → domain`, `infrastructure`
para I/O).

**Performance Goals**: importar uma fatura (~650 KB, 3 páginas) em < 2 s incluindo
decifragem; varredura da pasta com 20 arquivos em < 10 s. (Não são gargalos: pdf_extract já
processa os PDFs reais em centenas de ms.)

**Constraints**: nenhum PDF real no repositório (dados pessoais); senha nunca em plaintext
fora do keychain; aritmética monetária só com `Decimal`; importação atômica (confere antes
de gravar).

**Scale/Scope**: 4 faturas reais hoje (2 cartões), ~12/ano daqui em diante; PDFs de 2–4
páginas, ≤ 60 transações por fatura.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Princípio | Como o plano cumpre |
|---|---|
| I. TDD (NON-NEGOTIABLE) | Toda tarefa de implementação em tasks.md é precedida por teste vermelho; parser coberto por fixtures de texto real anonimizado (≥ 90%). |
| II. Clean Architecture | Gramática e conferência em `domain/santander_invoice.rs` (puro, sem I/O); PDF/decifragem em `infrastructure/santander_invoice.rs`; despacho pelo registro `INVOICE_READERS` (strategy da 014 — padrão justificado por problema concreto: 2º banco de fatura). |
| III. Simplicidade/YAGNI | Nenhuma abstração nova além da strategy existente; parcelamento tratado como melhor esforço (não há amostra real) protegido pela conferência; nada de suporte a bancos hipotéticos. |
| IV. Data Integrity | `Decimal` em tudo; conferência obrigatória (Resumo da Fatura) aborta a importação com a diferença em R$; nenhuma linha é descartada em silêncio — ou é regra explícita (cotação, pagamento de fatura, valor 0,00) ou quebra a conferência; cashback (crédito) vira `is_reversal` explícito. |
| V. Local-First & Privacy | pdf_extract local; senha só no keychain (chave própria por banco); fixtures anonimizadas; nenhum PDF real no repo (regra "Real files" do CLAUDE.md). |

**Gate inicial**: PASS. **Re-check pós-design (Phase 1)**: PASS — nenhum desvio; Complexity
Tracking vazio.

## Project Structure

### Documentation (this feature)

```text
specs/015-santander-invoice-adapter/
├── plan.md              # este arquivo (/speckit-plan)
├── research.md          # Phase 0
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1
├── contracts/           # Phase 1
│   ├── read_santander_invoice.md
│   └── password_per_bank.md
└── tasks.md             # Phase 2 (/speckit-tasks — não criado pelo plan)
```

### Source Code (repository root)

```text
src-tauri/src/
├── domain/
│   ├── santander_invoice.rs        # NOVO: FaturaSantander (parse → conferir → transações), puro
│   └── mod.rs                      # registra o módulo novo
├── infrastructure/
│   ├── santander_invoice.rs        # NOVO: PDF → texto (com senha) + SantanderInvoiceReader (strategy)
│   ├── invoice_reader.rs           # registra SantanderInvoiceReader; senha por leitura já flui pelo trait
│   └── secrets.rs                  # chave de senha por banco (invoice-password / invoice-password-santander)
├── application/
│   ├── import_invoice.rs           # fluxo intocado (strategy resolve o banco); erros novos mapeados
│   └── import_folder.rs            # ramo .pdf: extrato Banestes → fatura Santander (senha salva) → silêncio
└── commands/
    └── import.rs                   # senha efetiva por banco do arquivo; remember grava na chave certa

src/
└── components/dashboard/ImportButton.vue + textos   # seletor ganha .pdf; mensagens citam Santander

tests/fixtures/
├── santander_fatura.txt            # NOVO: texto anonimizado da fatura real jul/2026
├── santander_fatura_cashback.txt   # NOVO: pagamentos + cashback + internacional/IOF + multi-cartão
└── santander_fatura_quebrada.txt   # NOVO: um valor adulterado (conferência tem de falhar)
```

**Structure Decision**: mesmo desenho da 014 — classe pura no domínio, casca de I/O na
infraestrutura implementando a strategy, despacho só pelo registro. Nenhum diretório novo.

## Complexity Tracking

Sem violações da Constituição — tabela vazia.
