# Research — Backup e Restauração da Base de Dados

## Decisão 1: Mecanismo de backup (snapshot consistente)

- **Decisão**: Usar `VACUUM INTO '<destino>'` do SQLite para gerar a cópia.
- **Rationale**: Produz um arquivo de banco novo, compactado e consistente, mesmo com a
  conexão aberta e transações em andamento — sem risco de copiar um arquivo pela metade.
  Não depende do modo de journal.
- **Alternativas rejeitadas**:
  - `std::fs::copy` do arquivo aberto: pode capturar estado inconsistente (WAL/journal),
    arriscando um backup corrompido apresentado como válido (viola FR/SC de integridade).
  - Online Backup API do rusqlite: mais código para o mesmo resultado que `VACUUM INTO`.

## Decisão 2: Nome do arquivo de backup (sem colisão)

- **Decisão**: `financas-backup-YYYYMMDD-HHMMSS.db`; se já existir, sufixo incremental
  `-1`, `-2`, … até um nome livre.
- **Rationale**: Timestamp legível ordena naturalmente e evita sobrescrever (FR-003, SC-004);
  o sufixo cobre o caso raro de dois backups no mesmo segundo.
- **Alternativas rejeitadas**: sobrescrever nome fixo (perde histórico); UUID no nome
  (ilegível para o usuário).

## Decisão 3: Validação do arquivo de restauração

- **Decisão**: Antes de qualquer troca, abrir o candidato em conexão separada e checar:
  (a) `PRAGMA integrity_check` == "ok"; (b) presença das tabelas centrais do app
  (`invoices`, `transactions`, `settings`). Rejeitar com erro claro se falhar.
- **Rationale**: Impede substituir a base por um arquivo que não é do app ou está corrompido
  (FR-008), preservando os dados atuais.
- **Alternativas rejeitadas**: checar só a extensão `.db` (não garante conteúdo válido);
  não validar (viola integridade de dados).

## Decisão 4: Cópia de segurança automática antes de restaurar

- **Decisão**: Antes de trocar o arquivo, gravar a base atual como
  `financas-pre-restore-YYYYMMDD-HHMMSS.db` em `app_data_dir` (via `VACUUM INTO`).
- **Rationale**: Restauração é destrutiva; a cópia garante reversibilidade (FR-009, SC-003).
- **Alternativas rejeitadas**: confiar apenas no backup manual do usuário (pode não existir).

## Decisão 5: Troca do arquivo com conexão aberta

- **Decisão**: Dentro de `&mut Database`: (1) validar candidato; (2) gravar cópia de
  segurança; (3) fechar a conexão atual (substituindo por conexão in-memory temporária e
  dropando a antiga); (4) `std::fs::copy(candidato → db_path)`; (5) reabrir conexão no
  `db_path` e rodar `init()` (migrações idempotentes cobrem esquema mais antigo).
- **Rationale**: Garante que o arquivo esteja fechado antes de sobrescrever (necessário em
  Windows) e que o esquema fique atualizado após restaurar um backup de versão anterior.
- **Alternativas rejeitadas**: manter conexão aberta e sobrescrever (falha em Windows,
  arrisca corromper); exigir reinício do app (viola FR-011 — sem passos manuais).

## Decisão 6: Recarregar estado em memória após restaurar

- **Decisão**: O comando `restore_database` recarrega, a partir da base restaurada, o
  `Mutex<AppConfig>` (`load_config`) e o `SharedStore` (`load_invoices` → `replace_all`).
  O frontend, ao concluir, executa `window.location.reload()` para refazer as buscas.
- **Rationale**: O dashboard lê invoices/config do estado em memória (não relê o arquivo a
  cada chamada); sem recarga, a UI mostraria dados antigos. Reload do webview cumpre FR-011.
- **Alternativas rejeitadas**: apenas reabrir a conexão (dashboard continuaria com dados em
  memória antigos); emitir evento e recarregar store por store (mais frágil que um reload).

## Decisão 7: Seleção de pasta/arquivo (diálogos)

- **Decisão**: Backup usa `open({ directory: true })` (escolher pasta de destino);
  restauração usa `open({ filters: [{ name: "Backup", extensions: ["db"] }] })`.
- **Rationale**: Reusa `@tauri-apps/plugin-dialog` já em uso no app; `dialog:allow-open` já
  está nas capabilities, cobrindo diretório e arquivo. Sem necessidade de `dialog:allow-save`.
- **Alternativas rejeitadas**: `save()` dialog para o backup (exigiria nova permission e o
  usuário nomearia o arquivo; o requisito é escolher a pasta, com o nome gerado pelo app).
