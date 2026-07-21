# Feature Specification: Contracheque, Visão Anual, Avulsos & Relatórios

**Feature Branch**: `002-modern-dashboard-ui` (entregue de forma incremental)

**Created**: 2026-07-21

**Status**: Delivered

**Input**: Conjunto de pedidos do usuário após a feature 003 — importar contracheque, ver o ano todo, separar despesas avulsas, relatórios em PDF, treemap, matriz categoria × ano, drill-down por categoria, e empacotar o app para macOS/Windows.

> Spec retroativo: documenta o que foi construído sobre a base 001–003. Detalhes técnicos em [plan.md](plan.md) e [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md).

## User Scenarios & Testing

### User Story 1 — Importar contracheque (SouGov.br) (P1)

Usuário importa o PDF do contracheque; o app extrai e classifica tudo: salário × bônus (inclui "Cargo de Direção – CD" como bônus temporário), descontos (IR, GEAP, FUNPRESP, PSS) e detecta adiantamentos que se anulam ("wash"). O líquido do mês passa a alimentar o painel.

**Acceptance**:
1. Dado um PDF do SouGov.br, ao importar, o líquido, salário, bônus e descontos aparecem no painel do mês.
2. Importar o mesmo mês de novo faz upsert (não duplica).
3. Vários contracheques podem ser importados de uma vez.
4. O salário do contracheque **substitui** a renda-salário manual do mês (sem dupla contagem); renda extra manual (ex.: bolsa) continua somando.

### User Story 2 — Visão anual com filtro (P1)

Usuário vê o ano todo: gráfico receita × despesa, indicadores, teto do cartão e ranking, filtrando por **ano inicial → final** e intervalo de meses.

**Acceptance**:
1. O filtro de anos define o período de todos os cálculos e do gráfico.
2. O cartão é agrupado por **data da compra** (parcelas espalham nos meses corretos).

### User Story 3 — Matriz categoria × ano + evolução (P2)

Na tela Ano, uma **matriz categoria × ano** que é também o seletor: clicar na linha marca/desmarca a categoria. A seleção alimenta um **gráfico multi-linha** (uma linha por categoria + uma linha de Total) e um **treemap** do período.

**Acceptance**:
1. Célula = gasto da categoria no ano (heatmap por intensidade); rodapé com Total das selecionadas.
2. Sem seleção = todas as categorias; com seleção, gráfico e KPIs recalculam só com as marcadas.

### User Story 4 — Despesas avulsas separadas dos fixos (P1)

Lançamentos pontuais (débito/crédito, ex.: freelance, uma conta que apareceu) não entram nas contas fixas. Podem ser editados/removidos no painel do mês.

**Acceptance**:
1. Avulso não conta em "Contas fixas"; aparece em "Lançamentos avulsos" e na composição como fatia própria.
2. Editar/remover um avulso atualiza os totais.

### User Story 5 — Relatório em PDF (P2)

Usuário gera um relatório do mês e do período (respeitando o filtro) e exporta para PDF.

**Acceptance**:
1. O relatório abre em tela cheia e exporta para PDF de forma confiável em macOS e Windows.
2. O relatório do período reflete exatamente o filtro (anos + intervalo de meses).

### User Story 6 — Drill-down por categoria (P2)

No painel do mês, clicar numa categoria em "Gasto por categoria (casa completa)" mostra a lista das despesas dela — cartão, fixo, avulso e folha — com origem, data e valor.

**Acceptance**:
1. Cada item mostra a origem; total confere com a categoria quando um mês está selecionado.
2. Em "Todos os meses"/média, o app orienta a selecionar um mês (a barra é agregada).

### User Story 7 — Distribuir o app (P1)

Instaladores para macOS e Windows gerados automaticamente.

**Acceptance**:
1. Publicar a tag `vX.Y.Z` gera `.dmg`/`.app` (mac universal) e `.msi`/`.exe` (Windows) num GitHub Release.
2. Push/PR roda type-check, testes unitários e `cargo test`/clippy.

## Requisitos Funcionais

- **FR-001**: Parsear e classificar contracheque SouGov.br (salário/bônus/descontos/wash/líquido por item), com upsert por mês.
- **FR-002**: Resumo anual por intervalo de anos + meses; cartão por data da compra; `categories` por mês.
- **FR-003**: Matriz categoria × ano como seletor + gráfico multi-linha + treemap.
- **FR-004**: Separar despesas recorrentes (fixas) de avulsas em todos os totais (mês e ano); CRUD de avulsos no painel do mês.
- **FR-005**: Teto do cartão em dois cenários (renda recorrente vs. só salário).
- **FR-006**: Treemap de categorias no mês, no ano e no relatório.
- **FR-007**: Drill-down: despesas de uma categoria por origem (cartão/fixo/avulso/folha).
- **FR-008**: Relatório do mês e do período, exportável em PDF de forma confiável (mac/win).
- **FR-009**: Robustez: logar (não engolir) valores decimais/meses corrompidos.
- **FR-010**: CI (testes) + Release (instaladores mac/win) no GitHub Actions.

## Key Entities

- **Payslip / PayslipItem**: contracheque do mês e seus itens (rendimento/desconto, classe salário/bônus/wash, líquido por item, `offsetting`).
- **ManualEntry**: lançamento manual — `kind` (income/expense), `recurring` (fixo vs avulso), `is_salary`, `month`, `category`.
- **YearMonthPoint**: um mês do resumo anual — income/card/fixed/variable/payroll/expense/balance + `categories` (despesa por categoria).
- **DashboardData / YearSummary**: DTOs agregados do mês e do período.
