# Implementation Plan: Backup e Restauração da Base de Dados

**Branch**: `012-db-backup-restore` | **Date**: 2026-07-23 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/012-db-backup-restore/spec.md`

## Summary

Permitir que o usuário, pela tela de Configurações, gere um backup completo da
base SQLite (`financas.db`) em uma pasta escolhida e restaure/importe a base a
partir de um arquivo de backup, sempre localmente e com confirmação antes de
sobrescrever. Abordagem técnica: dois comandos Tauri (`backup_database`,
`restore_database`) sobre o `Database` existente; backup via `VACUUM INTO`
(snapshot consistente); restauração com validação prévia, cópia de segurança
automática da base atual, troca do arquivo e recarga do estado em memória
(`SharedStore` + `AppConfig`), seguida de reload do webview.

## Technical Context

**Language/Version**: Rust 1.75+ (backend), TypeScript 5.8 / Vue 3.5 (frontend)

**Primary Dependencies**: Tauri v2, rusqlite 0.32 (SQLite), `@tauri-apps/plugin-dialog`,
Pinia, chrono (timestamps)

**Storage**: SQLite `financas.db` em `app_data_dir` — fonte única de verdade

**Testing**: `cargo test` (unit/integration Rust), Vitest (frontend)

**Target Platform**: App desktop (macOS/Windows/Linux via Tauri)

**Project Type**: Desktop app (backend Rust + frontend Vue em webview)

**Performance Goals**: Backup/restauração de base pessoal (poucos MB) em < 30 s (SC-001)

**Constraints**: 100% offline; nenhuma operação destrutiva sem confirmação e sem
cópia de segurança prévia (FR-007, FR-009); dados atuais nunca corrompidos em falha (FR-010)

**Scale/Scope**: Usuário único; backup manual integral; 1 tela (Configurações),
2 comandos backend, 2 wrappers de serviço frontend

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. TDD (NON-NEGOTIABLE)**: Testes primeiro. Cobrir em `db.rs` (unit): backup gera
  arquivo válido e restauração faz roundtrip; validação rejeita arquivo inválido;
  base atual preservada em cópia de segurança antes da troca. Escrever red → green → refactor. ✅ PASS
- **II. Clean Architecture**: I/O de arquivo e SQLite ficam na camada `infrastructure`
  (`Database`); comandos (`commands/backup.rs`) orquestram e tocam o estado gerenciado;
  frontend só chama via `services/tauri.service.ts`. Sem lógica de negócio nova no domínio. ✅ PASS
- **III. Simplicidade/YAGNI**: Sem agendamento, sem export seletivo, sem versionamento de
  backups além do timestamp no nome. Reuso do `Database` e do diálogo existentes. ✅ PASS
- **IV. Integridade de dados**: `VACUUM INTO` garante snapshot consistente; validação por
  `PRAGMA integrity_check` + presença das tabelas centrais antes de qualquer troca; falha
  não altera a base atual; cópia de segurança automática permite reverter. ✅ PASS
- **V. Local-First & Privacy**: Nenhuma rede. Arquivos gravados apenas no local escolhido
  pelo usuário e em `app_data_dir` (cópia de segurança). ✅ PASS

**Resultado**: PASS — sem violações; Complexity Tracking não necessário.

## Project Structure

### Documentation (this feature)

```text
specs/012-db-backup-restore/
├── plan.md              # Este arquivo
├── research.md          # Fase 0
├── data-model.md        # Fase 1
├── quickstart.md        # Fase 1
├── contracts/
│   ├── backup_database.md
│   └── restore_database.md
├── checklists/
│   └── requirements.md
└── tasks.md             # Fase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
src-tauri/src/
├── infrastructure/
│   └── db.rs                 # + campo path; + backup_to / validate_backup / restore_from
├── application/
│   └── store.rs              # + InvoiceStore::replace_all (recarga pós-restauração)
├── commands/
│   ├── backup.rs             # NOVO: backup_database, restore_database
│   └── mod.rs                # + pub mod backup
└── lib.rs                    # registrar comandos; Database guarda o db_path

src/
├── services/tauri.service.ts # + backupDatabase / restoreDatabase
├── pages/SettingsPage.vue    # + seção "Backup e restauração" (botões + confirmações)
└── types/api.types.ts        # + BackupResult (se necessário)

src-tauri/capabilities/default.json  # + dialog:allow-save (se usarmos save-dialog); open-directory já coberto
```

**Structure Decision**: App desktop existente. Reuso das camadas atuais:
`infrastructure` (SQLite/arquivo), `commands` (boundary Tauri), `services` (IPC front),
`pages/SettingsPage.vue` (apresentação). Nada novo no `domain`.

## Complexity Tracking

> Sem violações constitucionais. Tabela não aplicável.
