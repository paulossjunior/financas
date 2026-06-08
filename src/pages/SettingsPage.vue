<script setup lang="ts">
import { onMounted } from "vue";
import { useSettingsStore } from "@/stores/settings.store";
import CategoryList from "@/components/settings/CategoryList.vue";

const store = useSettingsStore();

onMounted(async () => {
  await store.loadConfig();
});

async function handleSave(): Promise<void> {
  await store.saveCategories();
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
            v-model="store.config.faturas_directory"
            type="text"
            placeholder="faturas/"
            class="text-input"
          />
        </div>
      </section>

      <div class="divider" />

      <section class="section">
        <h2>Categorias &amp; Regras</h2>
        <p class="field-hint">Gerencie categorias e palavras-chave para classificação automática de transações.</p>

        <CategoryList
          :groups="store.categoryGroups"
          @add-category="store.addCategory('Nova Categoria')"
          @delete-category="store.deleteCategory"
          @rename-category="store.renameCategory"
          @update-keywords="(name, kws) => { const g = store.categoryGroups.find(g => g.name === name); if (g) { g.keywords.splice(0, g.keywords.length, ...kws); } }"
        />
      </section>

      <div class="divider" />

      <div class="actions">
        <div v-if="store.error" class="msg-bar msg-bar--error">⚠ {{ store.error }}</div>
        <button class="save-btn" :disabled="store.saving" @click="handleSave">
          {{ store.saving ? "Salvando…" : "Salvar" }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page { padding: 1.5rem 2rem; max-width: 720px; margin: 0 auto; }

.page-header { margin-bottom: 1.25rem; }
h1 { font-size: 1.25rem; font-weight: 600; color: var(--clr-text-primary); letter-spacing: -0.01em; }

.card {
  background: var(--clr-surface);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-lg);
  padding: 1.5rem;
  box-shadow: var(--shadow-sm);
}

.section { margin-bottom: 0.5rem; }
.section h2 {
  font-size: 0.8125rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--clr-text-muted);
  margin-bottom: 0.75rem;
}

.field { display: flex; flex-direction: column; gap: 0.25rem; margin-bottom: 0.5rem; }
label { font-size: 0.875rem; font-weight: 600; color: var(--clr-text-primary); }
.field-hint { font-size: 0.75rem; color: var(--clr-text-muted); margin-bottom: 0.5rem; }
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
.save-btn:hover:not(:disabled) { background: var(--clr-accent-hover); }
.save-btn:active:not(:disabled) { background: #005a9e; }
.save-btn:disabled { opacity: 0.6; cursor: default; }
</style>
