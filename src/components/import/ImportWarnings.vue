<script setup lang="ts">
import type { ParseWarning } from "@/types/api.types";

defineProps<{ warnings: ParseWarning[] }>();
</script>

<template>
  <div v-if="warnings.length > 0" class="warnings">
    <p class="warnings-title">⚠️ {{ warnings.length }} aviso(s) durante a importação:</p>
    <ul>
      <li v-for="w in warnings" :key="w.row">
        Linha {{ w.row }}: {{ w.message }}
      </li>
    </ul>
  </div>
</template>

<style scoped>
.warnings {
  background: #fff4ce;
  border: 1px solid #f7c948;
  border-left: 3px solid #f7c948;
  border-radius: var(--radius-md);
  padding: 0.6rem 1rem;
  margin-bottom: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}
.warnings-title {
  font-size: 0.8125rem;
  font-weight: 600;
  color: #4d3b00;
  display: flex;
  align-items: center;
  gap: 0.3rem;
}
ul { margin: 0; padding-left: 1.25rem; font-size: 0.75rem; color: #5d4900; }
li { margin-top: 0.15rem; }
</style>
