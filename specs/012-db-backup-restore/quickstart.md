# Quickstart — Backup e Restauração da Base

Guia de validação end-to-end. Detalhes de contrato em [contracts/](contracts/).

## Pré-requisitos

- App rodando com alguns dados (faturas/contracheques/lançamentos).
- Toolchain: `npm run tauri dev` para rodar; `cd src-tauri && cargo test` para testes Rust.

## Rodar os testes (TDD)

```bash
cd src-tauri && cargo test            # backup/validate/restore em db.rs
npm run test:run                      # wrappers de serviço (frontend)
npx vue-tsc --noEmit                  # type-check
```

Esperado: testes de `db.rs` cobrindo (1) backup gera arquivo abrível com os mesmos dados;
(2) `validate` aceita base do app e rejeita arquivo inválido; (3) restauração faz roundtrip
e grava a cópia de segurança da base anterior.

## Validação manual (na tela Configurações)

### Backup (US1)

1. Configurações → seção "Backup e restauração" → **Fazer backup**.
2. Escolher uma pasta no diálogo.
3. Esperado: mensagem de sucesso com o caminho completo `.../financas-backup-<ts>.db`.
4. Repetir: novo arquivo com timestamp diferente, sem apagar o anterior.
5. Cancelar o diálogo: nada acontece, sem erro.

### Restauração (US2)

1. **Restaurar backup** → escolher um `.db` de backup válido.
2. Confirmar no diálogo de aviso ("os dados atuais serão substituídos").
3. Esperado: app recarrega e o painel passa a refletir os dados do backup; existe um
   `financas-pre-restore-<ts>.db` no diretório do app.
4. Cancelar a confirmação: base atual permanece intacta.
5. Selecionar um arquivo que não seja base do app (ex.: um `.db` qualquer/corrompido):
   erro claro de arquivo inválido; base atual intacta.

## Localizar o diretório da base (para inspeção)

- macOS: `~/Library/Application Support/<bundle id>/financas.db`
- A cópia de segurança `financas-pre-restore-*.db` fica no mesmo diretório.

## Critérios de aceite atendidos

- FR-001..FR-005 (backup), FR-006..FR-011 (restauração com validação, cópia de segurança e
  recarga), FR-012 (offline). SC-001 (< 30 s), SC-002/SC-003 (roundtrip íntegro, base
  anterior recuperável), SC-004 (sem colisão de nomes).
