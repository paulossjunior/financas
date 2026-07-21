# Quickstart — validar os indicadores de inflação

Pré-requisito: app compilando (`npm run tauri dev`).

## 1. Testes de domínio (TDD, primeiro)

```bash
cd src-tauri && cargo test inflation
```

Cobrir:
- mapeamento categoria → grupo (Alimentação→bebidas, Transporte→transportes, Saúde→saúde, Assinaturas→comunicação…).
- reponderação: pesos {Alimentação 70%, Transporte 30%} × variações → inflação pessoal correta.
- categoria sem grupo → usa variação geral.
- sem gastos → pessoal == geral, diff == 0.
- invariante: soma dos pesos = total considerado.

## 2. Frontend

```bash
npx vue-tsc --noEmit && npm run test:run
```

## 3. Validação manual (app rodando)

1. Abra a tela **Ano** → card **Inflação**: se nunca atualizou, aparece estado vazio com **Atualizar índices**.
2. Clique **Atualizar índices** (online) → aparecem IPCA do mês/ano/12m + a **data** da atualização + sua **inflação pessoal** e a diferença para o IPCA.
3. Feche e reabra o app **offline** → os índices continuam visíveis (cache), com a data em que foram baixados.
4. Tela **Mês** → resumo compacto de inflação (mês + pessoal).
5. Desligue a internet e clique **Atualizar** → mensagem de erro clara; o cache anterior permanece.

## Pronto quando

- `cargo test inflation` verde (≥90% na lógica).
- IPCA + inflação pessoal no Ano (completo) e Mês (compacto).
- Funciona offline após 1 atualização; mostra a data do índice.
