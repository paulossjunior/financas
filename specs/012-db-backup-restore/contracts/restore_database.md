# Contract — `restore_database`

Tauri command. Substitui a base atual pela de um arquivo de backup, após validar e
preservar automaticamente uma cópia de segurança da base atual.

## Frontend wrapper

```ts
// services/tauri.service.ts
restoreDatabase(sourcePath: string): Promise<RestoreResult>
// invoke("restore_database", { sourcePath })
```

O frontend só chama este comando **após** o usuário confirmar a substituição.
Ao receber sucesso, executa `window.location.reload()`.

## Signature (backend)

```rust
#[tauri::command]
pub async fn restore_database(
    source_path: String,
    db: State<'_, SharedDb>,
    config: State<'_, Mutex<AppConfig>>,
    store: State<'_, SharedStore>,
) -> Result<RestoreResult, String>
```

## Input

| Param        | Tipo   | Regras                                        |
|--------------|--------|-----------------------------------------------|
| `sourcePath` | string | Caminho de um arquivo `.db` de backup válido. |

## Output — `RestoreResult`

```json
{ "backupOfPrevious": "/Users/.../Library/.../financas-pre-restore-20260723-142611.db" }
```

## Behavior (ordem obrigatória)

1. **Validar** `sourcePath` (conexão separada): `PRAGMA integrity_check` == `"ok"` **e**
   tabelas `invoices`, `transactions`, `settings` presentes. Falhou → erro, base intocada.
2. **Cópia de segurança** da base atual → `financas-pre-restore-<ts>.db` em `app_data_dir`.
3. **Fechar** a conexão atual; **copiar** `sourcePath` sobre `financas.db`; **reabrir** +
   `init()` (migrações idempotentes atualizam esquema antigo).
4. **Recarregar** estado em memória: `AppConfig` (via `load_config`) e `SharedStore`
   (via `load_invoices` → `replace_all`).
5. Retornar `{ backupOfPrevious }`.

## Errors (string codes)

| Código               | Quando                                                     |
|----------------------|------------------------------------------------------------|
| `FILE_NOT_FOUND`     | `sourcePath` não existe.                                   |
| `INVALID_BACKUP`     | Falha em `integrity_check` ou tabelas centrais ausentes.   |
| `RESTORE_FAILED: <detalhe>` | Falha de I/O na cópia/reabertura.                   |

Em `INVALID_BACKUP`/`FILE_NOT_FOUND`: nenhuma alteração ocorre. Em `RESTORE_FAILED` após a
cópia de segurança existir, o usuário pode reverter usando `financas-pre-restore-*.db`.

## Acceptance (mapeia spec)

- US2 cenário 1: base substituída; dados do backup aparecem após reload.
- US2 cenário 2: confirmação cancelada no frontend → comando não é chamado; base intacta.
- US2 cenário 3: arquivo inválido → `INVALID_BACKUP`; base intacta.
- US2 cenário 4: `backupOfPrevious` aponta para a cópia de segurança da base anterior.
