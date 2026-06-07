<script setup lang="ts">
import { onMounted, computed } from "vue";
import { useInvoiceStore } from "@/stores/invoice.store";
import MonthlyTrend from "@/components/dashboard/MonthlyTrend.vue";

const store = useInvoiceStore();

onMounted(async () => {
  await store.loadDashboard();
});

const hasHistory = computed(
  () => store.dashboard && store.dashboard.monthly_trend.length >= 2
);

function formatMonth(ym: string): string {
  const [year, month] = ym.split("-");
  const months = ["Jan","Fev","Mar","Abr","Mai","Jun","Jul","Ago","Set","Out","Nov","Dez"];
  return `${months[parseInt(month, 10) - 1]}/${year}`;
}
</script>

<template>
  <div class="page">
    <div class="page-header">
      <h1>Histórico Mensal</h1>
    </div>

    <div v-if="store.loading" class="loading">
      <div class="shimmer" style="height: 380px; border-radius: var(--radius-lg);" />
    </div>

    <template v-else-if="hasHistory">
      <div class="card">
        <MonthlyTrend :snapshots="store.dashboard!.monthly_trend" />
      </div>

      <div class="card mt">
        <h2>Resumo por Mês</h2>
        <table>
          <thead>
            <tr>
              <th>Mês</th>
              <th class="right">Total Líquido</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="s in store.dashboard!.monthly_trend" :key="s.month">
              <td>{{ formatMonth(s.month) }}</td>
              <td class="right amount">
                R$ {{ parseFloat(s.net_total).toLocaleString("pt-BR", { minimumFractionDigits: 2 }) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>

    <div v-else class="empty-state">
      <div class="empty-icon">📈</div>
      <h2>Dados insuficientes</h2>
      <p>Importe faturas de 2 ou mais meses para ver a evolução histórica.</p>
    </div>
  </div>
</template>

<style scoped>
.page { padding: 1.5rem 2rem; max-width: 1200px; margin: 0 auto; }

.page-header { margin-bottom: 1.25rem; }
h1 { font-size: 1.25rem; font-weight: 600; color: var(--clr-text-primary); letter-spacing: -0.01em; }

.card {
  background: var(--clr-surface);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-lg);
  padding: 1.25rem 1.5rem;
  box-shadow: var(--shadow-sm);
}
.mt { margin-top: 0.75rem; }
h2 { font-size: 0.875rem; font-weight: 600; color: var(--clr-text-primary); margin-bottom: 0.75rem; }

table { width: 100%; max-width: 360px; border-collapse: collapse; font-size: 0.8125rem; }
th {
  text-align: left;
  padding: 0.4rem 0.75rem;
  border-bottom: 1px solid var(--clr-stroke);
  color: var(--clr-text-muted);
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
td { padding: 0.55rem 0.75rem; border-bottom: 1px solid var(--clr-stroke-soft); color: var(--clr-text-primary); }
.right { text-align: right; }
.amount { font-variant-numeric: tabular-nums; font-weight: 500; }
tbody tr:last-child td { border-bottom: none; }
tbody tr:hover td { background: var(--clr-stroke-soft); }

.shimmer {
  background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
  background-size: 200% 100%;
  animation: shimmer 1.4s infinite;
}
@keyframes shimmer { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }

.empty-state { text-align: center; padding: 5rem 2rem; color: var(--clr-text-secondary); }
.empty-icon { font-size: 2.5rem; margin-bottom: 1rem; }
.empty-state h2 { font-size: 1.125rem; font-weight: 600; color: var(--clr-text-primary); margin-bottom: 0.5rem; }
.empty-state p { font-size: 0.875rem; }
</style>
