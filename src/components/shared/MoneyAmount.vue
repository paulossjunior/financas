<script setup lang="ts">
// Shared component formatting a money string as BRL with positive/negative styling.
const { amount } = defineProps<{ amount: string; showSign?: boolean }>();

function format(val: string): string {
  const num = parseFloat(val);
  if (isNaN(num)) return val;
  return num.toLocaleString("pt-BR", {
    style: "currency",
    currency: "BRL",
    minimumFractionDigits: 2,
  });
}
</script>

<template>
  <span :class="{ negative: parseFloat(amount) < 0, positive: parseFloat(amount) > 0 }">
    {{ format(amount) }}
  </span>
</template>

<style scoped>
.negative { color: var(--clr-negative); }
.positive { color: var(--clr-positive); }
</style>
