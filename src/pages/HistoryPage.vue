<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { ask } from "@tauri-apps/plugin-dialog";
import { useInvoiceStore } from "@/stores/invoice.store";
import ImportButton from "@/components/import/ImportButton.vue";
import ImportWarnings from "@/components/import/ImportWarnings.vue";
import MonthGroupComponent from "@/components/history/MonthGroup.vue";
import type { ParseWarning } from "@/types/api.types";
import { ref } from "vue";

const store = useInvoiceStore();
const router = useRouter();
const lastWarnings = ref<ParseWarning[]>([]);

onMounted(async () => {
  await store.refreshInvoices();
  await store.loadDashboard();
});

async function handleImport(paths: string[]): Promise<void> {
  try {
    const results = await store.importInvoices(paths);
    lastWarnings.value = results.flatMap((r) => r.warnings);
    await store.loadDashboard();
  } catch {
    // error set in store
  }
}

async function handleFilterMonth(month: string): Promise<void> {
  await store.setMonthFilter(month);
  router.push("/");
}

async function handleRemoveInvoice(invoiceId: string): Promise<void> {
  const confirmed = await ask(
    "Remover esta fatura permanentemente? Esta ação não pode ser desfeita.",
    { title: "Remover Fatura", kind: "warning" }
  );
  if (!confirmed) return;
  try {
    await store.removeInvoice(invoiceId);
  } catch {
    // error set in store
  }
}
</script>

<template>
  <div class="page">
    <div class="page-header">
      <h1>Histórico</h1>
      <div class="page-actions">
        <ImportButton @import-requested="handleImport" />
      </div>
    </div>

    <ImportWarnings :warnings="lastWarnings" />

    <div v-if="store.error" class="msg-bar msg-bar--error">
      <span>⚠ {{ store.error }}</span>
    </div>

    <!-- Loading -->
    <div v-if="store.loading" class="loading-list">
      <div v-for="i in 3" :key="i" class="shimmer group-shimmer" />
    </div>

    <!-- Month groups -->
    <template v-else-if="store.monthGroups.length > 0">
      <div class="groups-list">
        <MonthGroupComponent
          v-for="group in store.monthGroups"
          :key="group.month"
          :group="group"
          :is-active="store.monthFilter === group.month"
          @filter-month="handleFilterMonth"
          @remove-invoice="handleRemoveInvoice"
        />
      </div>
    </template>

    <!-- Empty state -->
    <div v-else class="empty-state">
      <div class="empty-icon">📂</div>
      <h2>Nenhuma fatura importada</h2>
      <p>Importe uma fatura BTG para ver o histórico de gastos.</p>
      <p class="empty-hint">O arquivo deve estar sem proteção por senha.</p>
    </div>
  </div>
</template>

<style scoped>
.page { padding: 1.5rem 2rem; max-width: 1100px; margin: 0 auto; }

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1.25rem;
}
h1 { font-size: 1.25rem; font-weight: 600; color: var(--clr-text-primary); letter-spacing: -0.01em; }

.msg-bar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.6rem 1rem;
  border-radius: var(--radius-md);
  font-size: 0.8125rem;
  margin-bottom: 1rem;
}
.msg-bar--error { background: #fde7e9; border: 1px solid #f1707b; color: var(--clr-negative); }

.groups-list { display: flex; flex-direction: column; gap: 0.75rem; }

/* Shimmer */
.loading-list { display: flex; flex-direction: column; gap: 0.75rem; }
.shimmer {
  background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
  background-size: 200% 100%;
  animation: shimmer 1.4s infinite;
  border-radius: var(--radius-lg);
}
.group-shimmer { height: 100px; }
@keyframes shimmer { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }

/* Empty state */
.empty-state { text-align: center; padding: 5rem 2rem; color: var(--clr-text-secondary); }
.empty-icon { font-size: 3rem; margin-bottom: 1rem; }
.empty-state h2 { font-size: 1.125rem; font-weight: 600; color: var(--clr-text-primary); margin-bottom: 0.5rem; }
.empty-state p { font-size: 0.875rem; }
.empty-hint { font-size: 0.75rem; color: var(--clr-text-muted); margin-top: 0.25rem; }
</style>
