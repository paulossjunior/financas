<script setup lang="ts">
// Faturas history page — import card invoices (BTG .xlsx, Santander .pdf) and
// browse/remove them grouped by month. Encrypted files prompt for the bank's
// password here (saved per bank in the OS keychain when "remember" is checked).
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { ask } from "@tauri-apps/plugin-dialog";
import { useInvoiceStore } from "@/stores/invoice.store";
import ImportButton from "@/components/import/ImportButton.vue";
import ImportWarnings from "@/components/import/ImportWarnings.vue";
import PasswordModal from "@/components/import/PasswordModal.vue";
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

// Password prompt state. Files import one by one: each encrypted file whose bank
// has no saved password opens the modal labeled with THAT bank (banks have
// different passwords); once remembered, the rest of the batch flows silently.
const pwOpen = ref(false);
const pwBank = ref<string | null>(null);
const pwError = ref<string | null>(null);
const importQueue = ref<string[]>([]);

function bankFromCode(msg: string): string | null {
  const [, bank] = msg.split(":");
  return bank || null;
}

async function handleImport(paths: string[]): Promise<void> {
  importQueue.value = [...paths];
  lastWarnings.value = [];
  await processQueue();
}

/// Import the queue head-first. `password` applies only to the file that asked.
async function processQueue(password?: string, remember?: boolean): Promise<void> {
  while (importQueue.value.length > 0) {
    const path = importQueue.value[0];
    try {
      const results = await store.importInvoices([path], password, remember);
      lastWarnings.value.push(...results.flatMap((r) => r.warnings));
      importQueue.value.shift();
      password = undefined;
      remember = undefined;
      pwOpen.value = false;
      pwError.value = null;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.startsWith("ENCRYPTED_FILE")) {
        pwBank.value = bankFromCode(msg);
        pwError.value = password ? "Informe a senha do arquivo." : null;
        pwOpen.value = true;
        return; // wait for the modal; the queue resumes on submit
      }
      if (msg.startsWith("WRONG_PASSWORD")) {
        pwBank.value = bankFromCode(msg);
        pwError.value = "Senha incorreta. Tente novamente.";
        pwOpen.value = true;
        return;
      }
      // Other errors land in the store's error bar; stop the batch there.
      break;
    }
  }
  await store.loadDashboard();
}

function submitPassword(password: string, remember: boolean): void {
  void processQueue(password, remember);
}

async function cancelPassword(): Promise<void> {
  // Canceling skips the file that asked and the rest of the batch — predictable,
  // and anything already imported stays.
  importQueue.value = [];
  pwOpen.value = false;
  pwError.value = null;
  await store.loadDashboard();
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
      <div>
        <h1>Faturas</h1>
        <span class="faturas-sub">Gerencie as faturas importadas — clique num mês para filtrar, ou remova uma fatura.</span>
      </div>
      <div class="page-actions">
        <ImportButton @import-requested="handleImport" />
      </div>
    </div>

    <ImportWarnings :warnings="lastWarnings" />

    <PasswordModal
      :open="pwOpen"
      :loading="store.loading"
      :error="pwError"
      :bank="pwBank"
      @submit="submitPassword"
      @cancel="cancelPassword"
    />

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
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 1.25rem;
}
h1 { font-size: 1.25rem; font-weight: 600; color: var(--clr-text-primary); letter-spacing: -0.01em; }
.faturas-sub { display: block; margin-top: 4px; font-size: 0.8125rem; color: var(--clr-text-secondary); max-width: 60ch; }

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
