# Contracts: Comandos Tauri — inflação pessoal detalhada

**Feature**: `011-personal-inflation-calc` | **Plan**: [../plan.md](../plan.md)

Regras gerais: dinheiro trafega como **decimal-string**; taxas como **number**.
Chamado apenas via `services/tauri.service.ts` (`invoke`). **Offline** — nenhuma
chamada de rede nova; consome o **cache de índices** já persistido por 006.

## `get_personal_inflation_detail`

Detalhamento rico da inflação pessoal: contribuições por categoria, comparação com o
oficial (em pontos percentuais), impacto em reais na cesta e na renda, e simulação
comportamental opcional.

- **Registro**: `commands/inflation.rs` → `lib.rs` (`invoke_handler`).
- **Assinatura (Rust)**:
  ```rust
  #[tauri::command]
  pub async fn get_personal_inflation_detail(
      store: State<'_, SharedStore>,
      config: State<'_, Mutex<AppConfig>>,
      db: State<'_, SharedDb>,
  ) -> Result<Option<PersonalInflationResult>, String>
  ```
- **Parâmetros (do frontend)**: **nenhum**. O comando deriva tudo do estado local.
- **Retorno**: `PersonalInflationResult | null` (ver [../data-model.md](../data-model.md)).

### Como monta as entradas

1. Lê o cache de índices (`load_inflation_cache`). Sem cache → `null`.
2. Obtém gastos por categoria e renda do dashboard (`get_dashboard`, filtro default).
3. Converte percent→decimal (÷100): oficial = `headline.month/100`; por categoria, a
   variação mensal do grupo mapeado (`map_category_to_group`) ÷100; sem grupo → IPCA
   geral com **proveniência** registrada.
4. Filtra categorias com gasto > 0. Sem nenhuma → `null`.
5. `compute(inputs, geral, renda, Some(DEFAULT_BEHAVIORAL_COEFFICIENT /*1,4*/), WeightMode::Current)`.

### Resultados / erros

| Situação | Retorno |
|----------|---------|
| Cache + gastos presentes | `PersonalInflationResult` (Ok) |
| Sem cache de índices | `null` |
| Sem categorias com gasto > 0 | `null` |
| Erro do cálculo puro (ex.: total ≤ 0) | `Err(String)` — mensagem do `PersonalInflationError` |
| Falha de lock / parse do cache | `Err(String)` |

**Observações**:
- Não faz fetch — se o cache estiver vazio, o usuário deve atualizar os índices via o
  fluxo opt-in de 006 (`fetch_ipca`), fora desta feature.
- Determinístico: mesmas entradas (mesmo cache + mesmos gastos) → mesma saída.
- Sempre inclui `aviso` (aviso metodológico) quando retorna `Ok(Some)`.

## Frontend

`src/services/tauri.service.ts`:

```ts
export async function getPersonalInflationDetail(): Promise<PersonalInflationDetail | null> {
  try {
    return await invoke<PersonalInflationDetail | null>("get_personal_inflation_detail");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}
```

Tipos em `src/types/api.types.ts`: `PersonalInflationDetail` e `InflationContribution`
(espelham o DTO do Rust; dinheiro como string, taxas como number).
