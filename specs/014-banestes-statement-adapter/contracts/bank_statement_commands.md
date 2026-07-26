# Contract: comandos Tauri afetados

**Feature**: 014-banestes-statement-adapter

Nenhum comando novo. As assinaturas continuam iguais; o que muda é o despacho por tipo de arquivo
e o campo `bank` viajando até a UI.

---

## `preview_bank_statement(path: String) -> StatementPreview`

**Mudança**: despacho por extensão do arquivo.

| Extensão | Leitor |
|---|---|
| `.pdf` | `infrastructure::banestes_statement::read_statement` |
| `.xls` / `.xlsx` | `infrastructure::btg_statement::read_statement` (atual) |
| outra | `Err("Formato não suportado. Use o extrato .pdf (Banestes) ou .xls/.xlsx (BTG).")` |

**`StatementPreview` ganha `bank`**:

```json
{
  "bank": "Banestes",
  "holder": "PAULO ...",
  "account": "12/1234567-8",
  "included": [ /* ClassifiedEntry[] */ ],
  "excluded": [ /* ClassifiedEntry[] com reason: "fatura" | "salario" | "interno" */ ]
}
```

Classificação, exclusões e categorização são **as mesmas** do BTG (`classify_statement`).

---

## `save_bank_statement(bank: String, account: String, entries: ClassifiedEntry[]) -> usize`

**Mudança**: ganha o parâmetro `bank` (hoje o valor `"BTG"` é literal no corpo da função). A UI
devolve o `bank` que recebeu na prévia.

Retorna quantos lançamentos foram gravados (dedup por id: reimportar o mesmo extrato ⇒ nenhum novo).

---

## `import_bank_statement(path: String) -> usize`

**Mudança**: mesmo despacho por extensão do preview; grava com o `bank` do `ParsedStatement`.

---

## `list_bank_entries() -> BankEntry[]`

**Sem mudança de assinatura.** `BankEntry.bank` passa a conter `"BTG"` **ou** `"Banestes"`; a UI
mostra esse campo na lista (FR-012).

---

## Varredura de pasta — `import_from_folder` (feature 013)

**Mudança**: novo ramo para `.pdf`.

| Arquivo na pasta | Resultado |
|---|---|
| `.pdf` que é extrato Banestes | importado; conta em `extratos` e `entries` |
| `.pdf` que não é (contracheque, outro) | `ignored: { reason: "NOT_RECOGNIZED" }` — **nunca** importado como extrato |
| `.pdf` que é Banestes mas não fecha a conferência | `ignored: { reason: "ERROR: <mensagem>" }` — nada gravado |
| `.xlsx` / `.xls` | comportamento atual, inalterado |

Uma falha nunca aborta a varredura; re-varrer não duplica.
