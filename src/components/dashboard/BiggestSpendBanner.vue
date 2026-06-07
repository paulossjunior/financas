<script setup lang="ts">
import type { Category } from "@/types/api.types";

defineProps<{ category: Category }>();
</script>

<template>
  <div class="banner">
    <div class="label">🏆 Maior Gasto</div>
    <div class="category-name">{{ category.name }}</div>
    <div class="amount">
      R$ {{ parseFloat(category.net_total).toLocaleString("pt-BR", { minimumFractionDigits: 2 }) }}
    </div>
    <div class="percentage">{{ category.percentage.toFixed(1) }}% do total</div>
  </div>
</template>

<style scoped>
.banner {
  background: linear-gradient(135deg, var(--clr-accent) 0%, #106ebe 60%, #005a9e 100%);
  color: white;
  border-radius: var(--radius-lg);
  padding: 1.25rem 1.75rem;
  display: grid;
  grid-template-columns: 1fr auto;
  grid-template-rows: auto auto;
  gap: 0.125rem 1.5rem;
  margin-bottom: 0.75rem;
  box-shadow: var(--shadow-md);
}
.label {
  grid-column: 1; grid-row: 1;
  font-size: 0.6875rem; font-weight: 600;
  text-transform: uppercase; letter-spacing: 0.06em;
  opacity: 0.8;
}
.category-name {
  grid-column: 1; grid-row: 2;
  font-size: 1.375rem; font-weight: 700;
  line-height: 1.2; letter-spacing: -0.01em;
}
.amount {
  grid-column: 2; grid-row: 1 / 3;
  align-self: center;
  font-size: 1.5rem; font-weight: 700;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.percentage {
  display: none;
}
</style>
