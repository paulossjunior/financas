# Data Model — Adapter de fatura Santander (PDF)

**Feature**: 015-santander-invoice-adapter · **Date**: 2026-07-26

## Entidades novas (domínio, internas)

### `FaturaSantander` (`domain/santander_invoice.rs`)

Retrato tipado de uma fatura Santander — o que o PDF declara, antes de virar
`Invoice`/`Transaction`. Molde de `ExtratoBanestes` (014).

| Campo | Tipo | Origem no PDF | Regra |
|---|---|---|---|
| `titular` | `String` | `FULANO ... - 4220 XXXX XXXX 1234` (primeira subseção) | informativo |
| `vencimento` | `Option<NaiveDate>` | `Vencimento dd/mm/yyyy` | fallback do mês de referência (R10) |
| `compras` | `Vec<Compra>` | blocos "Despesas" de todas as subseções de cartão | inclui IOFs como itens próprios (R3) |
| `creditos` | `Vec<Compra>` | bloco "Pagamento e Demais Créditos", exceto pagamentos | cashback etc.; valor negativo |
| `pagamentos_excluidos` | `Decimal` | Σ dos `PAGAMENTO DE FATURA`/`DEB AUTOM` | não viram transação; entram na conferência (R4/R7) |
| `resumo` | `Option<ResumoFatura>` | bloco "Resumo da Fatura" | `None` ⇒ conferência `SemDados` ⇒ recusa |

### `Compra` (interno ao domínio)

| Campo | Tipo | Regra |
|---|---|---|
| `date` | `NaiveDate` | `dd/mm` + ano inferido (R5) |
| `description` | `String` | texto entre a data e o valor; sub-linhas de cotação descartadas; IOF vira `IOF — <compra>` |
| `amount` | `Decimal` | R$ impresso (US$ ignorado); negativo = crédito |
| `installment` | `Option<InstallmentInfo>` | melhor esforço sobre a coluna "Parcela" (sem amostra real; conferência protege) |

### `ResumoFatura`

| Campo | Tipo | Linha do PDF |
|---|---|---|
| `saldo_anterior` | `Decimal` | `Saldo Anterior` |
| `despesas_brasil` | `Decimal` | `(+) Total Despesas/Débitos no Brasil` |
| `despesas_exterior` | `Decimal` | `(+) Total Despesas/Débitos no Exterior` (coluna R$) |
| `pagamentos` | `Decimal` | `(-) Total de pagamentos` |
| `creditos` | `Decimal` | `(-) Total de créditos` |
| `saldo_fatura` | `Decimal` | `(=) Saldo Desta Fatura` |

### `Conferencia` / `Checagem`

Reuso conceitual da 014 (tipos próprios do módulo, mesma semântica:
`Fechou | Divergiu{diferenca} | SemDados{faltou}` e política estrita `exigir()`):

- `despesas`: `Σ compras (>0) == despesas_brasil + despesas_exterior`
- `creditos_pagamentos`: `Σ |créditos lidos| + pagamentos_excluidos == creditos + pagamentos`
- validação interna do resumo: `saldo_anterior + despesas − pagamentos − creditos == saldo_fatura`

## Entidades existentes (reuso, sem campos novos)

| Entidade | Uso nesta feature |
|---|---|
| `Invoice` | `bank = "Santander"` (campo da 014); `id = UUIDv5(filename)`; `reference_month` do R10 |
| `Transaction` | uma por compra/IOF/crédito; `is_reversal` automático para valor negativo (cashback); `id = UUIDv5(invoice_id, row_index sequencial)` (R6) |
| `InvoiceInfo` (DTO) | já carrega `bank` — lista de faturas mostra "Santander" sem mudança de tipo |
| Senha (keychain) | nova credencial `invoice-password-santander`; a BTG (`invoice-password`) intocada (R9) |

## Fluxo de estados

```text
PDF ─extract_text[_encrypted]→ texto
  ─is_santander_invoice?─ não → Err("não é fatura Santander")
  ─FaturaSantander::parse→ struct tipada
  ─conferir().exigir()─ Divergiu/SemDados → Err(diferença/o que faltou) — nada gravado
  ─into transactions→ (Vec<Transaction>, warnings)  [categorização pelas regras do app]
  → import_invoice (fluxo BTG inalterado): Invoice{bank:"Santander"} → store.add (dedup) → SQLite
```

## Invariantes

1. Nenhuma transação com `amount == 0` (FR-006).
2. `Σ transactions` de uma fatura importada `== saldo_fatura − saldo_anterior + pagamentos`
   (consequência da conferência; o dashboard soma exatamente o gasto do mês).
3. Reimportação do mesmo arquivo substitui — nunca duplica (identidade por filename).
4. Nenhum literal `"Santander"` fora do strategy/domínio próprio (o banco viaja em
   `reader.bank()` → `Invoice.bank`).
