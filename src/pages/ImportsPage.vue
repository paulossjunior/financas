<script setup lang="ts">
// Imports page — tab shell for the Extratos & Faturas, Faturas, Extrato, and Contracheque views.
import { ref } from "vue";
import MovimentacoesPage from "@/pages/MovimentacoesPage.vue";
import HistoryPage from "@/pages/HistoryPage.vue";
import ExtratoPage from "@/pages/ExtratoPage.vue";
import ContrachequePage from "@/pages/ContrachequePage.vue";

const tab = ref<"movimentacoes" | "faturas" | "extrato" | "contracheque">("movimentacoes");
</script>

<template>
  <div class="imp">
    <div class="tabs" role="tablist">
      <button role="tab" :class="{ active: tab === 'movimentacoes' }" @click="tab = 'movimentacoes'">
        <span class="ic">🔁</span> Extratos &amp; Faturas
      </button>
      <button role="tab" :class="{ active: tab === 'faturas' }" @click="tab = 'faturas'">
        <span class="ic">📁</span> Faturas
      </button>
      <button role="tab" :class="{ active: tab === 'extrato' }" @click="tab = 'extrato'">
        <span class="ic">🏦</span> Extrato
      </button>
      <button role="tab" :class="{ active: tab === 'contracheque' }" @click="tab = 'contracheque'">
        <span class="ic">📄</span> Contracheque
      </button>
    </div>
    <MovimentacoesPage v-if="tab === 'movimentacoes'" />
    <HistoryPage v-else-if="tab === 'faturas'" />
    <ExtratoPage v-else-if="tab === 'extrato'" />
    <ContrachequePage v-else />
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
