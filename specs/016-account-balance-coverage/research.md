# Research — Saldo de conta, cobertura e conferência por segmento

**Feature**: 016-account-balance-coverage · **Date**: 2026-07-26

Base empírica: texto completo do extrato Banestes real de jul/2026 (63 linhas, extraído em
investigação fora do repo — anonimizado nas fixtures) + fixture BTG de testes.

## R1 — De onde vem cada dado novo

| Dado | Linha do extrato Banestes | Situação pré-016 |
|---|---|---|
| Saldo final da conta | `Saldo Conta 231,30` (rodapé "Saldos") | lido p/ conferência, descartado |
| Saldo da poupança | `Saldo Poupança 5.000,00` (consolidado) | dropado |
| Período coberto | `Cliente: … Período: 01/07/2026 à 25/07/2026` | regex corta e joga fora |
| Saldos diários | `JUL/26 Saldo 6.637,41` após cada dia | puro descarte |
| (BTG) Saldo Diário | linha `Saldo Diário` no grid `.xls` | pulada pelo parser |

## R2 — Posição: identidade e "corrente"

**Decision**: `AccountPosition { bank, account, product, balance, as_of, source_file }`
com `product ∈ {Corrente, Poupanca}` e id = `UUIDv5("position:{bank}:{account}:{product}:{as_of}")`.
"Posição corrente" = maior `as_of` por (bank, account, product) — regra pura no domínio,
independente da ordem de importação (spec US1 cenário 5).

**Rationale**: id determinístico dá idempotência via `INSERT OR REPLACE` (mesma receita do
`bank_entries`); `as_of` = fim do período coberto (Banestes) ou data do último Saldo
Diário (BTG). `source_file` para rastrear e para o FR-011.

**Alternatives**: tabela de contas separada + FK (CRUD implícito — YAGNI, spec exclui);
guardar só a corrente (perderia histórico, que o spec exige preservar).

## R3 — Cobertura: união e derivadas

**Decision**: persistir uma linha por importação (`start`, `end`, por conta); as derivadas
são funções puras sobre o conjunto: `merge` (união de intervalos), `month_status(mês)` →
`Full | Partial{até} | None`, `gaps(primeiro..último)` → meses sem cobertura,
`chain_warning(posições, saldo_anterior_novo)` → aviso quando o saldo anterior do extrato
novo ≠ saldo final da posição imediatamente anterior.

**Rationale**: guardar o fato bruto (período de cada importação) e derivar o resto mantém
o modelo à prova de reimportação/sobreposição (união resolve; nada de flags persistidas
que dessincronizam).

**Alternatives**: persistir "meses completos" materializados (dessincroniza ao remover
extrato); bloquear import com encadeamento divergente (spec manda avisar — extrato
faltando no meio é o caso comum).

## R4 — Segmentos no parser Banestes

**Decision**: durante a varredura, cada linha `Saldo <valor>` intermediária fecha um
segmento: `saldo_do_segmento_anterior + Σ movimentos do trecho == saldo_lido`. Divergiu →
`Checagem::Divergiu` numa nova checagem `segmentos` da `Conferencia`, com mensagem citando
o dia do primeiro segmento que não fechou e a diferença. Sem linhas de saldo intermediárias
→ `Checagem::SemDados`, tratada como **não-bloqueante** (diferente das outras duas): a
conferência total continua obrigatória; a segmentada é rede extra quando o extrato a
oferece (spec US3 cenário 3).

**Rationale**: pega +100/−100 auto-cancelado que a soma total não vê. O dado já está no
texto; as fixtures existentes já o contêm (zero fixture nova além da adulterada).

**Alternatives**: exigir segmentos sempre (quebraria extrato futuro sem eles — spec manda
degradar); validar sem abortar (violaria a política estrita: parser divergente = leitura
errada = não importa).

## R5 — BTG: melhor esforço honesto

**Decision**: no `parse_statement_rows`, a última linha cujo texto normalizado contém
`SALDO DIARIO` com valor e data vira UMA posição (`as_of` = data da linha). Sem essa linha
→ sem posição. Cobertura BTG: **não registrada** (o `.xls` não imprime período; min/max
dos lançamentos mentiria sobre dias sem movimento nas pontas).

**Rationale**: o dado existe no arquivo real (a fixture de teste do BTG já traz
`Saldo Diário … 5160`); usar só o que é impresso mantém a filosofia "dado incompleto se
declara incompleto".

**Alternatives**: inferir cobertura de datas de lançamento (mente); ignorar BTG por
completo (joga fora dado impresso).

## R6 — Persistência e remoção acoplada

**Decision**: tabelas `account_positions(id PK, bank, account, product, balance TEXT,
as_of TEXT, source_file TEXT, imported_at TEXT)` e `statement_coverage(id PK, bank,
account, start TEXT, end TEXT, source_file TEXT)`; `INSERT OR REPLACE`. `clear_bank_entries`
passa a limpar as três tabelas (FR-011 — sem saldo órfão); remoção de lançamento avulso
NÃO remove posição (posição vem do extrato, não do lançamento).

## R7 — Fluxo do aviso de encadeamento

**Decision**: `save_bank_statement`/`import_bank_statement` retornam struct
(`{saved: usize, chain_warning: Option<String>}` — DTO `SaveStatementResult`) em vez do
`usize` cru; a comparação usa a posição corrente anterior a `coverage.start`. Front mostra
o aviso no flash/banner da ExtratoPage. Pasta automática: aviso vai no summary
(`ignored`-like informativo? não — campo novo `warnings: Vec<String>` no
`FolderImportSummary`, exibido uma vez).

**Rationale**: aviso precisa chegar ao usuário no momento do import (spec FR-007), sem
bloquear e sem virar erro.

## R8 — Dashboard

**Decision**: comando `list_account_positions` → todas as posições; o card do painel
agrupa por (bank, account, product), pega a corrente de cada, soma o total. Comando
`coverage_summary` → por conta: meses parciais (com "até dia X") e buracos, consumido pela
ExtratoPage e pelo card (badge "dados até 25/07").

**Alternatives**: calcular no front (regra de negócio vazaria do domínio — as funções
puras ficam no Rust, o front só exibe).
