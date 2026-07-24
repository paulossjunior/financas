---

description: "Task list — Backup e Restauração da Base de Dados"
---

# Tasks: Backup e Restauração da Base de Dados

**Input**: Design documents from `/specs/012-db-backup-restore/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: INCLUÍDOS — a Constituição (Princípio I, TDD NON-NEGOTIABLE) exige testes escritos
antes da implementação. Testes Rust em `src-tauri` cobrem a lógica de backup/validação/restauração.

**Organization**: Tarefas agrupadas por user story (US1 backup P1, US2 restauração P2).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: paralelizável (arquivos distintos, sem dependência pendente)
- Caminhos de arquivo explícitos em cada tarefa

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Base para os comandos de backup/restauração.

- [X] T001 Verificar capabilities de diálogo em `src-tauri/capabilities/default.json`: confirmar que `dialog:allow-open` cobre seleção de pasta (`directory:true`) e de arquivo; nenhuma permission nova deve ser necessária (documentar no PR se for).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Pré-requisitos que TODAS as stories usam. Concluir antes de US1/US2.

- [X] T002 Adicionar campo `path: std::path::PathBuf` a `struct Database` em `src-tauri/src/infrastructure/db.rs`; preencher em `Database::open(path)` (guardar `path.to_path_buf()`); ajustar `open_in_memory` (test) para `PathBuf::new()`.
- [X] T003 Adicionar método `InvoiceStore::replace_all(&mut self, invoices: Vec<Invoice>)` em `src-tauri/src/application/store.rs` que limpa e repovoa o mapa (recarga pós-restauração).
- [X] T004 [P] Criar DTOs `BackupResult { path: String }` e `RestoreResult { backup_of_previous: String }` (serde `rename_all = "camelCase"`) em `src-tauri/src/commands/backup.rs` (arquivo novo); declarar `pub mod backup;` em `src-tauri/src/commands/mod.rs`.
- [X] T005 [P] Adicionar tipos `BackupResult` e `RestoreResult` em `src/types/api.types.ts` espelhando os DTOs Rust.

---

## Phase 3: User Story 1 — Exportar backup (Priority: P1) 🎯 MVP

**Goal**: Usuário gera, pela tela de Configurações, um backup completo da base numa pasta escolhida, com timestamp no nome e sem sobrescrever backups anteriores.

**Independent Test**: Escolher uma pasta e acionar o backup; conferir arquivo `.db` com timestamp cujo conteúdo abre e reflete os dados atuais.

### Tests (escrever primeiro — devem falhar)

- [X] T006 [US1] Teste em `src-tauri/src/infrastructure/db.rs` (`#[cfg(test)]`): `backup_to(dir)` grava um arquivo cujo caminho existe, é uma base SQLite abrível e contém os mesmos dados (roundtrip de invoices via `Database::open` no arquivo gerado).
- [X] T007 [US1] Teste em `db.rs`: dois `backup_to` na mesma pasta produzem caminhos distintos (sem colisão), incluindo o caso de mesmo timestamp (sufixo `-N`).

### Implementation

- [X] T008 [US1] Implementar `Database::backup_to(&self, dest_dir: &Path) -> Result<PathBuf, String>` em `db.rs`: validar que `dest_dir` é diretório; gerar nome `financas-backup-<YYYYMMDD-HHMMSS>.db` (chrono `Local::now`), aplicar sufixo `-N` se existir; executar `VACUUM INTO ?1`; retornar o caminho. Erros → `BACKUP_DIR_INVALID` / `BACKUP_FAILED: <detalhe>`.
- [X] T009 [US1] Implementar comando `backup_database(dest_dir, db: State<SharedDb>) -> Result<BackupResult, String>` em `src-tauri/src/commands/backup.rs` (locka `db`, chama `backup_to`, mapeia para `BackupResult`).
- [X] T010 [US1] Registrar `backup_database` no `invoke_handler` e no `use commands::backup::...` em `src-tauri/src/lib.rs`.
- [X] T011 [US1] Adicionar wrapper `backupDatabase(destDir: string): Promise<BackupResult>` em `src/services/tauri.service.ts` (invoke `"backup_database"`, `{ destDir }`, com `mapError`).
- [X] T012 [US1] Adicionar à `src/pages/SettingsPage.vue` a seção "Backup e restauração" com botão **Fazer backup**: abrir `open({ directory: true })`; se cancelado, não fazer nada; senão chamar `backupDatabase` e exibir mensagem de sucesso com o caminho retornado. Aplicar a skill `nielsen-heuristics` (visibilidade de status, prevenção de erro, mensagens claras).

**Checkpoint**: US1 entregue e testável isoladamente (MVP).

---

## Phase 4: User Story 2 — Restaurar/importar backup (Priority: P2)

**Goal**: Usuário restaura a base a partir de um arquivo de backup, com confirmação, validação, cópia de segurança automática da base atual e recarga do app.

**Independent Test**: Com um backup válido, acionar a restauração, confirmar e verificar que o painel reflete os dados do backup e que existe uma cópia `financas-pre-restore-*.db`.

### Tests (escrever primeiro — devem falhar)

- [X] T013 [US2] Teste em `db.rs`: `validate_backup(path)` retorna Ok para uma base gerada pelo app e `Err(INVALID_BACKUP)` para (a) arquivo não-SQLite e (b) SQLite sem as tabelas centrais.
- [X] T014 [US2] Teste em `db.rs`: `restore_from(src)` grava a cópia de segurança da base anterior, troca o arquivo e passa a ler os dados de `src` (roundtrip); a base anterior é recuperável a partir do caminho retornado.

### Implementation

- [X] T015 [US2] Implementar `Database::validate_backup(path: &Path) -> Result<(), String>` em `db.rs`: abrir conexão separada; `PRAGMA integrity_check` == `"ok"`; verificar existência das tabelas `invoices`, `transactions`, `settings` (via `sqlite_master`). Erro → `INVALID_BACKUP` / `FILE_NOT_FOUND`.
- [X] T016 [US2] Implementar `Database::restore_from(&mut self, src: &Path) -> Result<PathBuf, String>` em `db.rs`: (1) `validate_backup(src)`; (2) `VACUUM INTO` cópia de segurança `financas-pre-restore-<ts>.db` ao lado de `self.path`; (3) fechar conexão (mem::replace por `open_in_memory`, drop); (4) `fs::copy(src, &self.path)`; (5) reabrir `Connection::open(&self.path)` + `self.init()`; retornar o caminho da cópia de segurança. Erros pós-cópia → `RESTORE_FAILED: <detalhe>`.
- [X] T017 [US2] Implementar comando `restore_database(source_path, db, config: State<Mutex<AppConfig>>, store: State<SharedStore>) -> Result<RestoreResult, String>` em `commands/backup.rs`: chamar `restore_from`; então recarregar `AppConfig` (`db.load_config()` → sobrescrever o `Mutex<AppConfig>`) e `SharedStore` (`db.load_invoices()` → `store.replace_all(...)`); retornar `RestoreResult`.
- [X] T018 [US2] Registrar `restore_database` no `invoke_handler` e no `use` em `src-tauri/src/lib.rs`.
- [X] T019 [US2] Adicionar wrapper `restoreDatabase(sourcePath: string): Promise<RestoreResult>` em `src/services/tauri.service.ts` (invoke `"restore_database"`, `{ sourcePath }`, com `mapError`; mapear `INVALID_BACKUP` para mensagem amigável).
- [X] T020 [US2] Na `src/pages/SettingsPage.vue`, botão **Restaurar backup**: `open({ filters:[{name:'Backup',extensions:['db']}] })`; se selecionado, `ask(...)` de confirmação avisando que os dados atuais serão substituídos; ao confirmar, chamar `restoreDatabase`, exibir sucesso e executar `window.location.reload()`. Tratar/mostrar erro de arquivo inválido sem quebrar a tela. Aplicar `nielsen-heuristics` (confirmação destrutiva, recuperação de erro).

**Checkpoint**: Ciclo backup→restauração completo.

---

## Phase 5: Polish & Cross-Cutting Concerns

- [X] T021 [P] Adicionar códigos `INVALID_BACKUP` e demais ao mapa `ERROR_MESSAGES` em `src/services/tauri.service.ts` com texto claro em pt-BR.
- [X] T022 [P] Rodar `cd src-tauri && cargo test`, `npm run test:run` e `npx vue-tsc --noEmit`; garantir verde.
- [ ] T023 [P] (pendente — validação manual pelo usuário) Validar pela tela Configurações seguindo `specs/012-db-backup-restore/quickstart.md` (backup, restauração, cancelamentos, arquivo inválido).
- [X] T024 [P] Atualizar `docs/MAINTENANCE.md` com receita "Backup/restauração da base" (nomes dos arquivos, cópia de segurança pré-restauração, comandos).

---

## Dependencies & Execution Order

- **Setup (T001)** → **Foundational (T002–T005)** → **US1 (T006–T012)** → **US2 (T013–T020)** → **Polish (T021–T024)**.
- US2 depende de Foundational (T002 `path`, T003 `replace_all`) e conceitualmente do backup existir (US1), mas o código de US2 não importa símbolos de US1 — pode ser implementado após Foundational.
- Dentro de cada story: **testes antes da implementação** (TDD).

## Parallel Opportunities

- T004 e T005 (DTOs Rust vs TS) em paralelo.
- T006/T007 (testes US1) podem ser escritos juntos; idem T013/T014 (US2).
- Fase Polish: T021–T024 em paralelo.

## Implementation Strategy

- **MVP** = US1 (backup). Entrega valor imediato (proteção dos dados) e é testável sozinha.
- Incremento seguinte = US2 (restauração), fechando o ciclo.
