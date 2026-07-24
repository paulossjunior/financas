# Data Model — Backup e Restauração

Esta feature **não cria tabelas nem entidades de domínio novas**. Opera sobre o arquivo de
banco existente. Os "conceitos" abaixo são artefatos de arquivo e DTOs de fronteira.

## Artefatos de arquivo

### Arquivo de backup

- **O que é**: cópia integral e consistente de `financas.db` num ponto no tempo.
- **Local**: pasta escolhida pelo usuário.
- **Nome**: `financas-backup-YYYYMMDD-HHMMSS.db` (sufixo `-N` se houver colisão).
- **Como é gerado**: `VACUUM INTO`.
- **Ciclo de vida**: criado sob demanda; nunca sobrescrito por backups posteriores;
  gerenciado (mover/apagar) pelo próprio usuário fora do app.

### Cópia de segurança pré-restauração

- **O que é**: snapshot da base atual, gravado automaticamente imediatamente antes de uma
  restauração, para permitir reverter.
- **Local**: `app_data_dir` (mesma pasta de `financas.db`).
- **Nome**: `financas-pre-restore-YYYYMMDD-HHMMSS.db`.
- **Ciclo de vida**: criado a cada restauração bem-sucedida; retido no diretório do app.

## DTOs de fronteira (Tauri ↔ frontend)

### `BackupResult`

Retorno de `backup_database`.

| Campo    | Tipo   | Descrição                                            |
|----------|--------|------------------------------------------------------|
| `path`   | string | Caminho completo do arquivo de backup gerado.        |

### `RestoreResult`

Retorno de `restore_database`.

| Campo             | Tipo   | Descrição                                                |
|-------------------|--------|----------------------------------------------------------|
| `backupOfPrevious`| string | Caminho da cópia de segurança da base anterior (reverter).|

## Estado interno afetado (backend)

- `Database.path: PathBuf` — **novo campo**: guarda o caminho de `financas.db` para que
  backup/restauração saibam a origem/destino sem depender do `AppHandle`.
- `Mutex<AppConfig>` (estado gerenciado) — **recarregado** de `load_config()` após restaurar.
- `SharedStore` (estado gerenciado) — **recarregado** de `load_invoices()` via novo
  `InvoiceStore::replace_all(Vec<Invoice>)` após restaurar.

## Regras de validação (restauração)

1. O candidato DEVE passar em `PRAGMA integrity_check` (resultado `"ok"`).
2. O candidato DEVE conter as tabelas centrais: `invoices`, `transactions`, `settings`.
3. Falha em (1) ou (2) → erro `INVALID_BACKUP` e **nenhuma** alteração na base atual.
4. Erro de I/O (permissão, disco cheio) em qualquer etapa → a base atual permanece intacta;
   mensagem de erro compreensível ao usuário.

## Transições de estado (restauração, caminho feliz)

```
[base atual A]
  → validar(candidato B)            (falhou? aborta, A intacta)
  → gravar cópia de segurança de A  (financas-pre-restore-*.db)
  → fechar conexão de A
  → copiar B sobre financas.db
  → reabrir + init() (migrações)
  → recarregar AppConfig + SharedStore de B
  → [base atual B]  (+ cópia de segurança de A disponível)
```
