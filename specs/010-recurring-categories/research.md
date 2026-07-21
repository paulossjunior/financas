# Research: Categorias recorrentes + baseline + anti-duplicação

Decisões de projeto (Fase 0). Cada item: **Decisão**, **Justificativa**, **Alternativas rejeitadas**.

## D1 — Recorrência armazenada POR CATEGORIA em tabela nova

**Decisão**: Criar a tabela `recurring_categories (category TEXT PRIMARY KEY, start_month TEXT NULL, end_month TEXT NULL, created_at TEXT NOT NULL)`. A flag de recorrência e a vigência ficam **por categoria**, não em `category_rules`.

**Justificativa**:
- `category_rules` é **por palavra-chave** (várias linhas `category/keyword/priority` por categoria — ver `db.rs` e `load_config`). Colocar recorrência ali obrigaria a repetir/derivar o estado em N linhas e a resolver conflitos entre keywords da mesma categoria. A recorrência é um atributo de **categoria**, então uma chave primária `category` modela o domínio diretamente.
- Chave simples permite `PRIMARY KEY(category)` com upsert idempotente (`ON CONFLICT DO UPDATE`), no mesmo estilo de `bank_entries` e `inflation_cache`.
- Mantém `category_rules` intacta (feature 003), sem migração destrutiva.

**Nota**: A ideia bruta do usuário dizia "in category_rules". Esta decisão diverge de propósito — recorrência é keyed por categoria na tabela nova. Registrado aqui conforme pedido.

**Alternativas rejeitadas**:
- *Colunas `recurring/start_month/end_month` em `category_rules`*: estado duplicado por keyword, ambíguo, e mistura dois conceitos (mapeamento de merchant × recorrência de categoria).
- *Chave `settings` (JSON)*: perde consulta relacional e integridade; difícil de versionar/migrar.

## D2 — Janela do baseline = 3 meses

**Decisão**: `baseline(category, history, n=3)` = média dos **últimos 3 meses** realizados daquela categoria recorrente (ou dos meses disponíveis se houver menos de 3; zero sem histórico). Carrega flag `is_baseline`.

**Justificativa**: Valor decidido com o usuário (spec, Assumptions e FR-007). Três meses suaviam variação de contas variáveis (água/luz) sem arrastar sazonalidade antiga. `n` é parâmetro do domínio, fácil de revisar depois sem mudar a assinatura pública.

**Alternativas rejeitadas**:
- *Último mês só*: sensível a picos/estornos, ruim para água/luz.
- *12 meses*: arrasta reajustes antigos; distorce o Teto atual.
- *Mediana*: mais robusta a outliers, mas o usuário pediu média; mantida simplicidade (YAGNI).

## D3 — Equivalência para anti-duplicação = mesma categoria + mesmo mês

**Decisão**: Um fixo **manual** numa categoria recorrente é **suprimido inteiro** quando existe qualquer lançamento **importado equivalente** (mesma categoria recorrente **e** mesmo mês). Não há casamento por valor exato. O domínio devolve quais manuais foram suprimidos.

**Justificativa**: Espelha o mecanismo já existente `payslip → salário manual` (`get_dashboard.rs`: `manual_agg.retain(...)` por `is_salary && payslip_months.contains(month)`; `year.rs`: `superseded = e.is_salary && payslip_by_month.contains_key(&m)`). Consistência com o app e simplicidade. Cobre o caso do usuário que já tinha "Aluguel R$ 2.000" manual e passa a importar o extrato.

**Alternativas rejeitadas**:
- *Casar por valor aproximado*: frágil (água/luz variam); geraria duplicatas quando o valor real difere do manual.
- *Casar por descrição*: descrições de extrato/fatura diferem do texto manual; alto índice de falso-negativo.

## D4 — Detecção: ≥3 de 4 meses + variação limitada, opt-in, dispensas persistidas

**Decisão**: `detect_recurring(history)` retorna sugestões para alvos (categoria/descrição) que aparecem em **≥3 dos últimos 4 meses** com **variação pequena de valor**. Limiar concreto: **coeficiente de variação (desvio-padrão / média) ≤ 0,15**; como guarda adicional aceita-se também "todos os valores dentro de ±15% da mediana". Sugestão é **opt-in**: o app sugere, nunca marca sozinho. Dispensas ficam em `dismissed_recurring_suggestions (target TEXT PRIMARY KEY)` e são **filtradas** das sugestões futuras.

**Justificativa**: "≥3/4 meses" (FR-010, US4) equilibra sensibilidade e ruído. CV ≤ 0,15 é um limite objetivo e determinístico, adequado a assinaturas/aluguel; o critério ±15% da mediana cobre séries curtas onde a média é enviesada. Persistir dispensas cumpre FR-011 ("não reaparece para o mesmo item"). O alvo é uma string estável (nome da categoria ou descrição normalizada) para servir de PK.

**Alternativas rejeitadas**:
- *Marcar automaticamente*: viola FR-010 (sempre opt-in) e o princípio de não surpreender o usuário.
- *Só contagem de meses (sem limite de valor)*: marcaria gastos variáveis não-fixos (restaurantes recorrentes).
- *Dispensa em memória*: reapareceria a cada abertura do app (viola FR-011).

## D5 — Vigência com granularidade mensal (inclusiva)

**Decisão**: Vigência é `start_month`..`end_month` em `YYYY-MM`, ambos **opcionais** e **inclusivos** (mês a mês). `null/null` = contínua (ongoing); ambos definidos = finita (ex.: psicólogo jan–mar). Uma recorrente finita só conta em meses `m` com `start_month ≤ m ≤ end_month`; após o fim é excluída de **fixas, baseline e Teto**, inclusive em recálculo histórico (determinístico).

**Justificativa**: O app inteiro opera em granularidade de mês (`YYYY-MM`: `manual_entries.month`, `bank_entries.month`, `payslips.month`, `parse_month_start`). Manter a mesma granularidade evita conversões e mantém coerência. Inclusividade nas duas pontas é o que o usuário espera ("por 3 meses" = jan, fev, mar).

**Alternativas rejeitadas**:
- *Número de meses (contador)*: exige âncora e recomputo a cada mês; `start/end` explícitos são idempotentes e triviais de checar em recálculo histórico. (Se a UI oferecer "nº de meses", converte para `end_month` na gravação.)
- *Granularidade por data (dia)*: desnecessária; nenhum outro cálculo do app usa dia para escopo mensal.

## D6 — Fixas derivadas como VIEW sobre importados já contados (evitar dupla contagem no total)

**Decisão**: As contas fixas derivadas são uma **reclassificação/visão** dos lançamentos importados que **já entram** no `net_total` (transações de fatura via `total_card_net`; débitos de extrato via `BankEntry`). A derivação alimenta o **painel Fixos & Renda** e a **base do Teto** (`fixed_month`), e **suprime** o fixo manual equivalente — não soma um valor novo por cima do que o cartão/extrato já contabilizam.

**Justificativa**: Hoje `get_dashboard_cmd`/`get_year_summary_cmd` já convertem `BankEntry → ManualEntry` (avulso) e somam a fatura no cartão. Somar de novo como "fixa" duplicaria. A feature separa dois papéis: (a) **totais de despesa** continuam vindo de cartão + extrato + manuais não-suprimidos; (b) **Teto/projeção** usa `fixed_month` = fixas derivadas realizadas (ou baseline quando não importado) + fixos manuais não-suprimidos (fallback: débito automático, dinheiro). O importado numa categoria recorrente deve ser tratado como **fixo** (não avulso) para o painel Fixos & Renda e para o Teto.

**Alternativas rejeitadas**:
- *Somar a fixa derivada ao total além do cartão/extrato*: dupla contagem, quebra SC-002.
- *Migrar bank entries recorrentes para deixarem de contar no cartão/extrato e recontá-los como fixa*: mais invasivo e arriscado; a visão derivada resolve o mesmo sem mexer na fonte.

## D7 — Migração idempotente na inicialização

**Decisão**: Criar as duas tabelas com `CREATE TABLE IF NOT EXISTS` dentro de `Database::init()` (mesmo `execute_batch` das demais). Sem `ALTER` destrutivo. Reabrir o app N vezes é seguro.

**Justificativa**: Segue exatamente o padrão de `db.rs` (todas as tabelas são idempotentes; `ALTER ... ADD COLUMN` só onde há coluna nova em tabela antiga, tolerando "duplicate column"). Como as tabelas são novas, basta o `CREATE IF NOT EXISTS`.

**Alternativas rejeitadas**:
- *Sistema de versões de schema*: overkill para o escopo atual (YAGNI); o app já depende de criação idempotente.
