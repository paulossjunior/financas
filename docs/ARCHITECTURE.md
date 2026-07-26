# Arquitetura

Desktop app Tauri v2: backend Rust + frontend Vue 3, comunicando por comandos `#[tauri::command]`. SQLite é a fonte única de verdade.

## Camadas (backend Rust — `src-tauri/src/`)

```
commands/         Fronteira Tauri. Recebem State + args, chamam application/, retornam DTOs (String p/ dinheiro).
application/      Casos de uso: get_dashboard, import_invoice, recategorize, store (InvoiceStore em memória).
domain/           Regras de negócio puras, testáveis: dashboard, year, payslip, categorizer, manual_entry, category, invoice, transaction.
infrastructure/   I/O: db.rs (SQLite via rusqlite), xlsx_parser (calamine + office-crypto), btg_mapper, config_store.
```

Regra de dependência: `commands → application → domain`; `infrastructure` implementa I/O. `domain` não conhece Tauri nem SQLite.

## Frontend (`src/`)

- `pages/` — uma por rota (`/` mês, `/ano`, `/transacoes`, `/receitas-fixos`, `/contracheque`, `/mapeamento`, `/historico`, `/configuracoes`).
- `stores/` — Pinia. `invoice.store` guarda invoices, `allTransactions`, `manualEntries`, `dashboard`, `monthFilter`.
- `services/tauri.service.ts` — único ponto que chama `invoke(...)`.
- `types/api.types.ts` — espelha os DTOs do Rust (campos de dinheiro são `string`).

## Modelo de dinheiro

- Backend usa `rust_decimal::Decimal`; serializa como **string** (`serde-str`) → frontend faz `parseFloat` só para exibir/gráfico, nunca para persistir.
- **Modelo bruto**: receita = bruto do contracheque + renda extra manual. Despesa = cartão + fixos (recorrentes) + avulsos (pontuais) + descontos da folha. O líquido "na mão" emerge de receita − despesas.
- **Teto do cartão** = renda recorrente − contas fixas (dois cenários: toda renda recorrente vs. só salário permanente).

## Persistência (`infrastructure/db.rs`)

SQLite em `app_data_dir()/financas.db`. Tabelas: `invoices`, `transactions`, `manual_entries`, `payslips`, `payslip_items`, `category_rules`, `transaction_overrides`, `settings`.

- IDs determinísticos (`Uuid::new_v5`) para invoices e payslips → reimportar o mesmo mês/arquivo faz **upsert** em vez de duplicar.
- Migrações idempotentes no startup (ex.: `ALTER manual_entries ADD is_salary`).
- No startup: recategoriza tudo (overrides ganham das regras) e **poda overrides órfãos** (transações que não existem mais).
- `parse_money()` loga valores decimais corrompidos em vez de lê-los como 0 silenciosamente.

## Fluxos principais

**Importar fatura BTG** — `import.rs` → `import_invoice` → `xlsx_parser` (detecta cifrado → pede senha, guarda no keychain) → `btg_mapper` (linhas → `Transaction`, parcelas, estornos) → `categorizer` → upsert no DB.

**Importar contracheque** — `payslips.rs` → `payslip::parse_payslip_text` (regex sobre `pdf-extract`): classifica salário/bônus/wash, calcula líquido por item e `deduction_category` (IR→Impostos, GEAP→Saúde, FUNPRESP/PSS→Previdência). Salvo em `payslips`/`payslip_items`.

**Dashboard do mês** — `get_dashboard.rs`: expande manual entries no escopo (recorrentes contam todo mês; avulsos só no seu mês), o **payslip supersede o salário manual** do mês (evita duplicidade), descontos viram despesas categorizadas. Retorna `DashboardData` com `total_card_net`, `total_manual_expense` (fixos), `total_variable_expense` (avulsos), `total_payroll_deductions`, categorias, parcelas, assinaturas.

**Resumo do ano** — `domain/year.rs`: cartão agrupado por **data da transação** (parcelas espalham no ano), fixos/avulsos/descontos por mês, líquido do contracheque por mês. `YearMonthPoint` traz `categories` (despesa por categoria naquele mês) → o frontend deriva a **matriz categoria × ano**, o **gráfico multi-linha** por seleção e o **treemap** do período.

**Relatório PDF** — `components/report/ReportOverlay.vue`: como `window.print()` é instável no WKWebView (macOS), o relatório é serializado em um HTML standalone (canvases ECharts → PNG, `report.css` + tokens embutidos), escrito em AppData e aberto no **navegador do sistema** (plugin `opener`), onde imprimir → PDF é confiável. Fallback: `window.print()`.

## Comandos expostos (`lib.rs`)

`import_invoices`, `get_dashboard_cmd`, `get_year_summary_cmd`, `list_invoices`, `remove_invoice`, `get_config`, `save_config`, `recategorize_invoices_cmd`, `add_category_keyword`, `override_transaction_category`, `remove_transaction_override`, `list_all_transactions`, `list_manual_entries`, `add_manual_entry`, `update_manual_entry`, `remove_manual_entry`, `has_saved_password`, `clear_saved_password`, `import_payslip`, `save_payslip`, `list_payslips`, `remove_payslip`.

Plugins: `fs`, `dialog`, `store`, `opener`. Permissões em `src-tauri/capabilities/default.json`.

## Testes

- Rust: unitários por módulo em `domain/`/`application/` + integração em `src-tauri/tests/` (usa `tests/fixtures/sample_fatura.xlsx`, sintético, commitado).
- Frontend: Vitest (stores/componentes) + Playwright (E2E).

## Módulos por feature (specs 004–008)

- **004 Contracheque + Ano + Relatórios**: `domain/payslip.rs`, `domain/year.rs`, `components/report/ReportOverlay.vue`.
- **005 Previsão do cartão**: `domain/forecast.rs` (`compute_card_forecast`, dedup por compra, âncora determinística) → `YearSummary.card_forecast` + resumo no `DashboardData`; `components/dashboard/CardForecastChart.vue`.
- **006 Inflação (IPCA + pessoal)**: `domain/inflation.rs` (puro), `infrastructure/ibge.rs` (fetch opt-in via reqwest/rustls, agregados 1737+7060), cache `inflation_cache` no SQLite, `commands/inflation.rs`; `components/dashboard/InflationCard.vue`.
- **007 Explicador da inflação**: `src/utils/inflation-explainer.ts` (puro, testado) + `components/dashboard/InflationExplainer.vue`.
- **008 Extrato bancário**: `infrastructure/btg_statement.rs` (calamine .xls → linhas), `domain/bank_statement.rs` (parse + classificação: exclui fatura/salário-com-contracheque/transferências internas; categoriza app+fallback banco; dedup por UUIDv5), tabela `bank_entries`, `commands/bank.rs`; incluídos entram no pipeline como `ManualEntry` (avulso/renda). UI: `ExtratoPage` dentro de `ImportsPage` (📥 Importações) + dashboard `MovimentacoesPage` (Extratos & Faturas).
- **014 Extrato Banestes (PDF) + strategy por banco**: leitura de documentos é polimórfica — um **strategy** por banco, atrás de dois traits de infraestrutura: `statement_reader.rs::StatementReader` (impls `BanestesStatementReader` `.pdf`, `BtgStatementReader` `.xls`/`.xlsx`; registro `STATEMENT_READERS` + despacho por extensão em `statement_reader_for`) e `invoice_reader.rs::InvoiceReader` (impl `BtgInvoiceReader` `.xlsx`). `commands/bank.rs` e `application/import_folder.rs` só falam com o registro; `application/import_invoice.rs` idem e carimba `Invoice.bank = reader.bank()`. Persistência **genérica por banco**: `bank_entries.bank` e `invoices.bank` (TEXT, default `'BTG'` na migração) aceitam qualquer banco — nenhum literal de banco em call site. O domínio Banestes é uma classe: `domain/banestes_statement.rs::ExtratoBanestes` (`parse` → struct tipada com agência/conta/titular/saldos/totais declarados; `conferir` → `Conferencia { saldos, entradas_saidas }` com `Checagem::{Fechou, Divergiu, SemDados}`; `into_parsed` → `ParsedStatement` compartilhado). Política estrita: `Divergiu` **ou** `SemDados` ⇒ erro, nada gravado (extrato que não permite conferência é recusado, não importado sem rede). Consolidado: a conferência usa `Saldo Conta` (o `Saldo Total` soma poupança/investimento). Banestes não informa categoria (sem fallback do banco → "Outros" cai na fila de mapeamento).

Rede: única chamada externa é o fetch do IBGE (opt-in). Tudo mais é local.

## Feature 010 — Categorias recorrentes + baseline + anti-duplicação

- **Domínio** `domain/recurring.rs` (puro, testado): `RecurringCategory` (com vigência `start_month`/`end_month` e `base_amount` opcional), `Observation`/`DerivedFixed` (carregam `kind` crédito/débito), `derive_month` (realizado > valor base do usuário > média), `baseline` (média dos últimos 3 meses), `is_manual_superseded` (anti-duplicação, mesmo padrão payslip→salário) e `detect_suggestions` (recorrência provável, opt-in). O sinal (renda vs despesa) é **inferido do dado**.
- **Aplicação** `application/recurring_fixed.rs`: monta `Observation`s do cartão (Fatura) + extrato (Extrato, crédito e débito), deriva contas fixas/rendas por mês, calcula baseline/origem/varia por categoria e as sugestões.
- **Infra** `db.rs`: tabelas `recurring_categories` (+`base_amount`), `dismissed_recurring_suggestions`, `categories` (persiste nomes sem keyword), flag `bank_entries.user_categorized`; `recategorize_bank_entries` aplica as regras de keyword ao extrato (keyword vence, fallback BTG preservado, override manual respeitado).
- **Comandos** `commands/recurring.rs`: `list_recurring_categories`, `set_category_recurring`, `set_recurring_base`, `list_all_categories`, `recurring_suggestions`, `dismiss_recurring_suggestion`, `list_fixed_expenses`.
- **Integração**: `get_dashboard`/`year` reclassificam gasto de categoria recorrente do extrato como conta fixa e suprimem o fixo manual equivalente (anti-dup). Categorização unificada: keyword roda em **cartão + extrato** (recategorize + startup + ao criar keyword).
- **UI**: nova página **🗂️ Categorias** (abas *Categorias & Regras* — Recorrente/vigência/valor base/origem + sugestões + modal de nova categoria + confirmação de exclusão — e *Mapeamento de despesas* — fila do "Outros" cartão+extrato, realce "recém-saído"). **Fixos & Renda** mostra as fixas/rendas derivadas. Nav "Despesas & Receitas".
- **Skill** `.claude/skills/nielsen-heuristics/` para revisão de UI/UX.
