# Contract — `backup_database`

Tauri command. Gera um backup completo da base atual numa pasta escolhida.

## Frontend wrapper

```ts
// services/tauri.service.ts
backupDatabase(destDir: string): Promise<BackupResult>
// invoke("backup_database", { destDir })
```

## Signature (backend)

```rust
#[tauri::command]
pub async fn backup_database(
    dest_dir: String,
    db: State<'_, SharedDb>,
) -> Result<BackupResult, String>
```

## Input

| Param     | Tipo   | Regras                                              |
|-----------|--------|-----------------------------------------------------|
| `destDir` | string | Pasta existente com permissão de escrita. Não vazia.|

## Output — `BackupResult`

```json
{ "path": "/Users/.../Backups/financas-backup-20260723-142530.db" }
```

## Behavior

1. Determina o nome `financas-backup-<YYYYMMDD-HHMMSS>.db`; se já existir em `destDir`,
   acrescenta sufixo `-N` até um nome livre.
2. Executa `VACUUM INTO` no caminho de destino (snapshot consistente).
3. Retorna o caminho completo do arquivo criado.

## Errors (string codes)

| Código             | Quando                                        |
|--------------------|-----------------------------------------------|
| `BACKUP_DIR_INVALID` | `destDir` não existe ou não é diretório.     |
| `BACKUP_FAILED: <detalhe>` | Falha de escrita/VACUUM (permissão, disco).|

Em qualquer erro, nenhum arquivo parcial é apresentado como válido; a base atual é intocada.

## Acceptance (mapeia spec)

- US1 cenário 1: retorna `path` dentro de `destDir` com timestamp no nome.
- US1 cenário 2: novo backup não sobrescreve anterior (nomes distintos).
- US1 cenário 3: cancelamento do diálogo ocorre no frontend → comando não é chamado.
