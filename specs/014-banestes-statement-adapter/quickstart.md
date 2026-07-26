# Quickstart — validar o adapter de extrato Banestes

**Feature**: 014-banestes-statement-adapter

## Pré-requisitos

- Node 20+, Rust 1.75+, projeto instalado (`npm install`)
- Um extrato de conta corrente do Banestes em PDF (baixado do internet banking)
- Opcional, para US2: um contracheque já importado no mês do extrato

## 1. Testes automatizados (o essencial)

```bash
cd src-tauri && cargo test banestes        # parser do extrato Banestes
cd src-tauri && cargo test bank_statement  # regressão do extrato BTG (ids, exclusões)
cd src-tauri && cargo test                 # suíte completa
npx vue-tsc --noEmit                       # tipos do front
npm run test:run                           # Vitest
```

**Esperado**: tudo verde. Os testes do parser usam
`src-tauri/tests/fixtures/banestes_extrato.txt` (texto anonimizado) e cobrem:

| Cenário | Verificação |
|---|---|
| Extrato de jul/2026 | 9 lançamentos; saídas somam `7106.11`; entradas somam `0` |
| Conferência | `saldo_anterior 7337.41 + Σ = saldo_final 231.30` |
| Linha quebrada | `ALFA COMERCIO E REPRESENTACOES LTDA` = 1 lançamento, `-2729.78` |
| Linhas de saldo | `Saldo Anterior` / `Saldo` / `Saldo Conta` / `Saldo Total` fora do resultado |
| Data da operação | linha do dia `20` gera `date = 2026-07-19` |
| Metadados | `bank = "Banestes"`, `account` = `<agência>/<conta>` da fixture (valores fictícios), `holder` sem `Período:` |
| Entrada (crédito) | valor sem sinal ⇒ `amount > 0`, `kind = income` |
| Totais não fecham | valor alterado na fixture ⇒ `Err` e nada é gravado |
| Não é Banestes | texto de contracheque ⇒ `is_banestes_statement == false` |

## 2. Validação manual ponta a ponta

```bash
npm run tauri dev
```

**US1 — importar (P1)**

1. Menu **Importações → Extrato** → botão de importar → escolher o **PDF** do Banestes.
2. Conferir na prévia: banco `Banestes`, conta `12/1234567-8`, os lançamentos com data, contraparte
   e valor iguais ao PDF; nenhuma linha "Saldo".
3. Confirmar. Mensagem informa quantos lançamentos entraram.
4. Importar **o mesmo PDF outra vez** → deve gravar **0** novos.
5. Escolher um PDF de contracheque → mensagem clara, nada importado.

**US2 — exclusões e categorização (P1)**

6. Num extrato que tenha pagamento de fatura de cartão, crédito de salário (com contracheque no mês)
   e transferência para você mesmo: os três aparecem na lista de **excluídos** com o motivo.
7. Lançamentos sem palavra-chave aparecem como **Outros** e surgem em **Categorização**; criar uma
   palavra-chave lá recategoriza também os do cartão.

**US3 — dois bancos (P2)**

8. Importar também um extrato BTG (`.xls`). A lista de lançamentos mostra o banco de cada um.
9. **Movimentações** e **Despesas & Receitas** do mês somam as duas contas; nada que já vem de
   fatura ou contracheque aparece em dobro.

**US4 — pasta automática (P3)**

10. Em **Configurações**, apontar a pasta de importação automática para uma pasta com o PDF do
    extrato + um PDF de contracheque.
11. Reiniciar o app: o resumo informa `1 extrato` importado; o contracheque não gera lançamento.
12. Reiniciar de novo: nada duplicado.

## 3. Sinais de problema

| Sintoma | Onde olhar |
|---|---|
| "não fechou com os saldos" num extrato válido | gramática do parser (`domain/banestes_statement.rs`) — provável linha nova não prevista; comparar com `research.md` R2 |
| Lançamento com valor de outra linha | junção de continuação / âncora de valor no fim da linha |
| Lançamentos do BTG duplicando após esta feature | `entry_id`: a primeira ocorrência **tem** de manter a chave antiga |
| Contracheque virando extrato | `is_banestes_statement` |
