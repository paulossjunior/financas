<script setup lang="ts">
import { ref } from "vue";
import HistoryPage from "@/pages/HistoryPage.vue";
import ExtratoPage from "@/pages/ExtratoPage.vue";

const tab = ref<"faturas" | "extrato">("faturas");
</script>

<template>
  <div class="imp">
    <div class="tabs" role="tablist">
      <button role="tab" :class="{ active: tab === 'faturas' }" @click="tab = 'faturas'">
        <span class="ic">📁</span> Faturas
      </button>
      <button role="tab" :class="{ active: tab === 'extrato' }" @click="tab = 'extrato'">
        <span class="ic">🏦</span> Extrato
      </button>
    </div>
    <HistoryPage v-if="tab === 'faturas'" />
    <ExtratoPage v-else />
  </div>
</template>

<style scoped>
.imp { max-width: 1000px; margin: 0 auto; }
.tabs { display: flex; gap: 6px; margin: 0 0 14px; border-bottom: 1px solid var(--clr-stroke); }
.tabs button {
  font-family: inherit; font-size: 14px; font-weight: 700; padding: 10px 16px; cursor: pointer;
  background: transparent; border: none; border-bottom: 2px solid transparent; color: var(--clr-text-secondary);
  display: inline-flex; align-items: center; gap: 8px; margin-bottom: -1px;
}
.tabs button:hover { color: var(--clr-text-primary); }
.tabs button.active { color: var(--clr-accent); border-bottom-color: var(--clr-accent); }
.tabs .ic { font-size: 15px; }
</style>
