<script setup lang="ts">
// History row rendering one imported invoice file, with a remove action.
import type { InvoiceInfo } from "@/types/api.types";

const props = defineProps<{ invoice: InvoiceInfo }>();
const emit = defineEmits<{ remove: [invoiceId: string] }>();

function formatDate(iso: string): string {
  const [date] = iso.split("T");
  const [y, m, d] = date.split("-");
  return `${d}/${m}/${y}`;
}
</script>

<template>
  <div class="invoice-row">
    <div class="invoice-info">
      <span class="name-line">
        <span class="bank-chip" data-testid="invoice-bank">{{ props.invoice.bank }}</span>
        <span class="filename" :title="props.invoice.filename">{{ props.invoice.filename }}</span>
      </span>
      <span class="meta">{{ props.invoice.row_count }} transações · importado {{ formatDate(props.invoice.imported_at) }}</span>
    </div>
    <button
      class="remove-btn"
      data-testid="remove-btn"
      title="Remover fatura"
      @click="emit('remove', props.invoice.id)"
    >
      🗑
    </button>
  </div>
</template>

<style scoped>
.invoice-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.6rem 0.75rem;
  border-radius: var(--radius-md);
  transition: background 0.1s;
}
.invoice-row:hover { background: var(--clr-stroke-soft); }

.invoice-info {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  min-width: 0;
}
.name-line {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
}
/* same look as the bank tags on Extrato / Despesas & Receitas (consistency, H4) */
.bank-chip {
  flex-shrink: 0;
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--clr-text-secondary);
  background: var(--clr-surface-alt, #eef1f0);
  border: 1px solid var(--clr-stroke);
  border-radius: 100px;
  padding: 0.08rem 0.5rem;
  letter-spacing: 0.02em;
}
.filename {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--clr-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.meta {
  font-size: 0.6875rem;
  color: var(--clr-text-muted);
}

.remove-btn {
  background: none;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 0.875rem;
  padding: 0.25rem 0.4rem;
  color: var(--clr-text-muted);
  flex-shrink: 0;
  transition: background 0.1s, color 0.1s, border-color 0.1s;
  line-height: 1;
}
.remove-btn:hover {
  background: #fde7e9;
  color: var(--clr-negative);
  border-color: #f1707b;
}
</style>
