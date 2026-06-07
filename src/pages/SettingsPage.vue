<script setup lang="ts">
import { ref, onMounted } from "vue";
import { getConfig, saveConfig } from "@/services/tauri.service";
import type { AppConfig } from "@/types/api.types";

const config = ref<AppConfig>({ faturas_directory: "faturas", category_rules: [] });
const saved = ref(false);
const error = ref<string | null>(null);

onMounted(async () => {
  try {
    config.value = await getConfig();
  } catch {
    // use defaults
  }
});

async function handleSave(): Promise<void> {
  error.value = null;
  saved.value = false;
  try {
    await saveConfig(config.value);
    saved.value = true;
    setTimeout(() => (saved.value = false), 2000);
  } catch (e) {
    error.value = String(e instanceof Error ? e.message : e);
  }
}
</script>

<template>
  <div class="page">
    <div class="page-header">
      <h1>Configurações</h1>
    </div>

    <div class="card">
      <section class="section">
        <h2>Importação</h2>

        <div class="field">
          <label for="faturas-dir">Pasta das Faturas</label>
          <p class="field-hint">Caminho relativo ao diretório do app onde as faturas XLSX são procuradas.</p>
          <input
            id="faturas-dir"
            v-model="config.faturas_directory"
            type="text"
            placeholder="faturas/"
            class="text-input"
          />
        </div>
      </section>

      <div class="divider" />

      <div class="actions">
        <div v-if="error" class="msg-bar msg-bar--error">⚠ {{ error }}</div>
        <div v-if="saved" class="msg-bar msg-bar--success">✓ Configurações salvas</div>
        <button class="save-btn" @click="handleSave">Salvar</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page { padding: 1.5rem 2rem; max-width: 640px; margin: 0 auto; }

.page-header { margin-bottom: 1.25rem; }
h1 { font-size: 1.25rem; font-weight: 600; color: var(--clr-text-primary); letter-spacing: -0.01em; }

.card {
  background: var(--clr-surface);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-lg);
  padding: 1.5rem;
  box-shadow: var(--shadow-sm);
}

.section h2 {
  font-size: 0.8125rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--clr-text-muted);
  margin-bottom: 1rem;
}

.field { display: flex; flex-direction: column; gap: 0.25rem; }
label { font-size: 0.875rem; font-weight: 600; color: var(--clr-text-primary); }
.field-hint { font-size: 0.75rem; color: var(--clr-text-muted); margin-bottom: 0.25rem; }
.text-input {
  padding: 0.45rem 0.75rem;
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  font-family: var(--font-body);
  color: var(--clr-text-primary);
  background: var(--clr-bg);
  width: 100%;
  outline: none;
  transition: border-color 0.1s, box-shadow 0.1s;
}
.text-input:focus {
  border-color: var(--clr-accent);
  box-shadow: 0 0 0 2px rgba(0,120,212,0.15);
}

.divider { height: 1px; background: var(--clr-stroke); margin: 1.25rem 0; }

.actions { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; }
.msg-bar {
  flex: 1;
  padding: 0.45rem 0.75rem;
  border-radius: var(--radius-md);
  font-size: 0.8125rem;
  font-weight: 500;
}
.msg-bar--error { background: #fde7e9; border: 1px solid #f1707b; color: var(--clr-negative); }
.msg-bar--success { background: #dff6dd; border: 1px solid #107c10; color: var(--clr-positive); }

.save-btn {
  padding: 0.45rem 1.25rem;
  background: var(--clr-accent);
  color: white;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.8125rem;
  font-weight: 600;
  font-family: var(--font-body);
  transition: background 0.1s;
  white-space: nowrap;
}
.save-btn:hover { background: var(--clr-accent-hover); }
.save-btn:active { background: #005a9e; }
</style>
