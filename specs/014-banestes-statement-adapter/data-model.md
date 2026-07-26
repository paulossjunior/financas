# Phase 1 — Data Model: adapter de extrato Banestes

**Feature**: 014-banestes-statement-adapter

O pedido do usuário é explícito: **nenhuma entidade nova**. Este documento registra o que é
reusado como está, o que muda de forma mínima e por quê.

## Entidades reusadas (sem alteração de significado)

### `RawEntry` — linha crua do extrato (`domain/bank_statement.rs`)

| Campo | Tipo | Como o adapter Banestes preenche |
|---|---|---|
| `date` | `String` `YYYY-MM-DD` | data da **operação** impressa no lançamento; fallback = dia da coluna + mês/ano do grupo |
| `month` | `String` `YYYY-MM` | derivado de `date` |
| `btg_category` | `String` | **vazio** — o Banestes não classifica lançamentos (sem fallback de categoria do banco) |
| `transaction` | `String` | tipo de operação, ex. `Pix Enviado` |
| `description` | `String` | contraparte, ex. `GIGA MAIS FIBRA` |
| `amount` | `Decimal` | valor com sinal: **negativo = saída**, positivo = **entrada** |

> `btg_category` fica vazio em vez de ganhar campo novo: o nome é histórico ("categoria informada
> pelo banco"), e o `classify_entry` já trata vazio como "sem fallback".

### `ClassifiedEntry`, `BankEntry`, `ManualEntry`

Inalteradas. O Banestes passa pelo **mesmo** `classify_statement` → `BankEntry::from_classified`
→ `to_manual_entry`. `BankEntry.bank` recebe `"Banestes"`; `BankEntry.account` recebe
`"<agência>/<conta>"` (ex. `12/1234567-8`), que distingue contas de bancos diferentes na lista.

### Tabela `bank_entries` (SQLite)

**Sem migração.** As colunas `bank` e `account` já existem; hoje `bank` é sempre `"BTG"`.

## Alteração mínima em entidade existente

### `ParsedStatement` ganha `bank`

```text
ParsedStatement { bank: String, holder: String, account: String, entries: Vec<RawEntry> }
```

- **Por quê**: hoje o nome do banco é literal `"BTG"` em três chamadas
  (`commands/bank.rs` ×2, `application/import_folder.rs` ×1). Com dois leitores, o banco tem de
  viajar junto do que foi lido, senão o chamador precisa adivinhar pela extensão em cada ponto.
- **Compatibilidade**: `#[serde(default)]`; o leitor BTG preenche `"BTG"`, o Banestes `"Banestes"`.
- **Não é entidade nova**: é um campo de identificação de origem numa struct existente.

## Regras de validação (aplicadas antes de qualquer gravação)

| Regra | Falha ⇒ |
|---|---|
| Texto contém `Extrato de Conta Corrente` **e** (`Saldo Anterior` **ou** `Agência:`) | "não é um extrato Banestes" (na varredura de pasta: arquivo apenas ignorado) |
| Cabeçalho da tabela (`Data` … `Lançamento` … `Valor`) presente | "formato de extrato não reconhecido" |
| `saldo_anterior + Σ amount == saldo_final` | "a leitura do extrato não fecha com os saldos" — nada gravado |
| `Σ créditos == entradas declaradas` e `Σ |débitos| == saídas declaradas` (quando os totais do topo são lidos) | idem |
| Pelo menos 1 lançamento | "nenhum lançamento no período" (informativo, não erro) |

## Identidade e deduplicação

`entry_id(account, entry)` continua sendo `UUIDv5(OID, "bank:{account}:{date}:{norm(desc)}:{amount}")`
para a **primeira** ocorrência de uma combinação — assim **todo id BTG já gravado continua igual**.
Ocorrências repetidas da mesma combinação no mesmo arquivo recebem sufixo de ocorrência
(`:#1`, `:#2`, …) na chave.

- **Problema resolvido**: dois pix idênticos no mesmo dia para a mesma contraparte hoje colapsam em
  um único registro — dinheiro subcontado, silenciosamente.
- **Por que não usar o número do documento do Banestes na identidade**: ele não existe no BTG, e
  colocá-lo na descrição sujaria lista, fila de categorização e relatórios (ver research.md R5).

## Fluxo dos dados

```text
PDF (arquivo local)
  └─ infrastructure/banestes_statement.rs   pdf_extract::extract_text → String
       └─ domain/banestes_statement.rs      String → ParsedStatement{bank:"Banestes",…} + conferência
            └─ domain/bank_statement.rs     classify_statement  (fatura / salário / interno / categoria)
                 └─ BankEntry::from_classified(bank, account) → bank_entries (dedup por id)
                      └─ to_manual_entry() → painel, movimentações, despesas & receitas, recorrentes
```
