<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import type { Transaction } from "@/types/api.types";
import { getAllTransactions, overrideTransactionCategory, removeTransactionOverride } from "@/services/tauri.service";
import { useSettingsStore } from "@/stores/settings.store";
import MoneyAmount from "@/components/shared/MoneyAmount.vue";
import TransactionCategoryOverride from "@/components/settings/TransactionCategoryOverride.vue";

const settingsStore = useSettingsStore();
const transactions = ref<Transaction[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const search = ref("");
const categoryFilter = ref("");
const monthFilter = ref("");

const availableCategories = computed(() =>
  settingsStore.categoryGroups.map((g) => g.name)
);

const transactionOverrides = computed(() =>
  settingsStore.config?.transaction_overrides ?? {}
);

async function load() {
  loading.value = true;
  try {
    await settingsStore.loadConfig();
    transactions.value = await getAllTransactions();
  } catch (e) {
    error.value = String(e instanceof Error ? e.message : e);
  } finally {
    loading.value = false;
  }
}

onMounted(load);

async function handleOverride(txId: string, category: string) {
  await overrideTransactionCategory(txId, category);
  await load();
}

async function handleRemoveOverride(txId: string) {
  await removeTransactionOverride(txId);
  await load();
}

const MONTH_NAMES = ["Janeiro","Fevereiro","Março","Abril","Maio","Junho","Julho","Agosto","Setembro","Outubro","Novembro","Dezembro"];

function monthLabel(ym: string): string {
  const [y, m] = ym.split("-");
  return `${MONTH_NAMES[parseInt(m) - 1]} ${y}`;
}

const availableMonths = computed(() => {
  const set = new Set(transactions.value.map((t) => t.date.slice(0, 7)));
  return Array.from(set).sort().reverse();
});

const categories = computed(() => {
  const base = monthFilter.value
    ? transactions.value.filter((t) => t.date.startsWith(monthFilter.value))
    : transactions.value;
  const cats = new Set(base.map((t) => t.category));
  return Array.from(cats).sort();
});

const filtered = computed(() => {
  const q = search.value.toLowerCase().trim();
  const cat = categoryFilter.value;
  const month = monthFilter.value;
  return transactions.value.filter((t) => {
    const matchSearch = !q || t.description.toLowerCase().includes(q);
    const matchCat = !cat || t.category === cat;
    const matchMonth = !month || t.date.startsWith(month);
    return matchSearch && matchCat && matchMonth;
  });
});

function formatDate(iso: string): string {
  const [y, m, d] = iso.split("-");
  return `${d}/${m}/${y}`;
}

function formatInstallment(t: Transaction): string {
  if (!t.installment) return "";
  return ` (${t.installment.current}/${t.installment.total})`;
}
</script>

<template>
  <div class="page">
    <div class="page-header">
      <h1 class="page-title">Despesas</h1>
      <span v-if="!loading" class="count-badge">{{ filtered.length }} transações</span>
    </div>

    <div v-if="loading" class="loading">Carregando…</div>
    <div v-else-if="error" class="error-msg">{{ error }}</div>

    <template v-else>
      <div class="filters card">
        <input
          v-model="search"
          class="search-input"
          placeholder="Buscar por descrição…"
          data-testid="tx-search"
        />
        <select v-model="monthFilter" class="cat-filter" data-testid="tx-month-filter" @change="categoryFilter = ''">
          <option value="">Todos os meses</option>
          <option v-for="m in availableMonths" :key="m" :value="m">{{ monthLabel(m) }}</option>
        </select>
        <select v-model="categoryFilter" class="cat-filter" data-testid="tx-cat-filter">
          <option value="">Todas categorias</option>
          <option v-for="cat in categories" :key="cat" :value="cat">{{ cat }}</option>
        </select>
      </div>

      <div v-if="transactions.length === 0" class="empty-state">
        <div class="empty-icon">📂</div>
        <p>Nenhuma fatura importada. Importe uma fatura no Dashboard para ver as despesas.</p>
      </div>

      <div v-else class="card table-card">
        <table>
          <thead>
            <tr>
              <th>Data</th>
              <th>Descrição</th>
              <th>Categoria</th>
              <th class="right">Valor</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="tx in filtered"
              :key="tx.id"
              :class="{ reversal: tx.is_reversal }"
            >
              <td class="date-cell">{{ formatDate(tx.date) }}</td>
              <td class="desc">{{ tx.description }}{{ formatInstallment(tx) }}</td>
              <td>
                <TransactionCategoryOverride
                  v-if="availableCategories.length > 0"
                  :transactionId="tx.id"
                  :currentCategory="tx.category"
                  :availableCategories="availableCategories"
                  :hasOverride="!!(transactionOverrides[tx.id])"
                  @override="handleOverride"
                  @removeOverride="handleRemoveOverride"
                />
                <span v-else class="cat-badge">{{ tx.category }}</span>
              </td>
              <td class="right"><MoneyAmount :amount="tx.amount" /></td>
            </tr>
          </tbody>
        </table>
        <div v-if="filtered.length === 0" class="no-results">
          Nenhum resultado para os filtros aplicados.
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.page {
  padding: 1.5rem 2rem;
  max-width: 1320px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  align-items: baseline;
  gap: 0.75rem;
  margin-bottom: 1.25rem;
}

.page-title {
  font-size: 1.375rem;
  font-weight: 700;
  color: var(--clr-text-primary);
  letter-spacing: -0.02em;
  margin: 0;
}

.count-badge {
  font-size: 0.75rem;
  color: var(--clr-text-muted);
  background: var(--clr-stroke-soft);
  padding: 2px 8px;
  border-radius: 100px;
}

.card {
  background: var(--clr-surface);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-sm);
}

.filters {
  display: flex;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  margin-bottom: 1rem;
}

.search-input {
  flex: 1;
  padding: 0.4rem 0.75rem;
  font-size: 0.875rem;
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-sm, 4px);
  background: var(--clr-bg);
  color: var(--clr-text-primary);
  outline: none;
}

.search-input:focus { border-color: var(--clr-accent); }

.cat-filter {
  padding: 0.4rem 0.75rem;
  font-size: 0.875rem;
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-sm, 4px);
  background: var(--clr-bg);
  color: var(--clr-text-primary);
  outline: none;
  cursor: pointer;
}

.table-card { overflow: hidden; }

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.8125rem;
}

th {
  text-align: left;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--clr-stroke);
  color: var(--clr-text-muted);
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

td {
  padding: 0.6rem 0.75rem;
  border-bottom: 1px solid var(--clr-stroke-soft);
  color: var(--clr-text-primary);
  vertical-align: middle;
}

.right { text-align: right; font-variant-numeric: tabular-nums; }

.desc {
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.date-cell { color: var(--clr-text-secondary); white-space: nowrap; }

.cat-badge {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  background: var(--clr-accent-light);
  color: var(--clr-accent);
  border-radius: 100px;
  font-size: 0.6875rem;
  font-weight: 500;
}

tbody tr:last-child td { border-bottom: none; }
tbody tr:hover td { background: var(--clr-stroke-soft); }
tbody tr.reversal td { color: var(--clr-text-muted); }
tbody tr.reversal .cat-badge { background: var(--clr-stroke-soft); color: var(--clr-text-muted); }

.no-results {
  text-align: center;
  padding: 2rem;
  color: var(--clr-text-muted);
  font-size: 0.875rem;
}

.empty-state {
  text-align: center;
  padding: 4rem 2rem;
  color: var(--clr-text-muted);
}

.empty-icon { font-size: 2.5rem; margin-bottom: 0.75rem; }

.loading {
  text-align: center;
  padding: 3rem;
  color: var(--clr-text-muted);
}

.error-msg {
  padding: 1rem;
  color: var(--clr-danger);
  background: var(--clr-danger-light, #fde7e9);
  border-radius: var(--radius-lg);
  font-size: 0.875rem;
}
</style>
