# Data Model: Categorias recorrentes + baseline + anti-duplicação

Dinheiro em `rust_decimal::Decimal`, serializado como **string** na fronteira IPC
(`parseFloat` só para exibir). Meses em ISO `YYYY-MM`.

## 1. Tabelas SQLite (novas)

### `recurring_categories`

| Coluna        | Tipo | Nulo | Descrição |
|---------------|------|------|-----------|
| `category`    | TEXT | não  | **PK**. Nome da categoria marcada como recorrente (ex.: "Moradia & Serviços"). |
| `start_month` | TEXT | sim  | Início da vigência `YYYY-MM`. `NULL` = sem limite inferior. |
| `end_month`   | TEXT | sim  | Fim da vigência `YYYY-MM` (inclusivo). `NULL` = ongoing (sem fim). |
| `created_at`  | TEXT | não  | Timestamp ISO da marcação (auditoria/determinismo de ordenação). |

- Vigência: `NULL/NULL` = contínua; ambos definidos = finita (ex.: psicólogo jan–mar). Se só `end_month` for definido, conta de sempre até o fim; se só `start_month`, do início em diante.
- Upsert idempotente: `INSERT ... ON CONFLICT(category) DO UPDATE`.
- Desmarcar = `DELETE FROM recurring_categories WHERE category = ?`.

**Validação**:
- `category` não vazia (trim).
- `start_month`/`end_month`, se presentes, casam `^\d{4}-\d{2}$` (mês 01–12).
- Se ambos presentes, `start_month ≤ end_month` (senão erro `INVALID_VIGENCIA`).

### `dismissed_recurring_suggestions`

| Coluna    | Tipo | Nulo | Descrição |
|-----------|------|------|-----------|
| `target`  | TEXT | não  | **PK**. Alvo dispensado — nome da categoria ou descrição normalizada. |

- Filtrada de `recurring_suggestions`. Dispensar = upsert; aceitar/marcar não escreve aqui (marca em `recurring_categories`).

## 2. Entidades de domínio (Rust, `domain/recurring.rs`)

### `RecurringCategory` (persistida)

```
category:    String
start_month: Option<String>  // "YYYY-MM"
end_month:   Option<String>  // "YYYY-MM", inclusivo
created_at:  String
```

Métodos puros:
- `is_active(month) -> bool`: `start.map_or(true, |s| month >= s) && end.map_or(true, |e| month <= e)`.

### `DerivedFixedExpense` (derivada / DTO)

Resultado do cálculo por (categoria recorrente × mês).

```
category: String
amount:   String   // Decimal-string; soma líquida do mês (estornos reduzem)
origin:   Origin   // "extrato" | "fatura" | "manual" | "baseline"
status:   Status   // "realizado" | "estimado" | "suprimido"
```

- `Origin`:
  - `fatura` — soma vem de transações de cartão na categoria+mês.
  - `extrato` — soma vem de débitos de `BankEntry` na categoria+mês.
  - `manual` — fallback: fixo manual mantido (sem importado equivalente).
  - `baseline` — sem dado importado no mês → média (ver `BaselineValue`).
  - Quando há origens mistas (extrato **e** fatura no mesmo mês/categoria), a soma total é a fixa; a UI pode listar as parcelas por origem. O total do mês é `extrato + fatura`.
- `Status`:
  - `realizado` — valor vem de import do próprio mês.
  - `estimado` — valor vem do baseline (`is_baseline = true`).
  - `suprimido` — fixo manual substituído por importado equivalente (mostrado riscado; não soma).

### `BaselineValue`

```
category:    String
amount:      String  // Decimal-string; média dos últimos N=3 meses realizados
months_used: u32     // quantos meses entraram na média (0..=3)
is_baseline: bool    // sempre true nesta estrutura; propagado aos DTOs de painel
```

- Sem histórico → `amount = "0"`, `months_used = 0`.
- `< 3` meses → média dos disponíveis (`months_used = 1|2`), base parcial.

### `RecurringSuggestion` (DTO)

```
target:      String   // categoria ou descrição normalizada (== PK de dismissed)
avg:         String   // Decimal-string, valor médio observado
months_seen: u32      // meses em que o alvo apareceu na janela (≥3)
kind:        SuggestionKind  // "start" (marcar recorrente) | "end" (encerrar recorrente)
```

- `start`: alvo aparece em ≥3/4 meses com pouca variação e ainda não é recorrente (FR-010).
- `end`: recorrente contínua que **deixou de aparecer** no mês mais recente importado → sugerir encerrar (FR-014, US5-3).
- Sugestões cujo `target` está em `dismissed_recurring_suggestions` são omitidas.

## 3. Funções puras (assinaturas de referência)

```
derive_fixed_expenses(
    month: &str,
    transactions: &[Transaction],   // cartão (fatura)
    bank_entries: &[BankEntry],     // extrato (débitos)
    manual: &[ManualEntry],         // fixos manuais (fallback/override)
    recurring: &[RecurringCategory],
) -> DerivationResult
// DerivationResult { fixed: Vec<DerivedFixedExpense>, suppressed_manual_ids: Vec<Uuid> }

baseline(category: &str, history: &[(String /*month*/, Decimal)], n: usize /*=3*/) -> BaselineValue

detect_recurring(history: &[HistoryPoint], dismissed: &[String]) -> Vec<RecurringSuggestion>
```

- Determinísticas: mesma entrada → mesma saída (ordenação estável por categoria).
- Só contam categorias `is_active(month)` (vigência) — regra aplicada antes da soma e do baseline.

## 4. Campos adicionados aos DTOs de painel

Ver [contracts/commands.md](contracts/commands.md) para o shape completo.

### `DashboardData` (+)

```
fixed_expenses:      Vec<DerivedFixedExpense>  // contas fixas derivadas do mês em escopo
card_ceiling:        String   // Teto (renda recorrente − fixas)   [já pode existir no ano; espelhar no mês]
card_ceiling_is_baseline: bool // true quando as fixas do Teto vieram de baseline (estimado)
```

### `YearSummary` (+)

```
fixed_is_baseline:   bool   // true quando fixed_month/Teto do período usaram baseline
// (opcional por mês) YearMonthPoint.fixed_is_baseline: bool
```

- `card_ceiling` / `card_ceiling_salary` já existem em `YearSummary`. A mudança é: `fixed_month` passa a ser **fixas derivadas realizadas** (ou baseline quando não importado) **+** fixos manuais não-suprimidos; e um flag sinaliza estimado.

## 5. Transições de estado

### Supersede (anti-duplicação) — por (categoria recorrente × mês)

```
manual fixo existe, sem importado equivalente        → status = manual/realizado (conta)
manual fixo existe, COM importado equivalente no mês  → status = suprimido (não conta);
                                                        importado vira a fixa (realizado)
importado existe, sem manual                          → status = realizado (origem extrato/fatura)
nada importado, com histórico                         → status = estimado (origem baseline)
nada importado, sem histórico                         → fixa = 0 (não inventa valor)
```

Espelha `payslip → salário manual` (`get_dashboard.rs` / `year.rs`).

### Vigência (recorrente finita, mês `m`)

```
m < start_month                → excluída (não conta em fixas/baseline/Teto)
start_month ≤ m ≤ end_month     → ativa (conta normalmente)
m > end_month                   → excluída, inclusive em recálculo histórico
sem start/end (contínua)        → sempre ativa até ser desmarcada/encerrada
```

### Sugestão

```
detectada (≥3/4, variação ≤ limite) e não dispensada  → aparece (kind=start)
usuário "Marcar"                                       → INSERT recurring_categories; some da lista
usuário "Ignorar"                                      → INSERT dismissed_recurring_suggestions; não reaparece
recorrente contínua sumiu no mês mais recente          → aparece (kind=end) → "Encerrar" define end_month
```

## 6. Reuso / integração

- `Transaction`, `Invoice` (fatura), `BankEntry` (extrato), `ManualEntry`, `Categorizer` já existem — a derivação os consome, não os altera.
- `bank_entries.month` e `manual_entries.month` já são `YYYY-MM`; transações de cartão usam `date` → derivar mês por `date.format("%Y-%m")` (consistente com `year.rs`).
- Estornos: transações com `is_reversal = true` entram com sinal negativo na soma da fixa do mês (aritmética exata), como em `compute_year_summary`.
