# Data Model — Pasta de Importação Automática

Sem tabelas novas. Uma nova setting e DTOs de resumo. Faturas/extratos reusam as
entidades existentes.

## Configuração

### `import_directory` (setting)

- **O que é**: caminho absoluto da pasta única de importação. Vazio/ausente = recurso
  desligado.
- **Persistência**: tabela `settings`, chave `import_directory`.
- **No `AppConfig`**: campo `import_directory: Option<String>` (serde
  `default`/`skip_serializing_if` para compatibilidade com config antigo).

## DTOs de fronteira (Tauri ↔ frontend), serde camelCase

### `FolderImportSummary`

Resultado de uma varredura (retorno de `set_import_directory` e de
`get_startup_import_summary`).

| Campo      | Tipo             | Descrição                                        |
|------------|------------------|--------------------------------------------------|
| `faturas`  | number           | Faturas importadas com sucesso.                  |
| `extratos` | number           | Extratos importados (arquivos) com sucesso.      |
| `entries`  | number           | Lançamentos de extrato salvos (dedup aplicado).  |
| `ignored`  | `IgnoredFile[]`  | Arquivos ignorados, com motivo.                  |
| `directory`| string           | Pasta varrida.                                   |

### `IgnoredFile`

| Campo    | Tipo   | Descrição                                                   |
|----------|--------|-------------------------------------------------------------|
| `name`   | string | Nome do arquivo ignorado.                                   |
| `reason` | string | Motivo: `NOT_RECOGNIZED` / `ENCRYPTED_NO_PASSWORD` / `ERROR`.|

## Estado interno (backend)

- `Mutex<Option<FolderImportSummary>>` — **novo estado gerenciado**: guarda o resumo
  da varredura de boot até o front lê-lo uma vez (`get_startup_import_summary` limpa).
- `SharedStore` — recebe as faturas importadas (via `import_invoice` + `persist`).
- `SharedDb` — recebe os lançamentos de extrato (via `save_bank_entries`).

## Regras

1. Detecção de tipo (ver research): `.xls` → extrato; `.xlsx` → fatura, senão extrato.
2. Um arquivo que falha vira item em `ignored`; a varredura continua.
3. Dedup: fatura por `invoice_id` (nome) + substituição por filename; extrato por
   `BankEntry.id` (UNIQUE/UPSERT). Re-scan não duplica.
4. Pasta inexistente/ilegível: resumo com `ignored`/erro; sem travar o app.
5. `import_directory` vazio: nenhuma varredura; importação manual segue igual.
