# Contract — `set_import_directory`

Tauri command. Define (ou limpa) a pasta de importação e, se definida, varre+importa
o conteúdo na hora, devolvendo o resumo.

## Frontend wrapper

```ts
// services/tauri.service.ts
setImportDirectory(dir: string | null): Promise<FolderImportSummary | null>
// invoke("set_import_directory", { dir })
```

`dir === null` (ou "") limpa a configuração e retorna `null` (sem varredura).

## Signature (backend)

```rust
#[tauri::command]
pub async fn set_import_directory(
    dir: Option<String>,
    store: State<'_, SharedStore>,
    config: State<'_, Mutex<AppConfig>>,
    db: State<'_, SharedDb>,
) -> Result<Option<FolderImportSummary>, String>
```

## Behavior

1. Atualiza `AppConfig.import_directory` e persiste (setting `import_directory`).
2. Se `dir` vazio/ausente → retorna `Ok(None)` (recurso desligado).
3. Senão, valida que a pasta existe; roda `import_from_folder(db, store, cfg, senha
   salva)`; persiste snapshot de faturas; retorna `Ok(Some(summary))`.

## Errors

| Código               | Quando                                    |
|----------------------|-------------------------------------------|
| `IMPORT_DIR_INVALID` | Caminho informado não existe/não é pasta. |

Falhas por arquivo NÃO viram erro do comando — entram em `summary.ignored`.

## Acceptance (spec)

- US1-1: retorna resumo com contagens por tipo.
- US1-2: re-executar não duplica (dedup).
- US1-3: arquivo não reconhecido aparece em `ignored`, os demais importam.
- US1-4: `dir` nulo desliga o recurso.
