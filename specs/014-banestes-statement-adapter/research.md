# Phase 0 — Research: adapter de extrato Banestes (PDF)

**Feature**: 014-banestes-statement-adapter · **Data**: 2026-07-25

Todas as incógnitas foram resolvidas com o extrato real (jul/2026). Nenhum
`NEEDS CLARIFICATION` restante.

---

## R1. Como o app consegue texto de um PDF (e a ordem é confiável?)

**Decisão**: usar `pdf_extract::extract_text` (crate `pdf-extract 0.7`, **já** dependência do
projeto, usada pelo contracheque). Nenhuma dependência nova.

**Rationale**: rodei a extração no extrato real. O texto sai **linha a linha, com o valor na
mesma linha do lançamento** — exatamente o que o parser precisa:

```text
Data Lançamento Valor (R$)
Saldo Anterior  7.337,41
03  Pix Enviado 03/07/2026-21:49:38 1000000000001 Ana Paula Martins Lima - 700,00
JUL/26 Saldo  6.637,41
06  Pix Enviado 06/07/2026-11:24:10 1000000000002 CARLA MENDES ROCHA - 320,00
JUL/26 Saldo  6.317,41
07  Pix Enviado 07/07/2026-08:20:31 1000000000003 ALFA COMERCIO E REPRESENTACOES
LTDA - 2.729,78
JUL/26  Pix Enviado 07/07/2026-14:32:18 1000000000004 BETA SERVICOS MEDICOS LTDA - 500,00
 Pix Enviado 07/07/2026-14:34:47 1000000000005 GAMA SERVICOS MEDICOS LTDA - 750,00
Saldo  2.337,63
```

**Alternativas rejeitadas**:

- `pdftotext -layout` (poppler, binário externo): resolve o layout, mas viola "sem dependência
  externa não auditável" e não existe em Windows por padrão.
- Ler a ordem crua de objetos do PDF (o que `pdftotext` **sem** `-layout` devolve): o texto sai
  embaralhado — valores separados dos lançamentos (`- 500,00` aparece 30 linhas depois da linha a
  que pertence). Confirmado no arquivo real; é a razão de FR-004 existir. `pdf_extract` **não**
  tem esse problema.

**Risco residual**: a extração pode mudar de comportamento numa atualização do crate. Mitigação:
a checagem de FR-005 (entradas/saídas + saldos) transforma qualquer regressão de leitura em erro
explícito em vez de valor errado gravado.

---

## R2. Gramática do extrato Banestes (o que o parser precisa reconhecer)

**Decisão**: parser de linhas com estado (dia/mês corrente), âncora de valor no fim da linha e
junção de continuação.

Estrutura observada:

| Elemento | Forma no texto extraído | Tratamento |
|---|---|---|
| Metadados de conta | `Agência: 12 - CENTRO Conta: 1234567-8` | agência = número antes de ` - `; conta = token depois de `Conta:` |
| Titular + período | `Cliente: MARIA APARECIDA DA SILVA SOUZA Período: 01/07/2026 à 25/07/2026` | titular = texto entre `Cliente:` e `Período:` (cortar em `Período:`) |
| Totais do topo | `SALDO TOTAL ENTRADAS E SAÍDAS` / `R$ 231,30   R$ 0,00` / `R$ 7.106,11` | 3 valores na ordem: saldo total, **entradas**, **saídas** |
| Cabeçalho da tabela | `Data Lançamento Valor (R$)` | início da tabela |
| Saldo inicial | `Saldo Anterior  7.337,41` | valor de referência, **não** é lançamento |
| Dia / mês | prefixo `03` na linha do lançamento; `JUL/26` no início da linha seguinte | dia da coluna + mês do grupo (fallback de data) |
| Lançamento | `03  Pix Enviado 03/07/2026-21:49:38 1000000000001 Ana Paula Martins Lima - 700,00` | tipo + `dd/mm/aaaa-hh:mm:ss` + doc + contraparte + valor |
| Continuação | linha do lançamento termina sem valor; a próxima traz `LTDA - 2.729,78` | juntar as duas antes de interpretar |
| Saldo do dia | `JUL/26 Saldo  6.637,41` / `Saldo  2.337,63` | descartar |
| Saldos finais | `Saldo Conta  231,30` / `Saldo Total  231,30` | saldo final para a conferência |
| Rodapé | `Extrato Consolidado Até ...` / `Data/Hora Emissão: ...` / hash | descartar |

Regras derivadas:

- **Valor**: `-?\s?\d{1,3}(\.\d{3})*,\d{2}` no **fim** da linha. Sinal `-` (com espaço) ⇒ saída
  (débito); sem sinal ⇒ entrada (crédito).
- **Linha de saldo**: depois de remover prefixo de dia/mês, começa com `Saldo`. Descartada sempre.
- **Data**: `dd/mm/aaaa` embutido no lançamento é a data da operação e tem prioridade (no extrato
  real, a linha do dia `20` tem operação em `19/07/2026`). Sem data embutida, usa `dia da coluna` +
  `mês/ano do grupo`.
- **Continuação**: linha que tem tipo de operação mas **não** termina em valor ⇒ concatenar com a
  linha seguinte.

**Alternativa rejeitada**: regex única por linha para o formato inteiro. Falha nas linhas quebradas
e nas linhas sem data de operação (tarifas, débito automático), que o extrato pode conter em outros
meses.

---

## R3. Como detectar que o PDF é um extrato Banestes

**Decisão**: detectar por marcadores estruturais no texto — `Extrato de Conta Corrente` **e**
(`Saldo Anterior` **ou** `Agência:`) — não pelo nome do arquivo nem pela extensão.

**Rationale**: a palavra "Banestes" **não** aparece no texto extraído (o logo é imagem). Os
marcadores acima não existem num contracheque SouGov.br, que é o outro PDF que o usuário tem na
pasta (o parser de contracheque procura `MÊS/ANO` + itens de rendimento/desconto). Detecção por
conteúdo atende FR-011 e FR-015 (o usuário não informa o banco; a pasta não confunde os dois).

**Alternativa rejeitada**: decidir por nome de arquivo (`Banestes_extrato_*.pdf`) — quebra se o
usuário renomear.

---

## R4. Conferência de integridade (FR-005)

**Decisão**: duas checagens, executadas antes de qualquer gravação:

1. `saldo_anterior + Σ lançamentos == saldo_final` — obrigatória (o extrato sempre imprime os dois).
2. `Σ créditos == entradas declaradas` e `Σ |débitos| == saídas declaradas` — executada quando o
   bloco de totais do topo é lido com sucesso.

Falha em qualquer uma ⇒ erro explícito, nada gravado.

**Verificação no arquivo real**: `7.337,41 − 231,30 = 7.106,11`; soma dos 9 débitos =
`700 + 320 + 2.729,78 + 500 + 750 + 750 + 427,86 + 800 + 128,47 = 7.106,11`; entradas declaradas
`R$ 0,00` = soma dos créditos (nenhum). Fecha exato — serve de fixture de aceitação (SC-001).

**Rationale**: Constituição IV (nenhum erro silencioso em dinheiro). Um PDF é entrada frágil; sem
essa conferência, uma linha perdida viraria "mês mais barato" sem nenhum aviso.

**Alternativa rejeitada**: apenas avisar e deixar importar. Rejeitada: o usuário não tem como
descobrir depois qual linha faltou.

---

## R5. Reuso máximo sem entidades novas (pedido explícito do usuário)

**Decisão**: o adapter produz `ParsedStatement`/`RawEntry` — as estruturas que o extrato BTG já
usa — e daí para frente o caminho é **o mesmo código**: `classify_statement` (exclusões +
categorização), `BankEntry::from_classified` (gravação), `to_manual_entry` (dashboard).

Ajustes mínimos em código existente (nenhuma entidade nova):

| Ajuste | Por quê |
|---|---|
| `ParsedStatement` ganha campo `bank` | o banco precisa chegar até `BankEntry` sem um segundo canal; hoje `"BTG"` é literal em 3 lugares |
| helpers `norm` / `parse_amount` de `domain::bank_statement` passam a `pub(crate)` | o parser Banestes usa as mesmas regras de normalização — duplicar violaria DRY |
| detecção de fatura de cartão em `classify_entry` passa a exigir `FATURA` + `CART` em vez do literal `FATURA DO CART` | o Banestes escreve "Pagamento Fatura Cartão"; continua casando o texto do BTG |
| `entry_id` ganha índice de ocorrência (0 = chave atual) | dois lançamentos idênticos no mesmo dia hoje colapsam num só; o índice 0 preserva **todos** os ids já gravados |

**Alternativa rejeitada**: incluir o número do documento na descrição para garantir unicidade.
Rejeitada: sujaria a lista, a fila de categorização (agruparia por "contraparte + doc", um item por
pix) e os relatórios.

**Alternativa rejeitada**: trait `StatementReader` com implementações BTG/Banestes. Rejeitada por
YAGNI/Constituição III — são dois leitores concretos e uma função de despacho por extensão resolve;
trait entra quando aparecer o terceiro banco.

---

## R6. Onde o código mora (Constituição II)

**Decisão**: espelhar a divisão que o BTG já usa.

- `domain/banestes_statement.rs` — **puro**: `&str` (texto do PDF) → `ParsedStatement` + conferência
  de totais. Zero I/O, zero Tauri, 100% testável com fixture de texto.
- `infrastructure/banestes_statement.rs` — **I/O**: caminho do arquivo → `pdf_extract::extract_text`
  → chama o domínio.
- `commands/bank.rs` — despacho por extensão (`.pdf` → Banestes, `.xls`/`.xlsx` → BTG).
- `application/import_folder.rs` — ramo `.pdf` na varredura.

**Rationale**: idêntico ao par `infrastructure/btg_statement.rs` (I/O) + `domain/bank_statement.rs`
(regras). Mantém a inversão de dependência: domínio não conhece PDF nem arquivo.

---

## R7. Fixture de teste sem expor dado real

**Decisão**: fixture de **texto** anonimizado em `src-tauri/tests/fixtures/banestes_extrato.txt`
(saída de `extract_text` com titular, contrapartes e número de conta substituídos; datas e valores
preservados para a conferência de SC-001). Os testes do parser leem esse texto.

**Rationale**: o extrato real tem nome completo, agência/conta e nomes de terceiros — não vai para
o repositório. O texto anonimizado exercita a gramática inteira (linha quebrada, saldos, dia sem
mês, operação em dia diferente do lançamento).

**Consequência**: a camada de leitura de PDF (`infrastructure`) fica sem fixture binária; ela é
uma casca fina (extrair texto + delegar) e é validada manualmente pelo quickstart com o PDF real
do usuário.
