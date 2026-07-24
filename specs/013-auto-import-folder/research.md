# Research — Pasta de Importação Automática

## Decisão 1: Identificar fatura vs extrato numa pasta única

- **Decisão**: Detecção por formato + conteúdo, reusando os parsers existentes:
  - `.xls` → tratar como **extrato** (faturas BTG são sempre `.xlsx`).
  - `.xlsx` → tentar **fatura** (`import_invoice`/`map_sheet_to_transactions`); se
    devolver `INVALID_FORMAT`, tentar **extrato** (`read_statement`). Se ambos
    falharem → ignorar com aviso.
- **Rationale**: Os dois parsers já rejeitam o formato do outro:
  `map_sheet_to_transactions` exige a seção de transações da fatura BTG;
  `read_statement` (via `parse_statement_rows`) erra "formato não reconhecido"
  quando não há lançamentos. O fallback dá robustez sem convenção de nome de arquivo.
- **Alternativas rejeitadas**: convenção de nome ("fatura"/"extrato") — frágil;
  subpastas — usuário pediu pasta única; perguntar ao usuário por arquivo — quebra
  o "automático".

## Decisão 2: Reuso dos fluxos de importação

- **Decisão**: Fatura → `application::import_invoice::import_invoice` (adiciona ao
  `InvoiceStore`; persistir snapshot depois). Extrato → `read_statement` +
  `classify_entry` (mesma lógica de `commands::bank::classify_all`) → `save_bank_entries`.
  Extrair a classificação para uma função reutilizável evitando duplicar regra.
- **Rationale**: DRY e integridade — a categorização/dedup já validados são reusados.
- **Alternativas rejeitadas**: reimplementar parsing/classificação (duplicação, risco).

## Decisão 3: Dedup no re-scan

- **Decisão**: Confiar no dedup determinístico existente: fatura por `invoice_id`
  (UUIDv5 do nome do arquivo) + `store.add` substitui mesmo filename; extrato por
  `BankEntry.id` (UNIQUE, `ON CONFLICT DO UPDATE`).
- **Rationale**: Reabrir o app N vezes não duplica (SC-003). Nenhum estado extra
  de "arquivos já vistos" é necessário.
- **Alternativas rejeitadas**: manter lista de arquivos importados — redundante e
  quebra se o arquivo muda de conteúdo.

## Decisão 4: Senha de fatura na importação automática

- **Decisão**: Se um `.xlsx` estiver cifrado, usar a senha salva no Keychain
  (`secrets::get_password`). Sem senha salva → ignorar o arquivo com aviso
  "protegido por senha".
- **Rationale**: O boot/varredura não pode abrir prompt bloqueante; a senha salva
  já é usada pelo import manual silencioso.
- **Alternativas rejeitadas**: bloquear esperando digitação (trava o boot).

## Decisão 5: Momento e local da varredura

- **Decisão**: (a) No boot (`lib.rs setup`), após carregar DB/config/store: se
  `import_directory` está definido e existe, rodar `import_from_folder` com a senha
  salva; guardar o resumo numa célula gerenciada `Mutex<Option<FolderImportSummary>>`.
  (b) No comando `set_import_directory`, rodar a varredura na hora e devolver o resumo.
- **Rationale**: cobre US2 (auto no boot) e US1 (importar ao definir) com o mesmo
  núcleo `import_from_folder(db, store, cfg, password)`.
- **Alternativas rejeitadas**: watcher/polling em segundo plano — fora do escopo (YAGNI).

## Decisão 6: Mostrar o resumo do auto-import (visibilidade de status)

- **Decisão**: Célula gerenciada guarda o último resumo do boot; comando
  `get_startup_import_summary` lê **e limpa**. `App.vue` chama no `onMounted`; se
  houver resumo, mostra um toast/banner discreto ("N faturas, M extratos, K ignorados").
- **Rationale**: Evento no boot pode chegar antes de o front montar; a célula +
  leitura-única é confiável e simples. Cumpre FR-007.
- **Alternativas rejeitadas**: persistir resumo no DB (estado extra); emitir evento
  no setup (corrida com a montagem do front).

## Decisão 7: Configuração da pasta e campo legado

- **Decisão**: Novo campo `import_directory: Option<String>` no `AppConfig`
  (setting `import_directory`; vazio/ausente = desligado). Na tela Configurações,
  substituir o campo texto "Pasta das Faturas" (relativo, nunca usado para leitura)
  por um seletor de pasta (`open({ directory:true })`) com opção de limpar.
  `faturas_directory` permanece no struct por compatibilidade, oculto na UI.
- **Rationale**: Introduz o recurso sem quebrar o schema/fixtures existentes.
- **Alternativas rejeitadas**: remover `faturas_directory` (mexe em muitos testes e
  na migração de config).

## Decisão 8: Robustez no boot

- **Decisão**: Pasta ausente/ilegível → `import_from_folder` retorna resumo com erro
  registrado, sem `panic`/`unwrap`; o boot segue. Falha por arquivo é capturada e
  vira item "ignorado", nunca aborta a varredura.
- **Rationale**: FR-009/SC-005 — o app nunca trava por causa da pasta.
