# Contract: adapter de leitura do extrato Banestes

**Feature**: 014-banestes-statement-adapter

Duas funções públicas, uma por camada. Nenhum tipo novo exposto.

---

## `domain::banestes_statement`

### `is_banestes_statement(text: &str) -> bool`

Verdadeiro quando o texto contém `Extrato de Conta Corrente` **e** (`Saldo Anterior` **ou**
`Agência:`), comparação normalizada (maiúsculas, sem acento). Puro, sem I/O.

Usada para despachar sem tentar parsear: um contracheque SouGov.br devolve `false`.

### `parse_banestes_text(text: &str) -> Result<ParsedStatement, String>`

Texto extraído do PDF → `ParsedStatement { bank: "Banestes", holder, account, entries }`.

**Garantias**

| # | Garantia |
|---|---|
| C1 | `bank == "Banestes"`; `account == "<agência>/<conta>"`; `holder` sem o sufixo `Período: …` |
| C0 | Número de documento e hora da operação **não** entram na `description` (ficam fora do texto mostrado ao usuário) |
| C2 | Cada `RawEntry` tem `amount` com sinal: negativo = saída, positivo = entrada |
| C3 | `date` = data da operação do lançamento; sem ela, dia da coluna + mês/ano do grupo |
| C4 | Linha de saldo (`Saldo Anterior`, `Saldo`, `Saldo Conta`, `Saldo Total`), cabeçalho, bloco de totais do topo e rodapé **nunca** viram `RawEntry` |
| C5 | Lançamento quebrado em duas linhas vira **um** `RawEntry` com a descrição completa |
| C6 | `btg_category` é sempre `""` |
| C7 | `transaction` = tipo de operação; `description` = contraparte |

**Erros** (`Err(String)`, mensagem em português, pronta para a UI)

| Situação | Mensagem |
|---|---|
| Texto não é extrato Banestes | `Este PDF não é um extrato do Banestes.` |
| Cabeçalho da tabela ausente | `Não reconheci o formato deste extrato Banestes.` |
| Nenhum lançamento no período | `O extrato não tem lançamentos no período.` |
| Soma dos lançamentos ≠ saldo final − saldo anterior | `A leitura do extrato não fechou com os saldos (diferença de R$ X). Nada foi importado.` |
| Soma de créditos/débitos ≠ entradas/saídas declaradas | `A leitura do extrato não fechou com as entradas e saídas declaradas (diferença de R$ X). Nada foi importado.` |

**Determinismo**: mesmo texto ⇒ mesmo resultado, mesma ordem (Constituição IV).

---

## `infrastructure::banestes_statement`

### `read_statement(path: &str) -> Result<ParsedStatement, String>`

1. `pdf_extract::extract_text(path)` — falha ⇒ `Não consegui ler o PDF: {e}`
2. Texto vazio ou sem caracteres úteis ⇒ `Este PDF não tem texto para ler (pode ser digitalizado).`
3. Delega a `domain::banestes_statement::parse_banestes_text`

Nenhuma outra responsabilidade — sem DB, sem config, sem Tauri.
