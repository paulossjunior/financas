<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { useInvoiceStore } from "@/stores/invoice.store";
import { useSettingsStore } from "@/stores/settings.store";
import type { EntryKind, ManualEntry } from "@/types/api.types";

const store = useInvoiceStore();
const settings = useSettingsStore();

const INCOME_SUGGESTIONS = ["Salário", "Bolsa de Pesquisa", "Rendimentos", "Freelance", "Aluguel Recebido", "Outros"];

// ── form state ──
const kind = ref<EntryKind>("expense");
const description = ref("");
const amount = ref("");
const category = ref("");
const month = ref(currentMonth());
const recurring = ref(true);
const formError = ref<string | null>(null);
const editingId = ref<string | null>(null);

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
});

function resetForm(): void {
  editingId.value = null;
  description.value = "";
  amount.value = "";
  category.value = "";
  month.value = currentMonth();
  recurring.value = true;
  formError.value = null;
}

function startEdit(e: ManualEntry): void {
  editingId.value = e.id;
  kind.value = e.kind;
  description.value = e.description;
  amount.value = e.amount;
  category.value = e.category;
  month.value = e.month;
  recurring.value = e.recurring;
  formError.value = null;
  window.scrollTo({ top: 0, behavior: "smooth" });
}

async function submit(): Promise<void> {
  formError.value = null;
  const amt = parseFloat(amount.value.replace(",", "."));
  if (!description.value.trim()) { formError.value = "Informe uma descrição."; return; }
  if (!category.value.trim()) { formError.value = "Informe uma categoria."; return; }
  if (!(amt > 0)) { formError.value = "Informe um valor maior que zero."; return; }

  const input = {
    kind: kind.value,
    description: description.value.trim(),
    amount: String(amt),
    category: category.value.trim(),
    month: month.value,
    recurring: recurring.value,
  };

  try {
    if (editingId.value) {
      await store.updateManualEntry(editingId.value, input);
    } else {
      await store.addManualEntry(input);
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
        <h1>Receitas & Despesas Fixas</h1>
        <span class="subtitle">Lançamentos que não passam pelo cartão · fixos, variáveis e receitas</span>
      </div>
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

      <div class="form-grid">
        <label class="field field-desc">
          <span>Descrição</span>
          <input v-model="description" type="text" :placeholder="kind === 'income' ? 'Ex: Salário' : 'Ex: Aluguel'" @keyup.enter="submit" />
        </label>

        <label class="field">
          <span>Valor (R$)</span>
          <input v-model="amount" type="text" inputmode="decimal" placeholder="0,00" @keyup.enter="submit" />
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
      </div>

      <p class="hint-line">
        💡 Contas que <strong>variam</strong> (água, luz, gás): <strong>desmarque "Fixo"</strong>, escolha o
        <strong>mês</strong> e lance o valor real daquele mês. Todo mês você adiciona um novo lançamento com o valor da conta.
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
.page { padding: 1.5rem 2rem; max-width: 1200px; margin: 0 auto; }
.page-header { margin-bottom: 1.25rem; }
.page-title { display: flex; flex-direction: column; gap: 0.25rem; }
h1 { font-size: 1.25rem; font-weight: 600; color: var(--clr-text-primary); letter-spacing: -0.01em; }
.subtitle { font-size: 0.8125rem; color: var(--clr-text-secondary); }

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
