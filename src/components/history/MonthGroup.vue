<script setup lang="ts">
import type { MonthGroup } from "@/types/api.types";
import InvoiceRow from "./InvoiceRow.vue";

const props = defineProps<{ group: MonthGroup; isActive: boolean }>();
const emit = defineEmits<{
  "filter-month": [month: string];
  "remove-invoice": [invoiceId: string];
}>();

function formatTotal(val: string | null): string {
  if (val === null) return "—";
  const n = parseFloat(val) || 0;
  return n.toLocaleString("pt-BR", { style: "currency", currency: "BRL", minimumFractionDigits: 2 });
}
</script>

<template>
  <div class="month-group" :class="{ active: props.isActive }">
    <div class="group-header">
      <div class="group-meta">
        <span class="month-label">{{ props.group.label }}</span>
        <span class="count-badge">{{ props.group.invoice_count }} {{ props.group.invoice_count === 1 ? 'fatura' : 'faturas' }}</span>
      </div>
      <div class="group-actions">
        <span class="group-total">{{ formatTotal(props.group.net_total) }}</span>
        <button
          class="filter-btn"
          data-testid="filter-btn"
          @click="emit('filter-month', props.group.month)"
        >
          Ver dashboard →
        </button>
      </div>
    </div>

    <div class="invoice-list">
      <InvoiceRow
        v-for="invoice in props.group.invoices"
        :key="invoice.id"
        :invoice="invoice"
        @remove="emit('remove-invoice', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.month-group {
  background: var(--clr-surface);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-lg);
  overflow: hidden;
  box-shadow: var(--shadow-sm);
  transition: box-shadow 0.15s;
}
.month-group.active {
  border-color: var(--clr-accent);
  box-shadow: 0 0 0 2px rgba(0,120,212,0.15), var(--shadow-sm);
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.875rem 1rem;
  border-bottom: 1px solid var(--clr-stroke-soft);
  gap: 1rem;
}

.group-meta { display: flex; align-items: center; gap: 0.75rem; }
.month-label {
  font-size: 0.9375rem;
  font-weight: 600;
  color: var(--clr-text-primary);
  letter-spacing: -0.01em;
}
.count-badge {
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--clr-text-secondary);
  background: var(--clr-stroke-soft);
  border: 1px solid var(--clr-stroke);
  padding: 0.15rem 0.5rem;
  border-radius: 100px;
}

.group-actions { display: flex; align-items: center; gap: 0.75rem; }
.group-total {
  font-size: 1rem;
  font-weight: 700;
  color: var(--clr-text-primary);
  font-variant-numeric: tabular-nums;
}

.filter-btn {
  padding: 0.35rem 0.75rem;
  background: var(--clr-accent-light);
  color: var(--clr-accent);
  border: 1px solid rgba(0,120,212,0.2);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.75rem;
  font-weight: 600;
  font-family: var(--font-body);
  transition: background 0.1s;
  white-space: nowrap;
}
.filter-btn:hover { background: #daeeff; }

.invoice-list { padding: 0.25rem 0.25rem; }
</style>
