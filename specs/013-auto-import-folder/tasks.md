---

description: "Task list — Pasta de Importação Automática"
---

# Tasks: Pasta de Importação Automática

**Input**: Design documents from `/specs/013-auto-import-folder/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: INCLUÍDOS (Constituição §I, TDD). Testes Rust em `application::import_folder`.

**Organization**: por user story — US1 (definir pasta + importar) P1, US2 (auto no boot) P2.

## Format: `[ID] [P?] [Story] Description`

---

## Phase 1: Setup

- [ ] T001 Confirmar em `src-tauri/capabilities/default.json` que `dialog:allow-open` cobre `open({directory:true})` (já usado); nenhuma permission nova esperada.

---

## Phase 2: Foundational (Blocking Prerequisites)

- [ ] T002 Adicionar `import_directory: Option<String>` ao `AppConfig` em `src-tauri/src/domain/mod.rs` (serde `default`, `skip_serializing_if = "Option::is_none"`); ajustar `Default`. Atualizar fixtures/asserts de `AppConfig` que fazem match exaustivo (domain, db, import_invoice, recategorize) para incluir o campo.
- [ ] T003 Persistir/ler a setting em `src-tauri/src/infrastructure/db.rs`: em `save_config` gravar `settings('import_directory', ...)` quando `Some`; em `load_config` ler para `import_directory`.
- [ ] T004 [P] Espelhar tipos no frontend em `src/types/api.types.ts`: `import_directory?: string | null` no `AppConfig`; `FolderImportSummary` + `IgnoredFile`. Ajustar default em `src/stores/settings.store.ts`.
- [ ] T005 Extrair a classificação de extrato reutilizável: mover a lógica de `commands::bank::classify_all` para uma função em `application`/`domain` que receba `(path, &AppConfig, payslip_months)` e devolva entradas classificadas, para reuso pelo importador de pasta sem duplicar regra.

---

## Phase 3: User Story 1 — Definir a pasta e importar (Priority: P1) 🎯 MVP

**Goal**: Escolher uma pasta em Configurações; o app importa faturas e extratos dela, detectando o tipo, e mostra um resumo.

**Independent Test**: Definir uma pasta com uma fatura e um extrato; ambos importados e visíveis; resumo exibido.

### Tests (primeiro — devem falhar)

- [ ] T006 [US1] Teste em `src-tauri/src/application/import_folder.rs` (`#[cfg(test)]`): dada uma pasta com um `.xlsx` de fatura válido e um extrato, `import_from_folder` retorna `faturas=1`, `extratos>=1` e popula store/DB.
- [ ] T007 [US1] Teste: arquivo lixo (não fatura/nem extrato) entra em `ignored` com motivo e NÃO aborta a importação dos válidos.
- [ ] T008 [US1] Teste: rodar `import_from_folder` duas vezes na mesma pasta não duplica (faturas por nome; extrato por id).

### Implementation

- [ ] T009 [US1] Criar `src-tauri/src/application/import_folder.rs` com `FolderImportSummary`/`IgnoredFile` e `pub fn import_from_folder(db: &SharedDb, store: &SharedStore, cfg: &AppConfig, password: Option<&str>) -> FolderImportSummary`: varre `read_dir`; para cada arquivo `.xls`/`.xlsx` detecta tipo (`.xls`→extrato; `.xlsx`→fatura, senão extrato via fallback ao `INVALID_FORMAT`); importa reusando `import_invoice` (fatura) e o classificador de extrato (T005) + `save_bank_entries`; captura falhas por arquivo em `ignored`; persiste snapshot de faturas. Registrar `pub mod import_folder;` em `application/mod.rs`.
- [ ] T010 [US1] Comando `set_import_directory(dir: Option<String>, store, config, db) -> Result<Option<FolderImportSummary>, String>` em `src-tauri/src/commands/import.rs`: atualiza+persiste `import_directory`; se vazio retorna `None`; senão valida pasta (`IMPORT_DIR_INVALID`) e roda `import_from_folder` com senha do Keychain; retorna resumo.
- [ ] T011 [US1] Registrar `set_import_directory` no `use` e no `generate_handler!` em `src-tauri/src/lib.rs`.
- [ ] T012 [US1] Wrapper `setImportDirectory(dir: string | null): Promise<FolderImportSummary | null>` em `src/services/tauri.service.ts` (mapError; `dir` vazio → envia `null`).
- [ ] T013 [US1] `src/pages/SettingsPage.vue`: substituir o campo texto "Pasta das Faturas" por "Pasta de importação automática" com botão **Escolher pasta** (`open({directory:true})`), exibição do caminho atual, botão **Limpar**, e exibição do resumo/erros após definir. Aplicar skill `nielsen-heuristics` (status, recuperação de erro, linguagem clara).

**Checkpoint**: US1 entregue (MVP) — importar de uma pasta ao defini-la.

---

## Phase 4: User Story 2 — Auto-import ao abrir (Priority: P2)

**Goal**: Com a pasta definida, abrir o app importa arquivos novos automaticamente e mostra um resumo discreto.

**Independent Test**: Com pasta definida, adicionar arquivo novo, reabrir o app, ver os dados sem clicar em importar.

### Tests (primeiro)

- [ ] T014 [US2] Teste: `get_startup_import_summary` faz `take` (lê e limpa) da célula `Mutex<Option<FolderImportSummary>>` — segunda chamada retorna `None`.

### Implementation

- [ ] T015 [US2] Em `src-tauri/src/lib.rs setup`: após carregar config/store/db, se `import_directory` estiver definido, rodar `import_from_folder` (senha do Keychain) e guardar o resultado numa célula gerenciada `Mutex<Option<FolderImportSummary>>` (`app.manage`). Robustez: pasta ausente/ilegível não pode causar `panic`.
- [ ] T016 [US2] Comando `get_startup_import_summary(cell) -> Result<Option<FolderImportSummary>, String>` em `commands/import.rs` (take da célula). Registrar em `lib.rs`.
- [ ] T017 [US2] Wrapper `getStartupImportSummary(): Promise<FolderImportSummary | null>` em `src/services/tauri.service.ts`.
- [ ] T018 [US2] `src/App.vue`: no `onMounted`, chamar `getStartupImportSummary`; se houver resumo, mostrar toast/banner discreto ("N faturas, M extratos, K ignorados"), com detalhe de ignorados opcional. Aplicar `nielsen-heuristics` (visibilidade de status, sem bloquear).

**Checkpoint**: ciclo completo — definir pasta e auto-import no boot.

---

## Phase 5: Polish & Cross-Cutting

- [ ] T019 [P] Mensagens de erro pt-BR para `IMPORT_DIR_INVALID` e motivos de ignorado em `src/services/tauri.service.ts` / UI.
- [ ] T020 [P] Rodar `cd src-tauri && cargo test`, `npm run test:run`, `npx vue-tsc --noEmit` — tudo verde.
- [ ] T021 [P] Validar manualmente pela tela Configurações seguindo `specs/013-auto-import-folder/quickstart.md`.
- [ ] T022 [P] Atualizar `docs/MAINTENANCE.md` (receita "Pasta de importação automática": detecção de tipo, dedup, boot, célula de resumo).

---

## Dependencies & Execution Order

- Setup (T001) → Foundational (T002–T005) → US1 (T006–T013) → US2 (T014–T018) → Polish (T019–T022).
- US2 depende de `import_from_folder` (T009, em US1) e da célula gerenciada (T015).
- TDD: testes antes da implementação em cada story.

## Parallel Opportunities

- T004 (tipos TS) paralelo a T002/T003 (Rust).
- T006–T008 (testes US1) juntos. Polish T019–T022 em paralelo.

## Implementation Strategy

- **MVP** = US1 (definir pasta + importar ao definir). US2 (auto no boot) é o incremento de conveniência.
