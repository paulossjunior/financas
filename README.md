# Finanças

Aplicativo desktop de finanças pessoais — importa faturas do cartão **BTG** e o **contracheque do SouGov.br**, e transforma tudo em um painel do mês, um panorama do ano e relatórios em PDF. Roda 100% local; nenhum dado sai da máquina.

![CI](https://github.com/paulossjunior/financas/actions/workflows/ci.yml/badge.svg)

> Tauri v2 (Rust) + Vue 3 + SQLite. macOS e Windows.

---

## O que faz

**Importação**
- Faturas **BTG** em `.xlsx` (inclusive **cifradas** — senha guardada no keychain do SO).
- **Contracheque SouGov.br** (PDF): extrai e classifica tudo — salário × bônus (inclui "Cargo de Direção – CD" como bônus temporário), descontos (IR, GEAP, FUNPRESP, PSS), detecção de "wash" (adiantamentos que se anulam) e líquido por item. Importa vários de uma vez.

**Painel do mês**
- KPIs: receita, despesa total, saldo, líquido do contracheque, descontos, bônus.
- Composição da despesa: cartão · fixos · **avulsos** · descontos da folha.
- **Teto do cartão** = renda − contas fixas (dois cenários: renda recorrente vs. só salário).
- Lançamentos **avulsos** (débito/crédito pontual, ex.: freelance) com editar/remover.
- **Mapa de gastos (treemap)** e barras por categoria — **clique numa categoria para ver as despesas** (cartão, fixo, avulso, folha).

**Painel do ano**
- Filtro de **ano inicial → final** + intervalo de meses.
- Gráfico receita × despesa, indicadores, teto, ranking.
- **Matriz categoria × ano** que também é seletor (clique na linha) → alimenta um **gráfico multi-linha** (uma linha por categoria + Total) e um treemap do período.

**Categorização**
- Regras por palavra-chave (acento-insensível, raiz), overrides por transação, recategorização no startup.

**Relatórios**
- Relatório do **mês** e do **período** (respeita o filtro) → **exportar para PDF** pelo navegador do sistema.

---

## Instalação

Baixe o instalador da página de [**Releases**](https://github.com/paulossjunior/financas/releases):

| SO | Arquivo |
|----|---------|
| macOS (Intel + Apple Silicon) | `Financas_x.y.z_universal.dmg` |
| Windows | `Financas_x.y.z_x64-setup.exe` (ou `.msi`) |

Os builds **não são assinados**:
- **macOS**: no primeiro open, clique com o botão direito no app → **Abrir** → **Abrir**.
- **Windows**: SmartScreen → **Mais informações** → **Executar assim mesmo**.

---

## Desenvolvimento

Pré-requisitos: **Node 20+**, **Rust estável** e as dependências de sistema do [Tauri v2](https://v2.tauri.app/start/prerequisites/).

```bash
npm install            # dependências do frontend
npm run tauri dev      # roda o app (Vite + Tauri, hot-reload)
```

### Build local

```bash
npm run tauri build    # gera o instalador para o SO atual em src-tauri/target/release/bundle/
```

### Testes e checagens

```bash
npx vue-tsc --noEmit           # type-check TS/Vue
npm run test:run               # testes unitários (Vitest)
npx playwright test            # E2E (Playwright)
cd src-tauri && cargo test     # testes Rust
cd src-tauri && cargo clippy   # lint Rust
```

---

## CI/CD

Dois workflows do GitHub Actions ([`.github/workflows/`](.github/workflows/)):

- **CI** (`ci.yml`) — a cada push/PR: type-check + Vitest (frontend) e clippy + `cargo test` (Rust).
- **Release** (`release.yml`) — ao publicar a tag `vX.Y.Z`: builda **macOS universal** (`.dmg`/`.app`) e **Windows** (`.msi`/`.exe`) via `tauri-action` e cria um GitHub Release em rascunho. Execução manual sobe os instaladores como artifacts.

Lançar uma versão:

```bash
# ajuste a versão em src-tauri/tauri.conf.json, então:
git tag v0.1.0
git push origin v0.1.0
# publique o rascunho:  gh release edit v0.1.0 --draft=false
```

---

## Stack

| Camada | Tecnologia |
|--------|-----------|
| Shell desktop | Tauri v2 |
| Backend | Rust — `rusqlite` (SQLite bundled), `rust_decimal` (dinheiro), `calamine` (xlsx), `pdf-extract` (contracheque), `office-crypto` (xlsx cifrado, vendored), `keyring` (senha no OS), `uuid` |
| Frontend | Vue 3 (`<script setup>`), Pinia, Vue Router, `vue-echarts` (ECharts) |
| Build/test | Vite, `vue-tsc`, Vitest, Playwright |

---

## Estrutura

```
src/                     # frontend Vue
  pages/                 # Dashboard (mês), Year (ano), Transações, Fixos & Renda, Contracheque, Mapeamento, Histórico, Config
  components/            # report/ (relatório PDF), dashboard/ (gráficos, treemap), import/
  stores/                # Pinia (invoice.store, settings.store)
  services/tauri.service.ts   # ponte para os comandos Rust
  types/api.types.ts     # tipos compartilhados com o backend
src-tauri/               # backend Rust (Tauri)
  src/domain/            # regras de negócio (dashboard, year, payslip, categorizer, manual_entry…)
  src/application/       # casos de uso (get_dashboard, import_invoice, recategorize…)
  src/infrastructure/    # db.rs (SQLite), xlsx_parser, btg_mapper
  src/commands/          # comandos #[tauri::command] expostos ao frontend
docs/                    # documentação técnica (ARCHITECTURE.md)
specs/                   # especificações por feature (spec-kit)
```

Detalhes de arquitetura e fluxo de dados em [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

---

## Dados & privacidade

- Tudo é local. **Sem rede.**
- Banco: `financas.db` em `~/Library/Application Support/com.financas.app/` (macOS) / `%APPDATA%\com.financas.app\` (Windows).
- Senha da fatura fica no **keychain/credential manager** do SO, nunca no banco.
- Faturas e contracheques que você deixar em `tests/fixtures/` ou `faturas/` são ignorados pelo git (`.gitignore`).
