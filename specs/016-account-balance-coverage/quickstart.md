# Quickstart — validar saldo de conta, cobertura e segmentos

**Feature**: 016-account-balance-coverage

## 1. Testes automatizados

```bash
cd src-tauri && cargo test account_position   # entidade + regras puras
cd src-tauri && cargo test banestes           # parser: período, poupança, segmentos
cd src-tauri && cargo test                    # suíte completa (regressão 014/015)
npx vue-tsc --noEmit && npm run test:run      # front
```

| Cenário | Verificação |
|---|---|
| Posição da fixture jul | `balance 231,30`, `as_of 2026-07-25`, produto corrente |
| Consolidado | posição extra `poupanca 5.000,00`; total soma os dois |
| Idempotência | reimportar fixture ⇒ mesma contagem de posições/coberturas |
| Corrente | extrato antigo importado depois ⇒ corrente continua a de maior `as_of` |
| Cobertura | `month_coverage(07)` = `Partial{until 25/07}`; maio+julho sem junho ⇒ `gaps = [2026-06]` |
| Encadeamento | saldo anterior ≠ posição anterior ⇒ `chain_warning` com os dois valores; sem posição anterior ⇒ `None` |
| Segmentos | fixtures íntegras ⇒ `Fechou`; `banestes_extrato_autocancela.txt` (+100/−100) ⇒ recusada citando o dia; sem saldos intermediários ⇒ tolerado |
| FR-011 | `clear_bank_entries` zera posições e coberturas |
| BTG | fixture com `Saldo Diário` ⇒ 1 posição; sem a linha ⇒ nenhuma |

## 2. Validação manual (`npm run tauri dev`)

1. Importar o extrato Banestes real → painel mostra **Saldo em conta: R$ 231,30 · extrato
   de 25/07** (bate com o PDF).
2. Reimportar o mesmo → nada muda (1 posição).
3. ExtratoPage → julho marcado "dados até 25/07"; nenhum buraco (só um extrato).
4. Importar um extrato futuro de agosto (quando existir) → posição corrente troca; se o
   saldo anterior não bater, aviso com os dois valores.
5. Limpar extrato nas Configurações/tela → card de saldo some (sem órfão).

## 3. Sinais de problema

| Sintoma | Onde olhar |
|---|---|
| Saldo do card ≠ PDF | captura do `Saldo Conta` / seleção da posição corrente |
| "não fechou no dia DD" em extrato válido | delimitação de segmento (`Saldo` intermediário vs variantes de rodapé) — contracts/segment_reconciliation.md |
| Aviso de encadeamento indevido | escolha da "posição imediatamente anterior" (`as_of < start`) |
| Mês completo marcado parcial | união de intervalos em `month_coverage` |
