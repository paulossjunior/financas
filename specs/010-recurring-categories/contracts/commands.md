# Contracts: Comandos Tauri — 010 recurring-categories

Convenções (iguais ao resto do app):
- Só `src/services/tauri.service.ts` chama `invoke`. Tipos TS espelham os DTOs Rust em `src/types/api.types.ts`.
- Parâmetros do frontend em **camelCase**; o Tauri mapeia para `snake_case` no Rust.
- Dinheiro = **Decimal serializado como string**. Meses = `YYYY-MM` (string).
- Erros: `Result<T, String>` no Rust → `throw new Error(...)` no serviço (mapeado por `mapError`).
- Registrar os 4 comandos novos em `commands/recurring.rs`, `commands/mod.rs` (`pub mod recurring`) e no `invoke_handler` de `lib.rs`.

---

## 1. `set_category_recurring`

Marca/atualiza (ou desmarca) uma categoria como recorrente, com vigência opcional.

**invoke**: `set_category_recurring`

**Params**:
| camelCase (TS) | snake_case (Rust) | Tipo | Obrigatório | Notas |
|----------------|-------------------|------|-------------|-------|
| `category`     | `category`        | `string` | sim | Nome da categoria (trim, não vazio). |
| `recurring`    | `recurring`       | `boolean` | sim | `true` = marca/atualiza; `false` = desmarca (DELETE). |
| `startMonth`   | `start_month`     | `string \| null` | não | `YYYY-MM`. `null` = sem limite inferior. |
| `endMonth`     | `end_month`       | `string \| null` | não | `YYYY-MM` inclusivo. `null` = ongoing. |

**Retorno**: `RecurringCategory | null`
- Ao marcar: o registro efetivado.
- Ao desmarcar (`recurring=false`): `null`.

```ts
interface RecurringCategory {
  category: string;
  start_month: string | null;
  end_month: string | null;
  created_at: string;
}
```

**Efeitos colaterais**: upsert/delete em `recurring_categories`; recálculo determinístico das fixas/baseline na próxima leitura do painel.

**Erros**:
- `"A categoria não pode ficar vazia."` — `category` vazia.
- `INVALID_MONTH` — `startMonth`/`endMonth` fora de `^\d{4}-\d{2}$`.
- `INVALID_VIGENCIA` — `startMonth > endMonth`.

---

## 2. `list_recurring_categories`

Lista as categorias marcadas como recorrentes.

**invoke**: `list_recurring_categories`

**Params**: nenhum.

**Retorno**: `RecurringCategory[]` (ordenado por `category`).

**Erros**: nenhum específico (só falhas de I/O → `String`).

---

## 3. `recurring_suggestions`

Detecção opt-in: retorna candidatos a recorrente (e a encerrar), já filtrando dispensados.

**invoke**: `recurring_suggestions`

**Params**: nenhum (usa histórico local: transações + bank entries).

**Retorno**: `RecurringSuggestion[]`

```ts
interface RecurringSuggestion {
  target: string;        // categoria ou descrição normalizada (== chave de dismiss)
  avg: string;           // Decimal-string, valor médio observado
  months_seen: number;   // meses observados na janela (≥3)
  kind: "start" | "end"; // start = marcar recorrente; end = encerrar recorrente contínua
}
```

**Regra**: `kind="start"` quando o alvo aparece em ≥3/4 meses com variação pequena (CV ≤ 0,15 ou ±15% da mediana) e ainda não é recorrente. `kind="end"` quando uma recorrente contínua deixou de aparecer no mês mais recente importado. Alvos em `dismissed_recurring_suggestions` são omitidos. Nunca marca nada sozinho.

**Erros**: nenhum específico.

---

## 4. `dismiss_recurring_suggestion`

Dispensa (ignora) uma sugestão para que não reapareça.

**invoke**: `dismiss_recurring_suggestion`

**Params**:
| camelCase (TS) | snake_case (Rust) | Tipo | Obrigatório | Notas |
|----------------|-------------------|------|-------------|-------|
| `target`       | `target`          | `string` | sim | Mesmo `target` da sugestão. |

**Retorno**: `void`.

**Efeitos colaterais**: upsert em `dismissed_recurring_suggestions` (idempotente).

**Erros**:
- `"O alvo não pode ficar vazio."` — `target` vazio.

---

## 5. DTOs alterados — `get_dashboard_cmd`

Comando existente (`commands/dashboard.rs`). Sem mudança de assinatura; o `DashboardData` ganha campos.

**Campos adicionados a `DashboardData`**:
```ts
interface DashboardData {
  // ...campos existentes...
  /** Contas fixas derivadas do mês em escopo (fonte de verdade + fallback manual). */
  fixed_expenses: DerivedFixedExpense[];
  /** Teto do cartão (renda recorrente − fixas), Decimal-string. */
  card_ceiling: string;
  /** true quando as fixas usadas no Teto vieram de baseline (estimado, não realizado). */
  card_ceiling_is_baseline: boolean;
}

interface DerivedFixedExpense {
  category: string;
  amount: string;                 // Decimal-string (estornos reduzem)
  origin: "extrato" | "fatura" | "manual" | "baseline";
  status: "realizado" | "estimado" | "suprimido";
}
```

**Semântica**: o total de despesas (`net_total`) continua vindo de cartão + extrato + manuais **não-suprimidos** (fixas derivadas são uma VIEW dos importados já contados — ver research D6, sem dupla contagem). Manual suprimido não soma. `card_ceiling_is_baseline = true` sinaliza o chip "base: média".

---

## 6. DTOs alterados — `get_year_summary_cmd`

Comando existente. Sem mudança de assinatura; `YearSummary` ganha campo(s).

**Campos adicionados a `YearSummary`**:
```ts
interface YearSummary {
  // ...campos existentes (fixed_month, card_ceiling, card_ceiling_salary...)...
  /** true quando fixed_month/Teto do período usaram baseline (estimado). */
  fixed_is_baseline: boolean;
}
// (opcional) YearMonthPoint.fixed_is_baseline?: boolean  // por mês, para o chip na série
```

**Semântica**: `fixed_month` passa a ser fixas derivadas realizadas (ou baseline quando o mês não foi importado) **+** fixos manuais não-suprimidos; recorrências finitas fora da vigência não entram. `card_ceiling` = `max(0, salary_month − fixed_month)` como hoje, mas com `fixed_month` derivado.

---

## Resumo de registro

| Comando | Arquivo | invoke_handler |
|---------|---------|----------------|
| `set_category_recurring` | `commands/recurring.rs` | + |
| `list_recurring_categories` | `commands/recurring.rs` | + |
| `recurring_suggestions` | `commands/recurring.rs` | + |
| `dismiss_recurring_suggestion` | `commands/recurring.rs` | + |
| `get_dashboard_cmd` (DTO+) | `commands/dashboard.rs` | (já registrado) |
| `get_year_summary_cmd` (DTO+) | `commands/dashboard.rs` | (já registrado) |

Wrappers correspondentes em `src/services/tauri.service.ts`; interfaces em `src/types/api.types.ts`.
