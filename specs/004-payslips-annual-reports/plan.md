# Implementation Plan: Contracheque, Visão Anual, Avulsos & Relatórios

**Spec**: [spec.md](spec.md) | **Status**: Delivered | **Date**: 2026-07-21

## Summary

Extensões sobre a base 001–003, entregues incrementalmente na branch `002-modern-dashboard-ui`. Migrou a persistência para SQLite, adicionou import de contracheque SIGEPE, visão anual com filtro, matriz categoria × ano, treemap, separação de despesas avulsas, teto do cartão, relatórios em PDF, drill-down por categoria e CI/CD para gerar instaladores macOS/Windows.

## Technical Context

- **Stack**: Rust (Tauri v2) + Vue 3 / TS. SQLite (`rusqlite`, bundled), `rust_decimal`, `calamine`, `pdf-extract`, `office-crypto` (vendored), `keyring`, `uuid`. Frontend: Pinia, Vue Router, `vue-echarts`.
- **Storage**: `financas.db` em `app_data_dir()`. Senha no keychain do SO.
- **Testes**: `cargo test`, Vitest, Playwright. CI/CD: GitHub Actions.
- **Plataformas**: macOS (universal) e Windows.

## Componentes por área

**Backend**
- `domain/payslip.rs` — parser SIGEPE (regex sobre texto do PDF): salário/bônus (inclui "Cargo de Direção – CD"), wash, líquido por item, `deduction_category`.
- `domain/year.rs` — `compute_year_summary`: cartão por data da compra, fixos/avulsos/descontos por mês, `YearMonthPoint.categories`, teto (2 cenários), anos disponíveis.
- `domain/dashboard.rs` + `application/get_dashboard.rs` — split fixo/avulso/payroll; payslip supersede salário manual.
- `infrastructure/db.rs` — tabelas payslips/payslip_items; migrações; upsert por UUID v5; poda de overrides órfãos; `parse_money` com log.
- `commands/` — `payslips`, `manual_entries` (add/update/remove), `secrets` (senha), `transactions` (list_all).

**Frontend**
- `pages/DashboardPage.vue` — KPIs do contracheque, composição (cartão/fixos/avulsos/descontos), teto, avulsos (CRUD), treemap, drill-down por categoria.
- `pages/YearPage.vue` — filtro ano início/fim + meses, gráfico, teto, matriz categoria × ano (seletor) + gráfico multi-linha + treemap.
- `pages/ContrachequePage.vue`, `pages/ManualEntriesPage.vue`, `pages/MappingPage.vue`.
- `components/report/ReportOverlay.vue` + `assets/report.css` — relatório mês/ano, export PDF via navegador (plugin opener), fallback `window.print()`.
- `components/dashboard/CategoryTreemap.vue` — treemap reutilizável (ECharts).

**CI/CD**
- `.github/workflows/ci.yml` — type-check + Vitest + clippy + cargo test.
- `.github/workflows/release.yml` — `tauri-action` builda mac universal + Windows na tag `vX.Y.Z`.

## Decisões

- **Modelo bruto**: receita = bruto; despesas = cartão + fixos + avulsos + descontos; líquido emerge. Teto usa líquido/recorrente.
- **Fidelidade**: relatório usa os tokens reais do app (`--clr-*`) aliasados no overlay teleportado.
- **PDF**: `window.print()` é no-op no WKWebView → export via HTML standalone aberto no navegador do SO.
- **Fixture de teste**: `sample_fatura.xlsx` sintético é commitado (exceção no `.gitignore`) para o CI rodar os testes do parser.
