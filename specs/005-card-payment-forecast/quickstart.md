# Quickstart — validar a previsão do cartão

Roteiro para provar a feature ponta a ponta. Pré-requisito: app compilando (`npm run tauri dev`).

## 1. Testes de domínio (TDD, primeiro)

```bash
cd src-tauri && cargo test forecast
```

Espera-se cobrir:
- 1 compra em 3x com parcela 1/3 → 2 pontos futuros (mês+1, mês+2), cada = valor da parcela.
- 2 compras caindo no mesmo mês → ponto único com a soma + 2 itens.
- Mesma compra em 2 faturas (1/3 e 2/3) → dedup: projeta só de 2/3 (1 ponto), sem contar em dobro.
- Última parcela (3/3) → nenhum ponto futuro.
- Sem parcelamentos → série vazia.
- Compra estornada → não entra.
- **Invariante**: `Σ pontos == installments_future_total`.

## 2. Frontend

```bash
npx vue-tsc --noEmit && npm run test:run
```

## 3. Validação manual (app rodando)

1. Importe uma fatura BTG que tenha compras parceladas (ex.: "Mercado Livre (2/3)").
2. Abra a tela **Ano** → seção **Previsão do cartão**: uma barra por mês futuro com o valor; passe o mouse → composição (quais compras).
3. Confira que o **último mês com barra** é onde a parcela mais longa termina.
4. Abra a tela **Mês** → card **Próximos meses do cartão**: resumo dos próximos meses + total comprometido + mês que zera.
5. Confira que a **soma das barras** bate com "parcelas futuras" já mostrado no painel (`installments_future_total`).
6. Remova todas as faturas com parcelas → ambas as telas mostram estado vazio, sem erro.

## Pronto quando

- `cargo test forecast` verde (≥90% na lógica).
- Gráfico no Ano + resumo no Mês exibindo a projeção real.
- Soma projetada == total de parcelas futuras.
