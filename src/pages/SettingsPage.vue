<script setup lang="ts">
// Settings page (Configurações) — invoice folder path and saved-password (Keychain) options.
import { onMounted, ref } from "vue";
import { open, ask } from "@tauri-apps/plugin-dialog";
import { useSettingsStore } from "@/stores/settings.store";
import {
  hasSavedPassword,
  clearSavedPassword,
  backupDatabase,
  restoreDatabase,
  setImportDirectory,
} from "@/services/tauri.service";
import type { FolderImportSummary } from "@/types/api.types";

const store = useSettingsStore();

const pwSaved = ref(false);
const pwBusy = ref(false);

// Backup & restore (feature 012)
const backupBusy = ref(false);
const restoreBusy = ref(false);
const dbMsg = ref(""); // success feedback (backup path)
const dbErr = ref(""); // error feedback

// Auto-import folder (feature 013)
const importBusy = ref(false);
const importMsg = ref("");
const importErr = ref("");

function summaryText(s: FolderImportSummary): string {
  const parts = [`${s.faturas} fatura(s)`, `${s.extratos} extrato(s)`];
  if (s.ignored.length) parts.push(`${s.ignored.length} ignorado(s)`);
  return `Importado: ${parts.join(", ")}.`;
}

onMounted(async () => {
  await store.loadConfig();
  pwSaved.value = await hasSavedPassword();
});

async function handleSave(): Promise<void> {
  await store.saveCategories();
}

async function forgetPassword(): Promise<void> {
  pwBusy.value = true;
  try {
    await clearSavedPassword();
    pwSaved.value = false;
  } finally {
    pwBusy.value = false;
  }
}

async function chooseImportFolder(): Promise<void> {
  importMsg.value = "";
  importErr.value = "";
  const dir = await open({ directory: true, title: "Escolha a pasta de importação" });
  if (typeof dir !== "string") return; // cancelled
  importBusy.value = true;
  try {
    const summary = await setImportDirectory(dir);
    store.config.import_directory = dir;
    if (summary) importMsg.value = summaryText(summary);
  } catch (e) {
    importErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    importBusy.value = false;
  }
}

async function clearImportFolder(): Promise<void> {
  importMsg.value = "";
  importErr.value = "";
  importBusy.value = true;
  try {
    await setImportDirectory(null);
    store.config.import_directory = null;
  } catch (e) {
    importErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    importBusy.value = false;
  }
}

async function doBackup(): Promise<void> {
  dbMsg.value = "";
  dbErr.value = "";
  const dir = await open({ directory: true, title: "Escolha a pasta para o backup" });
  if (typeof dir !== "string") return; // cancelled
  backupBusy.value = true;
  try {
    const res = await backupDatabase(dir);
    dbMsg.value = `Backup salvo em: ${res.path}`;
  } catch (e) {
    dbErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    backupBusy.value = false;
  }
}

async function doRestore(): Promise<void> {
  dbMsg.value = "";
  dbErr.value = "";
  const file = await open({
    multiple: false,
    title: "Escolha o arquivo de backup",
    filters: [{ name: "Backup do Finanças", extensions: ["db"] }],
  });
  if (typeof file !== "string") return; // cancelled
  const ok = await ask(
    "Isto substitui todos os dados atuais pelos do backup. Uma cópia de segurança da base atual é guardada automaticamente antes, para você poder reverter. Deseja continuar?",
    { title: "Restaurar backup", kind: "warning", okLabel: "Restaurar", cancelLabel: "Cancelar" }
  );
  if (!ok) return;
  restoreBusy.value = true;
  try {
    await restoreDatabase(file);
    // Reload so every page refetches from the restored database.
    window.location.reload();
  } catch (e) {
    dbErr.value = e instanceof Error ? e.message : String(e);
    restoreBusy.value = false;
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
        <h2>Importação automática</h2>

        <div class="field">
          <label>Pasta de importação</label>
          <p class="field-hint">
            Escolha uma pasta com suas faturas (.xlsx) e extratos (.xls). Ao definir, o app
            importa o que já está lá; e toda vez que abrir, importa automaticamente os arquivos
            novos dessa pasta — identificando sozinho o que é fatura e o que é extrato, sem
            duplicar o que já foi importado.
          </p>
          <div class="db-row">
            <button class="db-btn" :disabled="importBusy" @click="chooseImportFolder">
              {{ importBusy ? "Importando…" : "Escolher pasta" }}
            </button>
            <button
              v-if="store.config.import_directory"
              class="db-btn"
              :disabled="importBusy"
              @click="clearImportFolder"
            >
              Limpar
            </button>
          </div>
          <p v-if="store.config.import_directory" class="field-path">
            Pasta atual: {{ store.config.import_directory }}
          </p>
          <p v-else class="field-path">Nenhuma pasta — importação manual apenas.</p>
          <p v-if="importMsg" class="db-feedback db-feedback--ok">✓ {{ importMsg }}</p>
          <p v-if="importErr" class="db-feedback db-feedback--err">⚠ {{ importErr }}</p>
        </div>
      </section>

      <div class="divider" />

      <section class="section">
        <h2>Segurança</h2>
        <div class="field">
          <label>Senha das faturas</label>
          <p class="field-hint">
            A senha de faturas protegidas fica guardada no Keychain do sistema (cifrada), nunca no banco de dados.
          </p>
          <div class="pw-row">
            <span class="pw-state" :class="pwSaved ? 'on' : 'off'">
              {{ pwSaved ? "✓ Senha salva neste dispositivo" : "Nenhuma senha salva" }}
            </span>
            <button
              v-if="pwSaved"
              class="forget-btn"
              :disabled="pwBusy"
              @click="forgetPassword"
            >
              {{ pwBusy ? "Removendo…" : "Esquecer senha" }}
            </button>
          </div>
        </div>
      </section>

      <div class="divider" />

      <section class="section">
        <h2>Backup e restauração</h2>
        <div class="field">
          <label>Cópia de segurança dos dados</label>
          <p class="field-hint">
            Gera uma cópia completa da base (faturas, contracheques, categorias e lançamentos)
            numa pasta à sua escolha. Ao restaurar, a base atual é substituída pela do backup —
            uma cópia de segurança da base atual é guardada automaticamente antes, para você poder
            reverter. Tudo local, sem internet.
          </p>
          <div class="db-row">
            <button class="db-btn" :disabled="backupBusy || restoreBusy" @click="doBackup">
              {{ backupBusy ? "Fazendo backup…" : "Fazer backup" }}
            </button>
            <button
              class="db-btn db-btn--warn"
              :disabled="backupBusy || restoreBusy"
              @click="doRestore"
            >
              {{ restoreBusy ? "Restaurando…" : "Restaurar backup" }}
            </button>
          </div>
          <p v-if="dbMsg" class="db-feedback db-feedback--ok">✓ {{ dbMsg }}</p>
          <p v-if="dbErr" class="db-feedback db-feedback--err">⚠ {{ dbErr }}</p>
        </div>
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
.field-path { font-size: 0.8125rem; color: var(--clr-text-secondary); margin-top: 0.5rem; word-break: break-all; }
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

.pw-row { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; }
.pw-state { font-size: 0.8125rem; font-weight: 500; }
.pw-state.on { color: var(--clr-positive, #107c10); }
.pw-state.off { color: var(--clr-text-muted); }
.forget-btn {
  padding: 0.35rem 0.9rem;
  background: transparent;
  color: var(--clr-negative);
  border: 1px solid var(--clr-negative);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.8125rem;
  font-weight: 600;
  font-family: var(--font-body);
  transition: background 0.1s;
}
.forget-btn:hover:not(:disabled) { background: rgba(209,52,56,0.08); }
.forget-btn:disabled { opacity: 0.6; cursor: default; }

.db-row { display: flex; gap: 0.75rem; flex-wrap: wrap; margin-top: 0.25rem; }
.db-btn {
  padding: 0.45rem 1rem;
  background: var(--clr-surface);
  color: var(--clr-text-primary);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.8125rem;
  font-weight: 600;
  font-family: var(--font-body);
  transition: background 0.1s, border-color 0.1s;
  white-space: nowrap;
}
.db-btn:hover:not(:disabled) { border-color: var(--clr-accent); }
.db-btn:disabled { opacity: 0.6; cursor: default; }
.db-btn--warn { color: var(--clr-negative); border-color: var(--clr-negative); }
.db-btn--warn:hover:not(:disabled) { background: rgba(209,52,56,0.08); border-color: var(--clr-negative); }
.db-feedback { font-size: 0.8125rem; font-weight: 500; margin-top: 0.6rem; word-break: break-all; }
.db-feedback--ok { color: var(--clr-positive, #107c10); }
.db-feedback--err { color: var(--clr-negative); }

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
