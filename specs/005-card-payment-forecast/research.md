# Research — Previsão de pagamento do cartão

Decisões que resolvem os pontos em aberto antes do design.

## D1 — Como espalhar as parcelas pelos meses futuros

**Decision**: Cada transação parcelada tem `installment { current, total }`, um `amount` (valor da parcela) e pertence a uma fatura com **mês de referência** (a parcela `current` cai nesse mês). As parcelas futuras `current+1 … total` caem em `refMonth+1 … refMonth+(total-current)`, cada uma valendo `amount`. A projeção de um mês = soma dessas parcelas que caem nele.

**Rationale**: Reusa exatamente os dados já extraídos (`compute_weekday_and_installments` já calcula `remaining = total - current`). Só falta distribuir no tempo em vez de somar num total único.

**Alternatives**: Estimar por data da transação em vez de mês de referência — rejeitado: o mês de referência é o ciclo real de cobrança do cartão no app.

## D2 — Evitar contar a mesma compra em dobro

**Decision**: A mesma compra aparece em várias faturas (parcela 1/3 em maio, 2/3 em junho…). Deduplicar por **compra** — chave `(descrição normalizada, total de parcelas, valor da parcela)` — e manter apenas a ocorrência de **maior `current`** (a parcela mais recente conhecida) com o mês de referência dela. Projetar as futuras a partir daí.

**Rationale**: Sem dedup, projetar de 1/3 (jun, jul) e de 2/3 (jul) contaria julho duas vezes. Usar a parcela mais recente dá a posição real do parcelamento hoje.

**Alternatives**: Projetar só a partir da fatura mais recente — frágil se uma compra não aparece na última fatura. A dedup por compra é mais robusta.

## D3 — Âncora temporal (o que é "próximo mês")

**Decision**: Âncora = **mês de referência mais recente entre as faturas importadas**. A projeção começa em `âncora + 1`. Não usa o relógio do sistema.

**Rationale**: A parcela `current` da última fatura já é o mês corrente do usuário; o compromisso futuro começa no mês seguinte. Ser determinístico deixa a função testável (Princípio I) e estável.

**Alternatives**: Usar a data atual do SO — rejeitado: quebra testes e diverge do ciclo de faturas.

## D4 — Horizonte da projeção

**Decision**: Do mês `âncora+1` até o mês da **última parcela pendente** entre todas as compras. Meses sem parcela dentro do intervalo aparecem com valor zero (série contínua, sem buracos no gráfico).

**Rationale**: Mostra quando o compromisso zera (SC-004) e mantém o eixo do tempo contínuo.

## D5 — Onde a projeção vive no DTO

**Decision**: Uma função de domínio `compute_card_forecast(invoices) -> Vec<ForecastPoint>`. O resultado entra:
- `YearSummary.card_forecast: Vec<ForecastPoint>` (gráfico completo na tela Ano).
- `DashboardData`: um resumo compacto (os próximos ~6 meses + total comprometido + mês final) para a tela Mês.

Sem comando novo: reusa `get_year_summary_cmd` e `get_dashboard_cmd`.

**Rationale**: Simplicidade (Princípio III) — nenhuma superfície nova de API; ambas as telas já buscam esses DTOs.

## D6 — Estornos e valores

**Decision**: Ignorar transações com `is_reversal` na projeção (estorno não gera parcela futura). Dinheiro em `Decimal`; valor por parcela constante (o app não tem juros por parcela).

**Rationale**: Integridade (Princípio IV); evita inflar a projeção.

## Resolved unknowns

Todos os NEEDS CLARIFICATION da Technical Context foram resolvidos acima. Nenhum pendente.
