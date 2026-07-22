<script setup lang="ts">
// Manual entries page (Fixos & Renda) — add/edit fixed expenses and extra income (salary is read-only from the payslip).
import { onMounted, ref, computed, watch } from "vue";
import { useInvoiceStore } from "@/stores/invoice.store";
import { useSettingsStore } from "@/stores/settings.store";
import { listPayslips, listFixedExpenses } from "@/services/tauri.service";
import { maskMoney, parseMoneyBR } from "@/utils/money";
import type { EntryKind, ManualEntry, Payslip, DerivedFixed } from "@/types/api.types";

const store = useInvoiceStore();
const settings = useSettingsStore();

// Salary now comes from the payslip (Contracheque); manual income here is EXTRA income only.
const INCOME_SUGGESTIONS = ["Bolsa de Pesquisa", "Rendimentos", "Freelance", "Aluguel Recebido", "Outros"];

// ── form state ──
const kind = ref<EntryKind>("expense");
const description = ref("");
const amount = ref("");
const category = ref("");
const month = ref(currentMonth());
const recurring = ref(true);
const repeatMonths = ref(1); // sporadic expense: repeat across N consecutive months
const formError = ref<string | null>(null);
const editingId = ref<string | null>(null);

function addMonths(ym: string, n: number): string {
  const [y, m] = ym.split("-").map(Number);
  const idx = (y * 12 + (m - 1)) + n;
  return `${Math.floor(idx / 12)}-${String((idx % 12) + 1).padStart(2, "0")}`;
}

// Salary from the latest payslip (read-only here).
const payslips = ref<Payslip[]>([]);
const latestPayslip = computed(() => payslips.value[0] ?? null); // list is month DESC

function currentMonth(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
}

const expenseCategories = computed(() => settings.categoryGroups.map((g) => g.name));
const categorySuggestions = computed(() =>
  kind.value === "income" ? INCOME_SUGGESTIONS : expenseCategories.value
);

const incomeEntries = computed(() => store.manualEntries.filter((e) => e.kind === "income"));
const expenseEntries = computed(() => store.manualEntries.filter((e) => e.kind === "expense"));

// Derived fixed expenses: categories marked recurring turn imported data (extrato/fatura)
// into contas fixas automatically. Read-only here; edit recurrence in Categorias.
const fixedMonth = ref(currentMonth());
const derivedFixed = ref<DerivedFixed[]>([]);
const derivedExpense = computed(() => derivedFixed.value.filter((f) => f.kind === "expense"));
const derivedIncome = computed(() => derivedFixed.value.filter((f) => f.kind === "income"));
const sumDerived = (list: DerivedFixed[]) => list.reduce((a, f) => a + (parseFloat(f.amount) || 0), 0);
const ORIGIN_LABEL: Record<string, string> = { extrato: "Extrato", fatura: "Fatura", baseline: "Base", manual: "Manual" };
async function loadDerived(): Promise<void> {
  try { derivedFixed.value = await listFixedExpenses(fixedMonth.value); } catch { derivedFixed.value = []; }
}
watch(fixedMonth, loadDerived);

const totalIncome = computed(() => sum(incomeEntries.value));
const totalExpense = computed(() => sum(expenseEntries.value));

function sum(entries: ManualEntry[]): number {
  return entries.reduce((acc, e) => acc + (parseFloat(e.amount) || 0), 0);
}

function formatBRL(v: number | string): string {
  const n = typeof v === "string" ? parseFloat(v) || 0 : v;
  return n.toLocaleString("pt-BR", { style: "currency", currency: "BRL", minimumFractionDigits: 2 });
}

const MONTHS = ["Jan","Fev","Mar","Abr","Mai","Jun","Jul","Ago","Set","Out","Nov","Dez"];
function formatMonth(m: string): string {
  const [y, mo] = m.split("-");
  return `${MONTHS[parseInt(mo) - 1] ?? mo}/${y}`;
}

onMounted(async () => {
  await settings.loadConfig();
  await store.loadManualEntries();
  try { payslips.value = await listPayslips(); } catch { /* ignore */ }
  await loadDerived();
});

function resetForm(): void {
  editingId.value = null;
  description.value = "";
  amount.value = "";
  category.value = "";
  month.value = currentMonth();
  recurring.value = true;
  repeatMonths.value = 1;
  formError.value = null;
}

function startEdit(e: ManualEntry): void {
  editingId.value = e.id;
  kind.value = e.kind;
  description.value = e.description;
  amount.value = (parseFloat(e.amount) || 0).toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  category.value = e.category;
  month.value = e.month;
  recurring.value = e.recurring;
  formError.value = null;
  window.scrollTo({ top: 0, behavior: "smooth" });
}

async function submit(): Promise<void> {
  formError.value = null;
  const amt = parseMoneyBR(amount.value);
  if (!description.value.trim()) { formError.value = "Informe uma descrição."; return; }
  if (!category.value.trim()) { formError.value = "Informe uma categoria."; return; }
  if (!(amt > 0)) { formError.value = "Informe um valor maior que zero."; return; }

  const base = {
    kind: kind.value,
    description: description.value.trim(),
    amount: String(amt),
    category: category.value.trim(),
    recurring: recurring.value,
    // Manual income here is always EXTRA (non-salary); salary comes from the payslip.
    isSalary: false,
  };
  // Sporadic expense: repeat as one-off entries across N consecutive months.
  const n = !recurring.value && !editingId.value ? Math.max(1, Math.min(60, Math.floor(repeatMonths.value) || 1)) : 1;

  try {
    if (editingId.value) {
      await store.updateManualEntry(editingId.value, { ...base, month: month.value });
    } else {
      for (let i = 0; i < n; i++) {
        await store.addManualEntry({ ...base, month: addMonths(month.value, i) });
      }
    }
    resetForm();
  } catch (e) {
    formError.value = String(e instanceof Error ? e.message : e);
  }
}

async function remove(id: string): Promise<void> {
  await store.removeManualEntry(id);
  if (editingId.value === id) resetForm();
}
</script>

<template>
  <div class="page">
    <div class="page-header">
      <div class="page-title">
        <h1>Despesas Fixas & Renda Extra</h1>
        <span class="subtitle">Contas fixas e renda que não é salário. O salário vem do Contracheque.</span>
      </div>
    </div>

    <!-- Salary from payslip (read-only) -->
    <div class="salary-card">
      <div class="sc-l">
        <span class="sc-lbl">Salário (do contracheque)</span>
        <span class="sc-val" v-if="latestPayslip">{{ formatBRL(latestPayslip.net) }} <small>líq · {{ formatMonth(latestPayslip.month) }}</small></span>
        <span class="sc-val muted" v-else>nenhum contracheque importado</span>
      </div>
      <RouterLink class="sc-link" to="/contracheque">📄 {{ latestPayslip ? "Ver contracheques" : "Importar contracheque" }}</RouterLink>
    </div>

    <!-- Derived fixed expenses from recurring categories (read-only) -->
    <div class="card derived-card">
      <div class="list-head">
        <h2>Recorrentes derivados <span class="dv-mut">· do extrato / fatura · crédito e débito</span></h2>
        <input v-model="fixedMonth" type="month" class="dv-month" aria-label="Mês das contas fixas derivadas" />
      </div>
      <p class="dv-note">
        Categorias marcadas como <RouterLink to="/categorias">recorrentes</RouterLink> viram contas fixas automaticamente,
        a partir do que foi importado — ou do <strong>valor base</strong> quando o mês ainda não tem dados. Contadas uma
        vez só (não duplicam com os lançamentos manuais abaixo).
      </p>
      <template v-if="derivedFixed.length">
        <!-- Receitas recorrentes (crédito) -->
        <template v-if="derivedIncome.length">
          <div class="dv-sub"><span class="income-text">↑ Receitas recorrentes</span></div>
          <ul class="entry-list">
            <li v-for="f in derivedIncome" :key="'in-' + f.category" class="entry">
              <div class="entry-main">
                <span class="entry-desc">{{ f.category }}</span>
                <span class="entry-meta">
                  <span class="chip">{{ ORIGIN_LABEL[f.origin] ?? f.origin }}</span>
                  <span class="badge">{{ f.is_baseline ? "estimado (base)" : "realizado" }}</span>
                </span>
              </div>
              <span class="entry-amount income-text">{{ formatBRL(f.amount) }}</span>
            </li>
          </ul>
        </template>
        <!-- Contas fixas (débito) -->
        <template v-if="derivedExpense.length">
          <div class="dv-sub"><span class="expense-text">↓ Contas fixas</span></div>
          <ul class="entry-list">
            <li v-for="f in derivedExpense" :key="'ex-' + f.category" class="entry">
              <div class="entry-main">
                <span class="entry-desc">{{ f.category }}</span>
                <span class="entry-meta">
                  <span class="chip">{{ ORIGIN_LABEL[f.origin] ?? f.origin }}</span>
                  <span class="badge">{{ f.is_baseline ? "estimado (base)" : "realizado" }}</span>
                </span>
              </div>
              <span class="entry-amount expense-text">{{ formatBRL(f.amount) }}</span>
            </li>
          </ul>
        </template>
        <div class="dv-total">
          <span>{{ formatMonth(fixedMonth) }}</span>
          <span class="dv-tots">
            <span v-if="derivedIncome.length" class="income-text">+ {{ formatBRL(sumDerived(derivedIncome)) }}</span>
            <strong class="expense-text">− {{ formatBRL(sumDerived(derivedExpense)) }}</strong>
          </span>
        </div>
      </template>
      <p v-else class="empty">
        Nenhuma conta fixa/receita derivada em {{ formatMonth(fixedMonth) }}. Marque categorias como recorrentes em
        <RouterLink to="/categorias">Categorias</RouterLink>.
      </p>
    </div>

    <!-- Form -->
    <div class="card form-card">
      <h2>{{ editingId ? "Editar lançamento" : "Novo lançamento" }}</h2>

      <div class="kind-toggle">
        <button
          type="button"
          :class="['kind-btn', { active: kind === 'income', income: true }]"
          @click="kind = 'income'"
        >↑ Receita</button>
        <button
          type="button"
          :class="['kind-btn', { active: kind === 'expense', expense: true }]"
          @click="kind = 'expense'"
        >↓ Despesa</button>
      </div>

      <p v-if="kind === 'income'" class="income-note">Renda extra (bolsa, rendimentos, freelance). O salário é importado no <RouterLink to="/contracheque">Contracheque</RouterLink>.</p>

      <div class="form-grid">
        <label class="field field-desc">
          <span>Descrição</span>
          <input v-model="description" type="text" :placeholder="kind === 'income' ? 'Ex: Bolsa de Pesquisa' : 'Ex: Aluguel'" @keyup.enter="submit" />
        </label>

        <label class="field">
          <span>Valor (R$)</span>
          <input v-model="amount" type="text" inputmode="numeric" placeholder="0,00" @input="amount = maskMoney(amount)" @keyup.enter="submit" />
        </label>

        <label class="field">
          <span>Categoria</span>
          <input v-model="category" type="text" list="cat-suggestions" placeholder="Categoria" @keyup.enter="submit" />
          <datalist id="cat-suggestions">
            <option v-for="c in categorySuggestions" :key="c" :value="c" />
          </datalist>
        </label>

        <label class="field field-month">
          <span>Mês</span>
          <input v-model="month" type="month" />
        </label>

        <label class="field field-recurring">
          <input v-model="recurring" type="checkbox" />
          <span><strong>Fixo</strong> — mesmo valor todo mês (aluguel, internet, plano)</span>
        </label>

        <label v-if="!recurring && !editingId" class="field field-repeat">
          <span>Repetir por</span>
          <div class="repeat-in">
            <input v-model.number="repeatMonths" type="number" min="1" max="60" />
            <span>{{ repeatMonths > 1 ? "meses" : "mês" }}</span>
          </div>
        </label>
      </div>

      <p class="hint-line">
        💡 <strong>Fixo</strong> = todo mês (aluguel, plano). <strong>Avulso/esporádico</strong> = desmarque "Fixo",
        escolha o mês e, se durar alguns meses (ex.: psicólogo por 3 meses), use <strong>"Repetir por N meses"</strong>
        — cria um lançamento em cada mês a partir do escolhido.
      </p>

      <div v-if="formError" class="form-error">⚠ {{ formError }}</div>

      <div class="form-actions">
        <button v-if="editingId" type="button" class="btn btn-ghost" @click="resetForm">Cancelar</button>
        <button type="button" class="btn btn-primary" :disabled="store.loading" @click="submit">
          {{ editingId ? "Salvar alterações" : "Adicionar" }}
        </button>
      </div>
    </div>

    <!-- Lists -->
    <div class="lists">
      <div class="card list-card">
        <div class="list-head">
          <h2 class="income-text">Receitas</h2>
          <strong class="income-text">{{ formatBRL(totalIncome) }}</strong>
        </div>
        <ul v-if="incomeEntries.length" class="entry-list">
          <li v-for="e in incomeEntries" :key="e.id" class="entry">
            <div class="entry-main">
              <span class="entry-desc">{{ e.description }}</span>
              <span class="entry-meta">
                <span class="chip">{{ e.category }}</span>
                <span v-if="e.is_salary" class="badge sal" title="O salário agora vem do contracheque — pode remover este lançamento">salário manual · use o contracheque</span>
                <span class="badge">{{ e.recurring ? "fixo · todo mês" : formatMonth(e.month) }}</span>
              </span>
            </div>
            <span class="entry-amount income-text">{{ formatBRL(e.amount) }}</span>
            <div class="entry-actions">
              <button class="icon-btn" title="Editar" @click="startEdit(e)">✎</button>
              <button class="icon-btn danger" title="Remover" @click="remove(e.id)">✕</button>
            </div>
          </li>
        </ul>
        <p v-else class="empty">Nenhuma receita cadastrada.</p>
      </div>

      <div class="card list-card">
        <div class="list-head">
          <h2 class="expense-text">Despesas (fora do cartão)</h2>
          <strong class="expense-text">{{ formatBRL(totalExpense) }}</strong>
        </div>
        <ul v-if="expenseEntries.length" class="entry-list">
          <li v-for="e in expenseEntries" :key="e.id" class="entry">
            <div class="entry-main">
              <span class="entry-desc">{{ e.description }}</span>
              <span class="entry-meta">
                <span class="chip">{{ e.category }}</span>
                <span class="badge">{{ e.recurring ? "fixo · todo mês" : formatMonth(e.month) }}</span>
              </span>
            </div>
            <span class="entry-amount expense-text">{{ formatBRL(e.amount) }}</span>
            <div class="entry-actions">
              <button class="icon-btn" title="Editar" @click="startEdit(e)">✎</button>
              <button class="icon-btn danger" title="Remover" @click="remove(e.id)">✕</button>
            </div>
          </li>
        </ul>
        <p v-else class="empty">Nenhuma despesa fixa cadastrada.</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page { padding: 1.5rem 2rem; max-width: 1320px; margin: 0 auto; }
.page-header { margin-bottom: 1.25rem; }
.page-title { display: flex; flex-direction: column; gap: 0.25rem; }
h1 { font-size: 1.25rem; font-weight: 600; color: var(--clr-text-primary); letter-spacing: -0.01em; }
.subtitle { font-size: 0.8125rem; color: var(--clr-text-secondary); }

.salary-card {
  display: flex; align-items: center; justify-content: space-between; gap: 1rem; flex-wrap: wrap;
  background: var(--clr-accent-light); border: 1px solid var(--clr-accent);
  border-radius: var(--radius-lg); padding: .8rem 1.1rem; margin-bottom: 1rem;
}
.sc-l { display: flex; flex-direction: column; gap: 2px; }
.sc-lbl { font-size: 10.5px; font-weight: 700; letter-spacing: .05em; text-transform: uppercase; color: var(--clr-accent); }
.sc-val { font-size: 1.15rem; font-weight: 780; color: var(--clr-text-primary); }
.sc-val small { font-size: .72rem; font-weight: 600; color: var(--clr-text-muted); }
.sc-val.muted { font-size: .9rem; font-weight: 600; color: var(--clr-text-muted); }
.sc-link { font-size: .82rem; font-weight: 700; color: var(--clr-accent); text-decoration: none; white-space: nowrap; }
.sc-link:hover { text-decoration: underline; }
.income-note { font-size: .78rem; color: var(--clr-text-muted); margin: 0 0 1rem; }
.income-note a { color: var(--clr-accent); }

/* Derived fixed expenses card */
.derived-card { margin-bottom: 1rem; }
.dv-mut { font-weight: 400; color: var(--clr-text-muted); font-size: 0.8rem; }
.dv-month { font-family: var(--font-body); font-size: 0.8125rem; padding: 0.35rem 0.55rem; border: 1px solid var(--clr-stroke); border-radius: var(--radius-md); background: var(--clr-surface); color: var(--clr-text-primary); outline: none; }
.dv-month:focus { border-color: var(--clr-accent); }
.dv-note { font-size: 0.78rem; color: var(--clr-text-secondary); margin: 0 0 0.75rem; line-height: 1.5; }
.dv-note a { color: var(--clr-accent); }
.dv-note strong { color: var(--clr-text-primary); }
.dv-total { display: flex; align-items: baseline; justify-content: space-between; margin-top: 0.75rem; padding-top: 0.75rem; border-top: 2px solid var(--clr-stroke); font-size: 0.8125rem; font-weight: 600; color: var(--clr-text-secondary); }
.dv-total strong { font-size: 0.95rem; font-variant-numeric: tabular-nums; }
.dv-tots { display: flex; gap: 1rem; align-items: baseline; font-variant-numeric: tabular-nums; }
.dv-sub { font-size: 0.72rem; font-weight: 700; text-transform: uppercase; letter-spacing: .04em; margin: 0.6rem 0 0.15rem; }

.card {
  background: var(--clr-surface);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-lg);
  padding: 1.25rem 1.5rem;
  box-shadow: var(--shadow-sm);
}
h2 { font-size: 0.9375rem; font-weight: 600; color: var(--clr-text-primary); margin-bottom: 1rem; }

/* Form */
.form-card { margin-bottom: 1rem; }
.kind-toggle { display: inline-flex; gap: 0; border: 1px solid var(--clr-stroke); border-radius: var(--radius-md); overflow: hidden; margin-bottom: 1rem; }
.kind-btn {
  padding: 0.5rem 1.1rem; background: var(--clr-surface); border: none; cursor: pointer;
  font-family: var(--font-body); font-size: 0.8125rem; font-weight: 600; color: var(--clr-text-secondary);
  transition: background 0.1s, color 0.1s;
}
.kind-btn:hover { background: var(--clr-surface-alt); }
.kind-btn.income.active { background: var(--clr-positive); color: #fff; }
.kind-btn.expense.active { background: var(--clr-negative); color: #fff; }

.salbonus { display: flex; align-items: center; gap: .5rem; margin-bottom: 1rem; flex-wrap: wrap; }
.sb-label { font-size: 0.8125rem; font-weight: 600; color: var(--clr-text-secondary); }
.sb-btn {
  font-family: inherit; font-size: 0.8125rem; font-weight: 600; padding: 4px 12px;
  border: 1px solid var(--clr-stroke); border-radius: var(--radius-md);
  background: var(--clr-surface); color: var(--clr-text-secondary); cursor: pointer;
}
.sb-btn.active { background: var(--clr-accent); color: #fff; border-color: var(--clr-accent); }
.sb-hint { font-size: 0.72rem; color: var(--clr-text-muted); }

.form-grid { display: grid; grid-template-columns: 2fr 1fr 1.5fr 1fr; gap: 0.75rem 1rem; align-items: end; }
.field { display: flex; flex-direction: column; gap: 0.3rem; }
.field > span { font-size: 0.6875rem; font-weight: 600; color: var(--clr-text-muted); text-transform: uppercase; letter-spacing: 0.04em; }
.field input[type="text"], .field input[type="month"] {
  font-family: var(--font-body); font-size: 0.875rem; padding: 0.5rem 0.65rem;
  border: 1px solid var(--clr-stroke); border-radius: var(--radius-md); background: var(--clr-surface);
  color: var(--clr-text-primary); outline: none; transition: border-color 0.1s;
}
.field input:focus { border-color: var(--clr-accent); }
.field-recurring { flex-direction: row; align-items: center; gap: 0.4rem; grid-column: 1 / -1; }
.field-recurring span { font-size: 0.8125rem; color: var(--clr-text-secondary); text-transform: none; letter-spacing: 0; font-weight: 400; }
.field-recurring input { width: 16px; height: 16px; accent-color: var(--clr-accent); }
.field-repeat { grid-column: 1 / -1; }
.repeat-in { display: flex; align-items: center; gap: .5rem; }
.repeat-in input { width: 72px; padding: 0.45rem 0.75rem; border: 1px solid var(--clr-stroke); border-radius: var(--radius-md); font-size: 0.875rem; font-family: var(--font-body); color: var(--clr-text-primary); background: var(--clr-bg); outline: none; }
.repeat-in span { font-size: 0.8125rem; color: var(--clr-text-secondary); }

.hint-line { margin-top: 0.85rem; font-size: 0.8125rem; color: var(--clr-text-secondary); background: var(--clr-surface-alt); border: 1px solid var(--clr-stroke); border-radius: var(--radius-md); padding: 0.6rem 0.8rem; line-height: 1.5; }
.hint-line strong { color: var(--clr-text-primary); font-weight: 700; }
.form-error { margin-top: 0.75rem; font-size: 0.8125rem; color: var(--clr-negative); }
.form-actions { display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 1rem; }
.btn { font-family: var(--font-body); font-size: 0.8125rem; font-weight: 600; padding: 0.5rem 1.1rem; border-radius: var(--radius-md); cursor: pointer; border: 1px solid transparent; transition: background 0.1s; }
.btn-primary { background: var(--clr-accent); color: #fff; }
.btn-primary:hover { background: var(--clr-accent-hover); }
.btn-primary:disabled { opacity: 0.5; cursor: default; }
.btn-ghost { background: var(--clr-surface); border-color: var(--clr-stroke); color: var(--clr-text-secondary); }
.btn-ghost:hover { background: var(--clr-surface-alt); }

/* Lists */
.lists { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.list-head { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 0.75rem; }
.list-head h2 { margin: 0; }
.income-text { color: var(--clr-positive); }
.expense-text { color: var(--clr-negative); }
.entry-list { list-style: none; display: flex; flex-direction: column; }
.entry { display: grid; grid-template-columns: 1fr auto auto; gap: 0.75rem; align-items: center; padding: 0.6rem 0; border-bottom: 1px solid var(--clr-stroke-soft); }
.entry:last-child { border-bottom: none; }
.entry-main { display: flex; flex-direction: column; gap: 0.25rem; min-width: 0; }
.entry-desc { font-size: 0.875rem; font-weight: 500; color: var(--clr-text-primary); }
.entry-meta { display: flex; gap: 0.4rem; align-items: center; flex-wrap: wrap; }
.chip { font-size: 0.6875rem; background: var(--clr-accent-light); color: var(--clr-accent); padding: 0.1rem 0.5rem; border-radius: 100px; font-weight: 600; }
.badge { font-size: 0.6875rem; color: var(--clr-text-muted); }
.badge.sal { color: var(--clr-accent); font-weight: 700; }
.badge.bon { color: var(--clr-amber); font-weight: 700; }
.entry-amount { font-size: 0.9375rem; font-weight: 600; font-variant-numeric: tabular-nums; white-space: nowrap; }
.entry-actions { display: flex; gap: 0.15rem; }
.icon-btn { background: none; border: none; cursor: pointer; color: var(--clr-text-muted); font-size: 0.875rem; padding: 0.25rem 0.4rem; border-radius: var(--radius-sm); line-height: 1; }
.icon-btn:hover { background: var(--clr-surface-alt); color: var(--clr-text-primary); }
.icon-btn.danger:hover { color: var(--clr-negative); background: #fde7e9; }
.empty { font-size: 0.8125rem; color: var(--clr-text-muted); padding: 0.5rem 0; }

@media (max-width: 860px) {
  .form-grid { grid-template-columns: 1fr 1fr; }
  .lists { grid-template-columns: 1fr; }
}
</style>
