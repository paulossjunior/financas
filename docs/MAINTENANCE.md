# Guia de manutenção — Finanças

Guia prático para manter e evoluir o app. Complementa a
[ARCHITECTURE.md](ARCHITECTURE.md) (camadas e fluxos) com **invariantes**, **receitas
de tarefas comuns** e **armadilhas**. Leia isto antes de mexer no código.

## Visão rápida

Desktop Tauri v2 — backend **Rust** + frontend **Vue 3** — 100% local, SQLite como
fonte única de verdade. Sem rede (única exceção: fetch opt-in do IBGE).

```
src-tauri/src/           backend Rust
  domain/                regras puras (sem Tauri/DB) — testáveis
  application/           casos de uso (orquestram domain)
  commands/              fronteira #[tauri::command]
  infrastructure/        I/O: db.rs (SQLite), xlsx_parser, btg_mapper, config_store, ibge
  lib.rs                 setup do app, migração/recategorização no startup, registro de comandos
src/                     frontend Vue
  pages/                 uma por rota
  components/            gráficos/cards/editores
  stores/                Pinia (invoice.store, settings.store)
  services/tauri.service.ts   ÚNICO ponto que chama invoke()
  types/api.types.ts     espelha os DTOs do Rust
  utils/                 helpers puros (money.ts, category-conflict.ts, inflation-explainer.ts)
specs/                   spec-kit por feature (001–011)
docs/                    ARCHITECTURE.md, MANUAL.md, este guia
.claude/skills/          skills do projeto (nielsen-heuristics)
```

## Invariantes (NÃO quebre)

1. **Dinheiro = `rust_decimal::Decimal`**, serializado como **string**. No front, `parseFloat`
   só para exibir/gráfico — nunca para persistir. Taxas de inflação são `f64` (razões).
2. **Regra de dependência**: `commands → application → domain`; `infrastructure` faz I/O.
   `domain` **não** conhece Tauri nem SQLite (mantém puro e testável).
3. **`services/tauri.service.ts` é o único lugar que chama `invoke`.** Páginas/stores usam os
   wrappers de lá. Tipos em `types/api.types.ts` espelham os DTOs Rust (campos snake_case).
4. **SQLite é a fonte de verdade** (`financas.db` em `app_data_dir`). IDs determinísticos
   (`Uuid::new_v5`) → reimportar faz **upsert**, não duplica.
5. **Determinismo**: agregações (dashboard, ano, forecast, inflação, recorrentes) devem dar o
   mesmo resultado para a mesma entrada. Sem `Date::now`/random no meio do cálculo.
6. **Migrações idempotentes** no startup (`CREATE TABLE IF NOT EXISTS`, `ALTER ... ADD COLUMN`
   ignorando "duplicate column"). Ver `db.rs::init`.
7. **Categorização roda em cartão + extrato** (crédito e débito), keyword vence, fallback BTG
   preservado, edição manual (`bank_entries.user_categorized`) respeitada.
8. **Máscara de dinheiro**: inputs de valor usam `src/utils/money.ts` (`maskMoney`/`parseMoneyBR`).
9. **UI/UX**: ao mexer em tela/fluxo/erro, aplique a skill `nielsen-heuristics`
   (`.claude/skills/nielsen-heuristics/SKILL.md`).
10. **Saldo/cobertura (016)**: posição é identificada por `(banco, conta, produto, as_of)` —
    UUIDv5 determinístico, então reimportar substitui em vez de duplicar; a **posição
    corrente** é sempre a de maior `as_of` (nunca "a última importada"). `clear_bank_entries`
    limpa `bank_entries` + `account_positions` + `statement_coverage` juntos (nada de saldo
    órfão). Na `Conferencia` do Banestes, `segmentos: SemDados` é **tolerado** de propósito
    (extrato pode não imprimir saldos parciais); `saldos` e `entradas_saidas` continuam
    bloqueando por ausência.
11. **Chave de id do extrato é congelada**: a **primeira** ocorrência de um lançamento usa
    `bank:{conta}:{data}:{desc normalizada}:{valor}` (ver `bank_statement::entry_key`).
    Mudar essa string faz *todo* lançamento já gravado reimportar como novo. Repetições
    idênticas no mesmo arquivo recebem sufixo `#n` (`entry_ids`) — nunca colapse duas em uma.

## Comandos

```bash
npm run tauri dev                 # app com hot-reload
npm run tauri build               # instalador do SO atual (bundle/)
npx vue-tsc --noEmit              # type-check TS/Vue
npm run test:run                  # Vitest (frontend)
npx playwright test               # E2E
cd src-tauri && cargo test        # testes Rust
cd src-tauri && cargo clippy -p financas --all-targets -- -D warnings   # lint (CI usa isso)
```

Antes de commitar: **vue-tsc + vitest + cargo test + clippy** verdes.

## Banco de dados (`infrastructure/db.rs`)

Tabelas: `invoices`, `transactions`, `settings`, `category_rules`, `categories`,
`transaction_overrides`, `manual_entries`, `payslips`, `payslip_items`, `inflation_cache`,
`bank_entries` (com `user_categorized`), `recurring_categories` (com `base_amount`),
`dismissed_recurring_suggestions`.

- `save_config`/`load_config` reescrevem regras/overrides/manual/settings/categories.
- Categorias sem keyword persistem em `categories` (senão sumiam ao recarregar).
- `parse_money()` loga valores decimais corrompidos em vez de virar 0 silencioso.

## Receitas (tarefas comuns)

### Adicionar um comando Tauri
1. Escreva a lógica no `domain`/`application` (pura, com testes).
2. `#[tauri::command]` em `commands/<área>.rs` (recebe `State`, chama application, retorna DTO).
3. Registre em `lib.rs`: no `use commands::{ ... }` e no `tauri::generate_handler![ ... ]`.
4. Wrapper em `services/tauri.service.ts` + tipo em `types/api.types.ts` (Tauri converte
   camelCase↔snake_case nos args).

### Adicionar uma página/rota
1. `src/pages/XPage.vue`. 2. Rota em `src/router/index.ts`. 3. Item de nav em `src/App.vue`
   (ícones do menu são SVG de linha herdando `currentColor`).

### Adicionar/alterar tabela (migração)
Em `db.rs::init`: `CREATE TABLE IF NOT EXISTS ...` no batch; para coluna nova em tabela
existente, `ALTER TABLE ... ADD COLUMN ...` tratando "duplicate column" como ok. Atualize
`save_config`/`load_config` se for config.

### Novo módulo de domínio (TDD)
Crie `domain/<x>.rs` puro, escreva os testes primeiro (`#[cfg(test)]`), registre em
`domain/mod.rs` (`pub mod x;` + re-export se precisar). Cobertura ≥90% em lógica de núcleo.

### Adicionar um novo banco de extrato (padrão strategy — feature 014)
1. `domain/<banco>_statement.rs` **puro**: uma classe `Extrato<Banco>` no molde de
   `ExtratoBanestes` — `parse(&str) -> Result<Self>` (estrutura tipada: metadados, saldos,
   totais declarados, movimentos), `conferir() -> Conferencia` quando o extrato imprime
   totais, `into_parsed() -> ParsedStatement` preenchendo `bank`. Detecção
   `is_<banco>_statement(&str) -> bool` por marcadores estruturais (o nome do banco costuma
   ser imagem e não aparece no texto). Teste com fixture de **texto anonimizado** em
   `tests/fixtures/` — extrato real nunca vai para o repositório.
2. `infrastructure/<banco>_statement.rs`: casca de I/O (arquivo → texto/linhas) + o
   **strategy**: `pub struct <Banco>StatementReader;` implementando
   `statement_reader::StatementReader` (`bank`, `extensions`, `recognizes`, `read`).
3. Registre: uma linha em `statement_reader.rs::STATEMENT_READERS`. **Não há mais despacho
   manual** — `commands/bank.rs` e `application/import_folder.rs` consultam o registro; a
   mensagem de "formato não suportado" é gerada do registro (`supported_formats`).
4. Reuse `classify_statement` (fatura/salário/interno + categorização) — **não** crie uma
   segunda lógica de exclusão. Se a grafia do banco não casar, generalize o keyword existente.
5. Conferência é **estrita**: `Divergiu` e `SemDados` recusam a importação
   (`Conferencia::exigir`). Extrato sem totais impressos = extração mudou de forma = erro
   visível, nunca importação sem rede. Num consolidado, confira pelo saldo **da conta**
   (`Saldo Conta`), não pelo `Saldo Total` (que soma poupança/investimento).
6. Frontend: acrescente a extensão no filtro de `ExtratoPage.vue`; a coluna "Origem" já mostra
   `BankEntry.bank`.

### Adicionar um novo banco de fatura de cartão (exemplo real: Santander, feature 015)
Mesmo padrão do extrato: classe pura no domínio + strategy na infraestrutura.
1. `domain/<banco>_invoice.rs`: classe `Fatura<Banco>` (`parse(&str)` → struct tipada com os
   totais declarados; `conferir()` estrito quando a fatura imprime um resumo;
   `into_transactions(invoice_id, categorizer)` com `row_index` sequencial → ids
   determinísticos). Fixture de texto anonimizado em `tests/fixtures/`.
2. `infrastructure/<banco>_invoice.rs`: I/O (decifragem se preciso) + `impl InvoiceReader`
   (`bank`, `extensions`, `read` → `InvoiceRead{transactions, warnings, reference_month}` —
   devolva `Some(mês)` se o nome do arquivo não seguir o padrão BTG `YYYY-MM-…`).
3. Registre em `INVOICE_READERS`. `application/import_invoice.rs` é genérico: escolhe o
   strategy pela extensão e carimba `Invoice.bank = reader.bank()`. A tabela `invoices` tem
   coluna `bank` (default `'BTG'`) — nada de literal de banco fora do strategy.
4. Senha: uma credencial **por banco** no keychain (`secrets::save/get/clear/has_password_for`);
   BTG fica na credencial legada `invoice-password`. O comando `import_invoices` resolve a
   senha efetiva pelo banco do arquivo e o `remember` grava na chave certa.
5. Pasta automática: a política do `.pdf` vive em `import_folder::pdf_route` (pura, testada
   em tabela) — extrato reconhecido → extrato; PDF **cifrado** → candidato a fatura (senha
   salva importa; sem senha → `ENCRYPTED_NO_PASSWORD`); resto (contracheque) → silêncio.

### Input de dinheiro
`inputmode="numeric"` + `@input="campo = maskMoney(campo)"`; no submit `parseMoneyBR(campo)`.
Prefill de edição: formate com `toLocaleString("pt-BR",{minimumFractionDigits:2})`.

### Backup/restauração da base (feature 012)
Comandos `backup_database`/`restore_database` (`commands/backup.rs`) sobre
`Database` (`infrastructure/db.rs`):
- **Backup**: `backup_to(dir)` usa `VACUUM INTO` (snapshot consistente com a conexão
  aberta) → `financas-backup-<YYYYMMDD-HHMMSS>.db` (sufixo `-N` se colidir; nunca sobrescreve).
- **Restauração**: `validate_backup(path)` (integrity_check + tabelas centrais) → grava
  cópia de segurança `financas-pre-restore-<ts>.db` ao lado de `financas.db` → fecha a
  conexão, `fs::copy` o arquivo, reabre + `init()` (migra esquema antigo) → retorna o
  caminho da cópia de segurança. O comando recarrega o estado em memória (`Mutex<AppConfig>`
  + `SharedStore` via `replace_all`) e o front faz `window.location.reload()`.
- Diálogos: pasta via `open({ directory:true })`, arquivo via `open({ filters:[...] })`
  (`dialog:allow-open` já cobre ambos). UI na tela Configurações; restauração pede `ask()`.

### Pasta de importação automática (feature 013)
Setting `import_directory` (`Option<String>` no `AppConfig`; vazio = desligado).
`application/import_folder.rs::import_from_folder(dir, db, store, cfg, senha)`:
- **Detecção de tipo**: `.xls` → extrato BTG; `.xlsx` → tenta fatura (`import_invoice`),
  se `INVALID_FORMAT`/`PARSE_ERROR` cai para extrato (`read_statement`); `.pdf` → só entra
  se `is_banestes_statement` reconhecer o texto.
- **O que vai para `ignored`**: planilha que nenhum parser aceita e fatura cifrada sem senha
  salva — o usuário pôs ali esperando importar, então silêncio esconderia falha. **PDF que
  não é extrato é pulado em silêncio**: contracheque mora legitimamente nessa pasta e a
  varredura roda a cada abertura do app — listá-lo como "ignorado" sempre pareceria erro.
  PDF já reconhecido como extrato que falha ao importar **é** reportado (`ERROR: …`).
  Nada aborta a varredura.
- **Dedup** herdado: fatura por `invoice_id` (nome) + `store.add`; extrato por `BankEntry.id`.
  Classificação de extrato compartilhada via `domain::bank_statement::classify_statement`.
- **Gatilhos**: `set_import_directory` (definir/limpar + importar na hora, retorna
  `FolderImportSummary`); no boot (`lib.rs setup`) se a pasta existe, roda e guarda o
  resumo em `Mutex<Option<FolderImportSummary>>`. `get_startup_import_summary` lê+limpa
  (App.vue mostra toast). Pasta ausente/ilegível não trava — vira item de erro no resumo.

## Mapa de features (spec-kit)

| Spec | O que |
|------|------|
| 001 | Dashboard do cartão (mês) |
| 002 | Lista mensal de faturas |
| 003 | Categorias customizadas (keyword rules) |
| 004 | Contracheque (SouGov) + ano + relatórios PDF |
| 005 | Previsão de pagamento do cartão (parcelas) |
| 006 | Inflação (IPCA + pessoal), fetch IBGE opt-in, cache |
| 007 | Explicador da inflação (para leigo) |
| 008 | Extrato bancário (.xls): crédito/débito, exclusão automática, dedup |
| 010 | Categorias recorrentes + baseline + vigência + anti-duplicação; categorização unificada cartão+extrato |
| 011 | Cálculo rigoroso de inflação pessoal (contribuições, cesta, renda, comportamental) |
| 012 | Backup e restauração do banco |
| 013 | Pasta de importação automática |
| 014 | Extrato Banestes (.pdf): adapter por banco, entradas/saídas conferidas contra os totais do extrato |
| 016 | Saldo em conta + cobertura de dados + conferência por segmento (dados que o extrato já imprimia) |
| 015 | Fatura Santander (.pdf cifrado): 2º InvoiceReader, conferência pelo Resumo da Fatura, senha por banco no keychain |

Cada `specs/NNN-*/` tem spec/plan/research/data-model/contracts/quickstart/tasks.

## CI/CD e release

- **CI** (`.github/workflows/ci.yml`): vue-tsc + Vitest (front) e clippy `-D warnings` + `cargo test` (Rust).
- **Release** (`release.yml`): ao publicar tag `vX.Y.Z`, builda **macOS universal** + **Windows**
  via `tauri-action` e cria release em rascunho.
- **Pages** (`pages.yml`): publica `site/` no push pra `main`.

Lançar versão:
```bash
# bump em package.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml
git tag vX.Y.Z && git push origin vX.Y.Z          # → CI builda
gh run watch <id> --exit-status                    # acompanha
gh release edit vX.Y.Z --draft=false               # publica
# atualize os links de download em site/index.html para vX.Y.Z
```

## Armadilhas conhecidas

- **`window.print()` é instável no WKWebView (macOS)** → o relatório é serializado em HTML e
  aberto no navegador do sistema (plugin `opener`). Ver `components/report/ReportOverlay.vue`.
- **IPCA em percent** (0,63 = 0,63%): ao usar no cálculo decimal (011), divida por 100.
- **`priority` de categoria é `u32`** (era `u8` e estourava com muitas categorias).
- **Nome do titular do extrato pode vir com mojibake** (U+FFFD) → detecção de transferência
  interna casa por tokens limpos. Ver `domain/bank_statement.rs`.
- **Fetch do IBGE é a única rede** e é opt-in (botão "Atualizar índices"); offline depois.
- **Não misture períodos de inflação** (mensal/anual/trimestral) — use os helpers de conversão
  composta em `domain/personal_inflation.rs` (nunca dividir anual por 12).
