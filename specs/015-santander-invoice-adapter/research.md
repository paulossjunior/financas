# Research — Adapter de fatura Santander (PDF)

**Feature**: 015-santander-invoice-adapter · **Date**: 2026-07-26

Base empírica: as 4 faturas reais do usuário (`Fatura_022026_*_5464_*`, `Fatura_052026`,
`Fatura_062026`, `Fatura_072026_*_6428_*`), decifradas e extraídas com
`pdf_extract::extract_text_encrypted` numa investigação fora do repositório. Nenhum PDF nem
texto real entrou no repo; os trechos abaixo estão anonimizados.

## R1 — Decifragem: `pdf_extract::extract_text_encrypted`

**Decision**: usar `pdf_extract::extract_text_encrypted(path, senha)` quando o PDF for
cifrado; `extract_text` para PDF aberto. Detectar cifra tentando `extract_text` primeiro e
caindo para a variante com senha quando o erro contiver o marcador de documento cifrado.

**Rationale**: lib já usada (contracheque, Banestes) — zero dependência nova. Testado nas 4
faturas reais: senha errada → `PDF error: the supplied password is incorrect` (vira
`WRONG_PASSWORD`); sem senha → erro pedindo `extract_text_encrypted` (vira
`ENCRYPTED_FILE`); com senha certa → 5.6–12.1k chars de texto útil.

**Alternatives considered**: `lopdf` direto (mais controle, mais código); `qpdf` externo
(dependência de binário — viola local-first simples).

## R2 — Gramática do texto extraído

**Decision**: parser por varredura de linhas com âncoras estruturais, no molde do
`ExtratoBanestes`. Seções e regras (anonimizado):

```text
Detalhamento da Fatura
FULANO F F TAL -  4220 XXXX XXXX 1234        ← subseção por cartão (físico e virtuais)
Pagamento e Demais Créditos                  ← bloco de créditos
Compra Data Descrição Parcela R$ US$         ← cabeçalho de colunas (repete por bloco)
  03/06 PAGAMENTO DE FATURA-INTERNET -4.923,40    ← EXCLUIR (transferência)
  15/01 DEB  AUTOM  DE FATURA EM C/ -4.867,94     ← EXCLUIR (débito automático da fatura)
  29/06 DESCONTO DO MES -149,30                   ← cashback → crédito (is_reversal)
Despesas                                     ← bloco de despesas
  06/06 AWS BRAZIL 4.004,50                       ← nacional: data dd/mm, descrição, R$
  03/06 ANTHROPIC* TEAM T1 7.019,45 1.320,29      ← internacional: R$ + US$ (US$ ignorado)
COTAÇÃO DOLAR R$ 5,3166                           ← sub-linha: DESCARTAR
IOF DESPESA NO EXTERIOR 245,68                    ← sub-linha: vira transação própria (IOF)
2 02/04 OPENAI 546,99 100,00                      ← nº na coluna "Compra": ignorar o prefixo
  09/02 ANUIDADE DIFERENCIADA 0,00                ← valor 0,00: não vira transação
VALOR TOTAL 19.038,29 2.011,40               ← total da subseção: âncora de fim, não é transação
Resumo da Fatura                             ← bloco da conferência (R7)
```

**Rationale**: mesma técnica que já provou funcionar no Banestes (linha termina em
dinheiro; estado por seção; regras explícitas para cada linha que não vira transação).

**Alternatives considered**: parsing posicional por coordenadas do PDF (pdf_extract não as
expõe de forma estável); regex única por página (frágil a quebras de linha).

## R3 — IOF de compra internacional é transação própria

**Decision**: cada `IOF DESPESA NO EXTERIOR <valor>` vira uma `Transaction` com descrição
`IOF — <descrição da compra anterior>` e a data da compra a que se refere.

**Rationale**: o "Total Despesas/Débitos no Exterior" do Resumo **inclui** o IOF (conferido
na fatura real: 10.783,10 = compras US$ em reais + IOFs). Embutir o IOF no valor da compra
tornaria a linha diferente do PDF (o usuário confere contra o papel); descartá-lo quebraria
a conferência. Linha própria = fiel ao extrato e fecha a soma.

**Alternatives considered**: embutir na compra (valor divergiria do impresso); ignorar
(conferência nunca fecharia).

## R4 — Pagamentos de fatura excluídos; cashback é crédito

**Decision**: linhas de "Pagamento e Demais Créditos" cuja descrição normalizada contém
`PAGAMENTO DE FATURA` ou `DEB AUTOM DE FATURA` **não viram transação**. `DESCONTO DO MES`
(cashback) vira transação de valor negativo (o `Transaction::new` já marca `is_reversal`
para valor < 0 — mesmo mecanismo do estorno BTG).

**Rationale**: o pagamento da fatura é transferência conta→cartão; já aparece no extrato
bancário (e lá é excluído como `fatura` desde a 008/014) — entrar aqui duplicaria. O
cashback é dinheiro real que abate o mês. A conferência (R7) subtrai os dois blocos do lado
declarado, então a exclusão é modelada **dentro** da identidade contábil, não fora.

## R5 — Ano das compras (`dd/mm` sem ano)

**Decision**: ano = ano do mês de referência; se `mm` da compra > `mm` do vencimento, ano =
ano de referência − 1.

**Rationale**: fatura 07/2026 traz compras de 29/05 a 29/06 (meses anteriores ao
vencimento). A janela de compras nunca chega perto de 12 meses, então a única virada
possível é dez→jan (fatura 01/2027 com compras 11–12/2026), coberta pela regra.

## R6 — Identidade e dedup

**Decision**: reusar exatamente o mecanismo BTG: `invoice_id = UUIDv5(filename)` e
`Transaction::new(invoice_id, row_index, …)` com `row_index` = índice sequencial da
transação na ordem do PDF.

**Rationale**: reimportar o mesmo arquivo → mesmo id → `store.add` substitui (FR-009).
Índice sequencial é determinístico para o mesmo arquivo (mesma extração → mesmas linhas na
mesma ordem).

## R7 — Conferência pelo "Resumo da Fatura"

**Decision**: obrigatória e estrita (política da 014). Do bloco:

```text
Saldo Anterior 4.923,40
(+) Total Despesas/Débitos no Brasil 8.255,19
(+) Total Despesas/Débitos no Exterior 10.783,10 2.011,40
(-) Total de pagamentos   21.005,57
(-) Total de créditos  149,30
(=) Saldo Desta Fatura 2.806,82
```

Duas checagens independentes: (a) `Σ despesas lidas` (compras + IOFs, todas as subseções)
`== Despesas Brasil + Despesas Exterior` — **agregada de propósito**: na fatura real o
"Total no Exterior" (10.783,10) é exatamente a soma dos R$ das compras em US$, e os IOFs
(377,41) entram no "Total no Brasil" (7.877,78 + 377,41 = 8.255,19); somar os dois lados
declara a mesma coisa sem depender de classificar IOF por coluna. (b) `Σ créditos lidos`
(cashback) + `Σ pagamentos excluídos` `== Total de créditos + Total de pagamentos`.
Qualquer `Divergiu` ou `SemDados` ⇒ `Err` pt-BR com a diferença em R$, nada gravado. A
identidade completa (saldo anterior + despesas − pagamentos − créditos = saldo da fatura) é
conferida como validação do próprio resumo.

**Rationale**: mesmo racional do Banestes — PDF é entrada frágil; linha perdida tem de
virar erro visível, nunca mês mais barato. Validado nas 4 faturas reais: a identidade fecha
com `Decimal` exato em todas.

## R8 — Detecção de fatura Santander vs outros PDFs

**Decision**: dois níveis.

1. **Com texto em mãos** (`is_santander_invoice(text)`): marcadores estruturais —
   `Detalhamento da Fatura` + (`Resumo da Fatura` ou `BANCO SANTANDER`). Contracheque
   SouGov e extrato Banestes não têm nenhum dos dois primeiros.
2. **Sem texto** (PDF cifrado, senha ausente — caso da pasta automática): PDF **cifrado** é
   tratado como candidato a fatura Santander (é o único documento cifrado do domínio do
   app; contracheque e extrato Banestes são abertos). Com senha salva → decifra e confirma
   pelo nível 1; sem senha → `ignored` com motivo `ENCRYPTED_NO_PASSWORD` (código que o
   front já traduz desde a 013).

**Rationale**: a pasta precisa distinguir sem poder ler; a cifra é o discriminador
disponível e honesto. Falso positivo (PDF cifrado de outra origem) morre no nível 1 após
decifrar, sem importar nada.

**Alternatives considered**: casar pelo nome `*_SANTANDER.PDF` (frágil — o usuário renomeia
arquivos); pedir senha na varredura automática (quebra o "sem intervenção" da 013).

## R9 — Senha por banco no keychain

**Decision**: `secrets.rs` parametrizado por banco: a chave BTG continua
`invoice-password` (compatibilidade com a senha já salva do usuário); Santander usa
`invoice-password-santander`. O comando `import_invoices` resolve a senha efetiva pelo
banco do leitor escolhido para cada arquivo, e `remember` grava na chave desse banco.

**Rationale**: senhas são credenciais distintas (CPF vs senha de planilha BTG); uma chave
única sobrescreveria a outra. Manter o nome antigo para o BTG evita re-pedir uma senha que
o usuário já salvou.

**Alternatives considered**: uma chave única compartilhada (colisão de credenciais);
migrar a chave BTG para nome novo (perderia a senha salva sem ganho).

## R10 — Mês de referência

**Decision**: `Fatura_MMYYYY_…` do nome do arquivo (regex `Fatura_(\d{2})(\d{4})`);
fallback: `Vencimento dd/mm/yyyy` impresso no corpo (mês/ano do vencimento).

**Rationale**: os 4 arquivos reais seguem o padrão `MMYYYY`; o fallback cobre renomeações.
O `infer_month_from_filename` do BTG espera `YYYY-MM-…`, então o Santander resolve o mês
dentro do próprio leitor/domínio (formato diferente) e o resultado alimenta o mesmo
`Invoice.reference_month`.
