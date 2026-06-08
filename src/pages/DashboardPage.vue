<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { useInvoiceStore } from "@/stores/invoice.store";
import { useSettingsStore } from "@/stores/settings.store";
import ImportButton from "@/components/import/ImportButton.vue";
import ImportWarnings from "@/components/import/ImportWarnings.vue";
import BiggestSpendBanner from "@/components/dashboard/BiggestSpendBanner.vue";
import CategoryChart from "@/components/dashboard/CategoryChart.vue";
import CategoryRanking from "@/components/dashboard/CategoryRanking.vue";
import TopTransactions from "@/components/dashboard/TopTransactions.vue";
import type { ParseWarning } from "@/types/api.types";

const store = useInvoiceStore();
const settingsStore = useSettingsStore();
const lastWarnings = ref<ParseWarning[]>([]);

const availableCategories = computed(() =>
  settingsStore.categoryGroups.map((g) => g.name)
);

const transactionOverrides = computed(() =>
  settingsStore.config?.transaction_overrides ?? {}
);

onMounted(async () => {
  await store.refreshInvoices();
  await settingsStore.loadConfig();
  if (store.invoices.length > 0) {
    await store.loadDashboard();
  }
});

async function handleOverrideRefresh() {
  await settingsStore.loadConfig();
  await store.loadDashboard();
}

async function handleImport(paths: string[]): Promise<void> {
  try {
    const results = await store.importInvoices(paths);
    lastWarnings.value = results.flatMap((r) => r.warnings);
    await store.loadDashboard();
  } catch {
    // error already set in store
  }
}

function formatAmount(val: string): string {
  const n = parseFloat(val) || 0;
  return n.toLocaleString("pt-BR", { style: "currency", currency: "BRL", minimumFractionDigits: 2 });
}

const MONTHS = ["Jan","Fev","Mar","Abr","Mai","Jun","Jul","Ago","Set","Out","Nov","Dez"];
const MONTHS_FULL = ["Janeiro","Fevereiro","Março","Abril","Maio","Junho","Julho","Agosto","Setembro","Outubro","Novembro","Dezembro"];

function formatPeriod(from: string, to: string): string {
  if (!from || !to) return "—";
  const [yf, mf] = from.split("-");
  const [yt, mt] = to.split("-");
  const mfLabel = MONTHS[parseInt(mf) - 1];
  const mtLabel = MONTHS[parseInt(mt) - 1];
  if (from === to) return `${mfLabel}/${yf}`;
  return `${mfLabel}/${yf} – ${mtLabel}/${yt}`;
}

function formatMonthFilter(month: string): string {
  const [year, m] = month.split("-");
  return `${MONTHS_FULL[parseInt(m) - 1] ?? m}/${year}`;
}
</script>

<template>
  <div class="page">
    <!-- Page header -->
    <div class="page-header">
      <div class="page-title">
        <h1>Dashboard</h1>
        <span v-if="store.dashboard" class="period-badge">
          {{ formatPeriod(store.dashboard.period.from, store.dashboard.period.to) }}
        </span>
      </div>
      <div class="page-actions">
        <ImportButton @import-requested="handleImport" />
      </div>
    </div>

    <ImportWarnings :warnings="lastWarnings" />

    <!-- Month filter badge -->
    <div v-if="store.monthFilter" class="filter-badge">
      <span>Filtrado: <strong>{{ formatMonthFilter(store.monthFilter) }}</strong></span>
      <button class="clear-filter" @click="store.setMonthFilter(null)">✕ Limpar</button>
    </div>

    <div v-if="store.error" class="msg-bar msg-bar--error">
      <span class="msg-icon">⚠</span>
      {{ store.error }}
    </div>

    <!-- Loading shimmer -->
    <div v-if="store.loading" class="loading-row">
      <div class="shimmer kpi-shimmer" />
      <div class="shimmer kpi-shimmer" />
      <div class="shimmer kpi-shimmer" />
    </div>

    <template v-if="store.dashboard && !store.loading">
      <!-- KPI row -->
      <div class="kpi-row">
        <div class="kpi-card">
          <span class="kpi-label">Total Líquido</span>
          <span class="kpi-value kpi-value--large">{{ formatAmount(store.dashboard.net_total) }}</span>
        </div>
        <div class="kpi-card">
          <span class="kpi-label">Total Cobrado</span>
          <span class="kpi-value">{{ formatAmount(store.dashboard.total_charged) }}</span>
        </div>
        <div class="kpi-card">
          <span class="kpi-label">Estornos</span>
          <span class="kpi-value kpi-value--positive">{{ formatAmount(store.dashboard.total_reversals) }}</span>
        </div>
        <div class="kpi-card">
          <span class="kpi-label">Faturas</span>
          <span class="kpi-value">{{ store.dashboard.invoice_count }}</span>
          <span class="kpi-sub">importadas</span>
        </div>
      </div>

      <!-- Biggest spend -->
      <BiggestSpendBanner
        v-if="store.dashboard.categories.length > 0"
        :category="store.dashboard.categories[0]"
      />

      <!-- Charts -->
      <div class="charts-grid">
        <div class="card">
          <CategoryChart :categories="store.dashboard.categories" />
        </div>
        <div class="card">
          <CategoryRanking :categories="store.dashboard.categories" />
        </div>
      </div>

      <!-- Transactions -->
      <div class="card mt">
        <TopTransactions
          :transactions="store.dashboard.top_transactions"
          :availableCategories="availableCategories"
          :transactionOverrides="transactionOverrides"
          @refresh="handleOverrideRefresh"
        />
      </div>
    </template>

    <!-- Empty state -->
    <div v-else-if="!store.loading && store.invoices.length === 0" class="empty-state">
      <div class="empty-icon">📂</div>
      <h2>Nenhuma fatura importada</h2>
      <p>Importe uma fatura BTG para visualizar seu dashboard de gastos.</p>
      <p class="empty-hint">O arquivo deve estar sem proteção por senha.</p>
    </div>
  </div>
</template>

<style scoped>
.page {
  padding: 1.5rem 2rem;
  max-width: 1200px;
  margin: 0 auto;
}

/* Header */
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1.25rem;
}
.page-title { display: flex; align-items: baseline; gap: 0.75rem; }
h1 { font-size: 1.25rem; font-weight: 600; color: var(--clr-text-primary); letter-spacing: -0.01em; }
.period-badge {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--clr-text-secondary);
  background: var(--clr-stroke-soft);
  padding: 0.2rem 0.6rem;
  border-radius: 100px;
  border: 1px solid var(--clr-stroke);
}

/* Month filter badge */
.filter-badge {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 1rem;
  background: var(--clr-accent-light);
  border: 1px solid rgba(0,120,212,0.25);
  border-radius: var(--radius-md);
  font-size: 0.8125rem;
  color: var(--clr-accent);
  margin-bottom: 0.75rem;
}
.filter-badge strong { font-weight: 700; }
.clear-filter {
  background: none;
  border: 1px solid rgba(0,120,212,0.3);
  border-radius: var(--radius-sm);
  color: var(--clr-accent);
  cursor: pointer;
  font-size: 0.75rem;
  font-weight: 600;
  font-family: var(--font-body);
  padding: 0.2rem 0.6rem;
  transition: background 0.1s;
}
.clear-filter:hover { background: rgba(0,120,212,0.1); }

/* Message bar */
.msg-bar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.6rem 1rem;
  border-radius: var(--radius-md);
  font-size: 0.8125rem;
  margin-bottom: 1rem;
}
.msg-bar--error {
  background: #fde7e9;
  border: 1px solid #f1707b;
  color: var(--clr-negative);
}
.msg-icon { font-style: normal; }

/* KPI row */
.kpi-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 0.75rem;
  margin-bottom: 1rem;
}
.kpi-card {
  background: var(--clr-surface);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-lg);
  padding: 1rem 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  box-shadow: var(--shadow-sm);
  transition: box-shadow 0.15s;
}
.kpi-card:hover { box-shadow: var(--shadow-md); }
.kpi-label {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--clr-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.kpi-value {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--clr-text-primary);
  line-height: 1.2;
  font-variant-numeric: tabular-nums;
}
.kpi-value--large { font-size: 1.5rem; font-weight: 700; }
.kpi-value--positive { color: var(--clr-positive); }
.kpi-sub {
  font-size: 0.6875rem;
  color: var(--clr-text-muted);
}

/* Card wrapper */
.card {
  background: var(--clr-surface);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-lg);
  padding: 1.25rem 1.5rem;
  box-shadow: var(--shadow-sm);
}
.mt { margin-top: 0.75rem; }

/* Charts */
.charts-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
  margin-top: 0.75rem;
}

/* Shimmer loading */
.loading-row { display: grid; grid-template-columns: repeat(4, 1fr); gap: 0.75rem; margin-bottom: 1rem; }
.shimmer {
  background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
  background-size: 200% 100%;
  animation: shimmer 1.4s infinite;
  border-radius: var(--radius-lg);
}
.kpi-shimmer { height: 80px; }
@keyframes shimmer { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }

/* Empty state */
.empty-state {
  text-align: center;
  padding: 5rem 2rem;
  color: var(--clr-text-secondary);
}
.empty-icon { font-size: 3rem; margin-bottom: 1rem; }
.empty-state h2 { font-size: 1.125rem; font-weight: 600; color: var(--clr-text-primary); margin-bottom: 0.5rem; }
.empty-state p { font-size: 0.875rem; }
.empty-hint { font-size: 0.75rem; color: var(--clr-text-muted); margin-top: 0.25rem; }
</style>
