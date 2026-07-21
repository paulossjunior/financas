# Research — Extrato BTG

## Formato (arquivo real, JasperReports .xls OLE2)
- Aba "Extrato". Metadados linhas 0–9: "Cliente: <nome>", CPF, Agência, Conta, Período, Saldo atual.
- Cabeçalho: linha com "Data e hora | Categoria | Transação | … | Descrição | … | Valor" (colunas com gaps → mapear por texto do cabeçalho, não por índice fixo).
- Lançamentos: data "DD/MM/YYYY HH:MM", Categoria BTG, Transação (Pix enviado/Transferência recebida/Pagamento de fatura/Portabilidade de salário…), Descrição (contraparte), Valor (+cred/−deb).
- Ignorar: linhas em branco e Descrição == "Saldo Diário".

## Decisões
- **D1 dedup**: id = UUIDv5(NAMESPACE_OID, "bank:{conta}:{data}:{desc}:{valor}"). Reimport = upsert, sem duplicar.
- **D2 exclusão automática**:
  - fatura do cartão: Transação/Descrição contém "fatura do cart".
  - salário: Categoria BTG == "Salário" OU Transação contém "salário"/"portabilidade de salário" — excluir só quando há contracheque no mês.
  - transferência interna: Descrição normalizada == nome do titular (do cabeçalho) → excluir.
  - motivo registrado (fatura/salário/interno) para a prévia.
- **D3 categorização**: `Categorizer::categorize(descricao)`; se cair em "Outros"/sem match, usar a Categoria do BTG.
- **D4 integração**: incluídos convertidos em `ManualEntry` (kind income p/ crédito, expense p/ débito; recurring=false; month do lançamento) e mesclados ao pipeline → contam como avulso/renda sem nova agregação.
- **D5 parser testável**: `parse_statement_rows(rows: Vec<Vec<String>>)` puro; o reader calamine só produz as linhas.
