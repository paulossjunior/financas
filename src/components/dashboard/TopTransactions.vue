<script setup lang="ts">
// Dashboard list of top transactions, each with a per-transaction category override.
import type { TransactionSummary } from "@/types/api.types";
import MoneyAmount from "@/components/shared/MoneyAmount.vue";
import TransactionCategoryOverride from "@/components/settings/TransactionCategoryOverride.vue";
import { overrideTransactionCategory, removeTransactionOverride } from "@/services/tauri.service";

const props = defineProps<{
  transactions: TransactionSummary[];
  availableCategories?: string[];
  transactionOverrides?: Record<string, string>;
}>();

const emit = defineEmits<{ refresh: [] }>();

function formatDate(iso: string): string {
  const [y, m, d] = iso.split("-");
  return `${d}/${m}/${y}`;
}

async function handleOverride(txId: string, category: string) {
  await overrideTransactionCategory(txId, category);
  emit("refresh");
}

async function handleRemoveOverride(txId: string) {
  await removeTransactionOverride(txId);
  emit("refresh");
}
</script>

<template>
  <div class="top-transactions">
    <h3>Top 5 Transações</h3>
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
        <tr v-for="tx in transactions" :key="tx.id">
          <td class="date-cell">{{ formatDate(tx.date) }}</td>
          <td class="desc">{{ tx.description }}</td>
          <td>
            <TransactionCategoryOverride
              v-if="props.availableCategories && props.availableCategories.length > 0"
              :transactionId="tx.id"
              :currentCategory="tx.category"
              :availableCategories="props.availableCategories"
              :hasOverride="!!(props.transactionOverrides && props.transactionOverrides[tx.id])"
              @override="handleOverride"
              @removeOverride="handleRemoveOverride"
            />
            <span v-else class="cat-badge">{{ tx.category }}</span>
          </td>
          <td class="right"><MoneyAmount :amount="tx.amount" /></td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.top-transactions {}
h3 {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--clr-text-primary);
  margin-bottom: 0.75rem;
  letter-spacing: -0.005em;
}
table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.8125rem;
}
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
td {
  padding: 0.6rem 0.75rem;
  border-bottom: 1px solid var(--clr-stroke-soft);
  color: var(--clr-text-primary);
  vertical-align: middle;
}
.right { text-align: right; font-variant-numeric: tabular-nums; }
.desc { max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.cat-badge {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  background: var(--clr-accent-light);
  color: var(--clr-accent);
  border-radius: 100px;
  font-size: 0.6875rem;
  font-weight: 500;
}
.date-cell { color: var(--clr-text-secondary); white-space: nowrap; }
tbody tr:last-child td { border-bottom: none; }
tbody tr:hover td { background: var(--clr-stroke-soft); }
</style>
