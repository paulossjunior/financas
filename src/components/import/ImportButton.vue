<script setup lang="ts">
// Import button — opens a file picker for .xlsx invoices and emits the chosen paths.
import { ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";

const emit = defineEmits<{
  (e: "import-requested", paths: string[]): void;
}>();

const loading = ref(false);

async function handleClick(): Promise<void> {
  loading.value = true;
  try {
    const selected = await open({
      multiple: true,
      filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
    });
    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected];
      if (paths.length > 0) emit("import-requested", paths);
    }
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <button class="import-btn" :disabled="loading" @click="handleClick">
    <span v-if="loading">Importando...</span>
    <span v-else>📂 Importar Faturas</span>
  </button>
</template>

<style scoped>
.import-btn {
  padding: 0.45rem 1rem;
  background: var(--clr-accent);
  color: white;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.8125rem;
  font-weight: 600;
  font-family: var(--font-body);
  letter-spacing: 0.01em;
  transition: background 0.1s, box-shadow 0.1s;
  display: flex;
  align-items: center;
  gap: 0.35rem;
  white-space: nowrap;
}
.import-btn:hover:not(:disabled) {
  background: var(--clr-accent-hover);
  box-shadow: 0 0 0 2px rgba(0,120,212,0.2);
}
.import-btn:active:not(:disabled) { background: #005a9e; }
.import-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
