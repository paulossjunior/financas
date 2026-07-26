# Quickstart — validar o adapter de fatura Santander

**Feature**: 015-santander-invoice-adapter

## Pré-requisitos

- Node 20+, Rust 1.75+, projeto instalado (`npm install`)
- As 4 faturas Santander reais em `~/Documents/casa/faturas/` (PDF cifrado) e a senha
- Opcional (US4): pasta de importação automática configurada

## 1. Testes automatizados (o essencial)

```bash
cd src-tauri && cargo test santander        # parser + conferência + strategy
cd src-tauri && cargo test btg              # regressão fatura BTG (FR-013)
cd src-tauri && cargo test                  # suíte completa
npx vue-tsc --noEmit && npm run test:run    # front
```

**Esperado**: tudo verde. Fixtures anonimizadas em `tests/fixtures/santander_*.txt` cobrem:

| Cenário | Verificação |
|---|---|
| Fatura jul/2026 | compras + IOFs presentes; Σ fecha com "Saldo Desta Fatura" |
| Internacional | compra pelo R$ impresso; `IOF — <compra>` como transação própria; "COTAÇÃO DOLAR" fora |
| Pagamentos/cashback | `PAGAMENTO DE FATURA`/`DEB AUTOM` não viram transação; `DESCONTO DO MES` vira crédito (`is_reversal`) |
| Multi-cartão | subseções do cartão físico e virtuais somadas |
| Valor 0,00 | anuidade isenta não vira transação |
| Conferência | fixture adulterada ⇒ `Err` com diferença em R$; sem "Resumo da Fatura" ⇒ `Err` do que faltou |
| Detecção | contracheque e extrato Banestes ⇒ "não é uma fatura do Santander" |
| Mês de referência | `Fatura_MMYYYY_…` no nome; fallback vencimento |
| Senha | keychain por banco (mock): salvar/ler/limpar Santander sem tocar a BTG |

## 2. Validação manual ponta a ponta

```bash
npm run tauri dev
```

**US1 — importar (P1)**

1. **Importações → Faturas** → importar → escolher um PDF Santander.
2. App pede a senha (primeira vez) → informar e marcar "lembrar".
3. Conferir: fatura na lista com banco `Santander`, mês certo; transações batem com o PDF
   (amostrar 3 linhas + o total). Dashboard do mês soma o valor da fatura.
4. Importar o **mesmo PDF de novo** → substitui, nada duplica.
5. Importar as outras 3 faturas → **sem** pedir senha de novo (SC-005).

**US2 — conferência (P1)**

6. (Já provada nos testes com a fixture adulterada; manualmente, basta ver que as 4 reais
   importam — se uma recusar, a mensagem diz a diferença.)

**US3 — erros (P2)**

7. Senha errada → mensagem clara, pede de novo, não salva.
8. Selecionar um contracheque PDF no fluxo de fatura → "não é uma fatura do Santander".
9. Importar uma fatura BTG `.xlsx` → comportamento idêntico ao de antes.

**US4 — pasta automática (P3)**

10. Pasta com fatura Santander + extrato Banestes + contracheque; senha salva → reabrir o
    app: resumo informa 1 fatura + 1 extrato; contracheque em silêncio.
11. Limpar a senha Santander nas Configurações → reabrir: fatura em "ignorados" com motivo
    de senha; nada gravado.
12. Reabrir de novo com senha salva: nada duplica.

## 3. Sinais de problema

| Sintoma | Onde olhar |
|---|---|
| "não fechou com o resumo impresso" numa fatura válida | gramática do parser (`domain/santander_invoice.rs`) — linha nova não prevista; comparar com research.md R2 |
| IOF dobrado ou ausente | R3 — associação IOF↔compra anterior |
| Fatura BTG pedindo senha Santander (ou vice-versa) | resolução por banco em `commands/import.rs` (contracts/password_per_bank.md) |
| Contracheque virando fatura | `is_santander_invoice` / ordem do ramo `.pdf` na pasta (contracts/read_santander_invoice.md) |
| Compra com ano errado (dez/jan) | regra do ano em research.md R5 |
