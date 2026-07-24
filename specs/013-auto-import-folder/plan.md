# Implementation Plan: Pasta de Importação Automática

**Branch**: `013-auto-import-folder` | **Date**: 2026-07-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/013-auto-import-folder/spec.md`

## Summary

Configurar uma pasta única (caminho absoluto) de onde o app importa faturas
(`.xlsx`) e extratos (`.xls`/`.xlsx`), identificando o tipo por conteúdo/formato.
Ao definir a pasta importa na hora; ao abrir o app varre e importa arquivos novos
automaticamente. Reuso dos fluxos existentes (`import_invoice`, `read_statement` +
classificação) e do dedup determinístico. Um resumo (faturas/extratos/ignorados)
é mostrado após cada varredura.

## Technical Context

**Language/Version**: Rust 1.75+ (backend), TypeScript 5.8 / Vue 3.5 (frontend)

**Primary Dependencies**: Tauri v2, rusqlite, calamine (leitura xls/xlsx),
`@tauri-apps/plugin-dialog` (escolher pasta), Pinia

**Storage**: SQLite `financas.db`; nova setting `import_directory`

**Testing**: `cargo test`, Vitest

**Target Platform**: App desktop (macOS/Windows/Linux via Tauri)

**Project Type**: Desktop app (backend Rust + frontend Vue)

**Performance Goals**: pasta pessoal (dezenas de arquivos); varredura no boot não
congela o uso indefinidamente

**Constraints**: 100% offline; dedup obrigatório; falha de um arquivo não
interrompe os demais; app nunca trava por pasta ausente/ilegível

**Scale/Scope**: usuário único; 1 setting, 1 módulo de aplicação novo, 2 comandos,
ajuste no boot (`lib.rs`) e na tela Configurações

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. TDD (NON-NEGOTIABLE)**: Testes primeiro para o classificador de tipo e para
  o importador de pasta (detecta fatura vs extrato; ignora inválido sem abortar;
  dedup no re-scan). ✅ PASS
- **II. Clean Architecture**: Detecção + orquestração da varredura na camada
  `application` (`import_folder.rs`), reusando `application::import_invoice` e a
  `infrastructure` de extrato/BTG; comandos só no boundary; front só via service. ✅ PASS
- **III. Simplicidade/YAGNI**: Sem watcher em segundo plano, sem agendamento, sem
  export; reuso máximo dos parsers/dedup existentes. ✅ PASS
- **IV. Integridade de dados**: Dedup determinístico já existente evita duplicatas;
  arquivo inválido/senha ausente é ignorado com aviso explícito, nunca importado
  como lixo silencioso. ✅ PASS
- **V. Local-First & Privacy**: Só leitura de arquivos locais; nenhuma rede. ✅ PASS

**Resultado**: PASS — sem violações.

## Project Structure

### Documentation (this feature)

```text
specs/013-auto-import-folder/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── set_import_directory.md
│   └── get_startup_import_summary.md
├── checklists/requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
src-tauri/src/
├── domain/mod.rs                 # AppConfig + campo import_directory
├── application/
│   ├── import_folder.rs          # NOVO: varre pasta, detecta tipo, importa, resumo
│   └── mod.rs                    # + pub mod import_folder
├── commands/
│   ├── import.rs                 # + set_import_directory, get_startup_import_summary
│   └── mod.rs
├── infrastructure/db.rs          # settings: persistir/ler import_directory
└── lib.rs                        # boot: auto-import se pasta definida; managed summary cell

src/
├── services/tauri.service.ts     # + setImportDirectory / getStartupImportSummary
├── pages/SettingsPage.vue        # trocar campo "Pasta das Faturas" por seletor de pasta
├── App.vue                       # ao montar: buscar+mostrar resumo do auto-import (toast)
└── types/api.types.ts            # + FolderImportSummary; import_directory no AppConfig
```

**Structure Decision**: Reuso das camadas atuais. Nova lógica de orquestração de
varredura fica em `application/import_folder.rs` (testável). Detecção de tipo é
regra de aplicação, não de domínio.

## Complexity Tracking

> Sem violações constitucionais. Tabela não aplicável.
