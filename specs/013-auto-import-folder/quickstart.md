# Quickstart — Pasta de Importação Automática

Guia de validação. Contratos em [contracts/](contracts/); decisões em [research.md](research.md).

## Rodar os testes (TDD)

```bash
cd src-tauri && cargo test      # detecção de tipo + import_folder (dedup, ignora inválido)
npm run test:run                # wrappers de serviço / store
npx vue-tsc --noEmit            # type-check
```

Esperado: testes de `application::import_folder` cobrindo — (1) `.xlsx` de fatura vira
fatura; (2) extrato vira extrato; (3) arquivo lixo entra em `ignored` sem abortar;
(4) rodar a varredura 2x não duplica.

## Validação manual

### Definir pasta e importar (US1)

1. Configurações → "Pasta de importação automática" → **Escolher pasta**.
2. Escolher uma pasta contendo pelo menos uma fatura `.xlsx` e um extrato `.xls`.
3. Esperado: toast/resumo "N faturas, M extratos, K ignorados"; painel reflete os dados.
4. Repetir/clicar de novo: nada duplica.
5. **Limpar pasta**: recurso desliga; importação manual segue disponível.

### Auto-import ao abrir (US2)

1. Com a pasta definida, colocar um arquivo novo (fatura ou extrato) nela.
2. Fechar e abrir o app.
3. Esperado: dados novos no painel sem clicar em importar; aviso discreto do resumo.
4. Colocar um arquivo inválido na pasta e reabrir: os válidos importam; o inválido
   aparece como ignorado; o app não trava.
5. Apagar/mover a pasta e reabrir: app abre normalmente e avisa que a pasta sumiu.

## Critérios de aceite

FR-001..FR-011; SC-001 (importa sem seleção manual), SC-002 (novo no painel ao abrir),
SC-003 (0% duplicata em N aberturas), SC-004 (inválido não bloqueia válidos),
SC-005 (pasta ausente não trava).
