<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import {
  previewBankStatement,
  importBankStatement,
  listBankEntries,
  removeBankEntry,
  clearBankEntries,
} from "@/services/tauri.service";
import type { BankEntry, StatementPreview } from "@/types/api.types";

const entries = ref<BankEntry[]>([]);
const preview = ref<StatementPreview | null>(null);
const pendingPath = ref<string | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const flash = ref<string | null>(null);

const num = (s: string) => parseFloat(s) || 0;
const brl = (s: string | number) => {
  const v = typeof s === "string" ? num(s) : s;
  return (v < 0 ? "− " : v > 0 ? "+ " : "") + "R$ " + Math.abs(v).toLocaleString("pt-BR", { minimumFractionDigits: 2 });
};
const day = (iso: string) => { const p = iso.split("-"); return p.length === 3 ? `${p[2]}/${p[1]}` : iso; };
const REASON: Record<string, string> = { fatura: "já vem da fatura", salario: "já vem do contracheque", interno: "transferência interna" };

const total = computed(() => entries.value.reduce((a, e) => a + num(e.amount), 0));

async function load() {
  try { entries.value = await listBankEntries(); } catch (e) { error.value = msg(e); }
}
function msg(e: unknown) { return e instanceof Error ? e.message : String(e); }

async function pick() {
  error.value = null; flash.value = null;
  const sel = await open({ multiple: false, filters: [{ name: "Extrato", extensions: ["xls", "xlsx"] }] });
  if (!sel || Array.isArray(sel)) return;
  loading.value = true;
  try {
    pendingPath.value = sel;
    preview.value = await previewBankStatement(sel);
  } catch (e) { error.value = msg(e); preview.value = null; pendingPath.value = null; }
  finally { loading.value = false; }
}

async function confirmImport() {
  if (!pendingPath.value) return;
  loading.value = true; error.value = null;
  try {
    const n = await importBankStatement(pendingPath.value);
    flash.value = `${n} lançamento${n === 1 ? "" : "s"} importado${n === 1 ? "" : "s"}.`;
    preview.value = null; pendingPath.value = null;
    await load();
  } catch (e) { error.value = msg(e); }
  finally { loading.value = false; }
}
function cancelPreview() { preview.value = null; pendingPath.value = null; }

async function remove(id: string) {
  try { await removeBankEntry(id); await load(); } catch (e) { error.value = msg(e); }
}
async function clearAll() {
  try { await clearBankEntries(); await load(); } catch (e) { error.value = msg(e); }
}

onMounted(load);
</script>

<template>
  <div class="page">
    <header class="top">
      <div>
        <p class="eyebrow">Extrato · contas bancárias</p>
        <h1>Importar extrato</h1>
        <p class="sub">Lê o extrato do banco (.xls), categoriza os lançamentos e soma no painel. Ignora o que já é contado (fatura do cartão, salário com contracheque, transferências entre suas contas).</p>
      </div>
      <button class="btn primary" :disabled="loading" @click="pick">{{ loading ? "Lendo…" : "↑ Importar .xls" }}</button>
    </header>

    <p v-if="error" class="state err">⚠ {{ error }}</p>
    <p v-if="flash" class="state ok">✓ {{ flash }}</p>

    <!-- Preview / review -->
    <section v-if="preview" class="card">
      <div class="rev-head">
        <div>
          <h2>Prévia</h2>
          <p class="sub2">Titular <b>{{ preview.holder }}</b> · conta <b>{{ preview.account }}</b></p>
        </div>
        <div class="rev-actions">
          <button class="btn" @click="cancelPreview">Cancelar</button>
          <button class="btn primary" :disabled="loading || !preview.included.length" @click="confirmImport">
            Confirmar importação ({{ preview.included.length }})
          </button>
        </div>
      </div>

      <h3>Entram <span class="mut">{{ preview.included.length }}</span></h3>
      <div class="tblwrap" v-if="preview.included.length">
        <table>
          <thead><tr><th>Data</th><th>Descrição</th><th>Categoria</th><th>Tipo</th><th class="r">Valor</th></tr></thead>
          <tbody>
            <tr v-for="c in preview.included" :key="c.id">
              <td class="dt">{{ day(c.date) }}</td>
              <td class="ds">{{ c.description }}</td>
              <td><span class="bdg cat">{{ c.category }}</span></td>
              <td><span class="bdg" :class="c.kind">{{ c.kind === "income" ? "crédito" : "débito" }}</span></td>
              <td class="r" :class="c.kind === 'income' ? 'pos' : 'neg'">{{ brl(c.amount) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-else class="mut">Nada novo para importar.</p>

      <h3>Excluídos <span class="mut">{{ preview.excluded.length }}</span></h3>
      <div class="tblwrap" v-if="preview.excluded.length">
        <table class="ex">
          <thead><tr><th>Data</th><th>Descrição</th><th class="r">Valor</th><th>Motivo</th></tr></thead>
          <tbody>
            <tr v-for="c in preview.excluded" :key="c.id">
              <td class="dt">{{ day(c.date) }}</td>
              <td class="ds strike">{{ c.description }}</td>
              <td class="r strike">{{ brl(c.amount) }}</td>
              <td><span class="bdg" :class="c.reason">{{ REASON[c.reason] ?? c.reason }}</span></td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-else class="mut">Nenhum lançamento excluído.</p>
    </section>

    <!-- Imported list -->
    <section class="card">
      <div class="rev-head">
        <h2>Lançamentos importados <span class="mut">{{ entries.length }}</span></h2>
        <button v-if="entries.length" class="btn danger" @click="clearAll">Limpar tudo</button>
      </div>
      <div class="tblwrap" v-if="entries.length">
        <table>
          <thead><tr><th>Data</th><th>Descrição</th><th>Categoria</th><th class="r">Valor</th><th></th></tr></thead>
          <tbody>
            <tr v-for="e in entries" :key="e.id">
              <td class="dt">{{ day(e.date) }}</td>
              <td class="ds">{{ e.description }}</td>
              <td><span class="bdg cat">{{ e.category }}</span></td>
              <td class="r" :class="e.kind === 'income' ? 'pos' : 'neg'">{{ brl(e.amount) }}</td>
              <td class="r"><button class="x" title="Remover" @click="remove(e.id)">✕</button></td>
            </tr>
            <tr class="tot"><td colspan="3">Total</td><td class="r" :class="total >= 0 ? 'pos' : 'neg'">{{ brl(total) }}</td><td></td></tr>
          </tbody>
        </table>
      </div>
      <p v-else class="mut">Nenhum extrato importado ainda. Clique em <b>Importar .xls</b>.</p>
    </section>
  </div>
</template>

<style scoped>
.page { max-width: 1000px; margin: 0 auto; padding: 8px 4px 60px; }
.top { display: flex; align-items: flex-start; gap: 16px; margin-bottom: 16px; }
.eyebrow { font-size: 11px; letter-spacing: .12em; text-transform: uppercase; color: var(--clr-accent); font-weight: 700; margin: 0 0 6px; }
h1 { font-size: 1.5rem; font-weight: 800; letter-spacing: -.02em; margin: 0; }
.sub { color: var(--clr-text-secondary); font-size: 14px; margin: 8px 0 0; max-width: 68ch; }
.btn { margin-left: auto; flex: none; font-family: inherit; font-size: 13px; font-weight: 700; padding: 9px 16px; border-radius: 9px; border: 1px solid var(--clr-stroke); background: var(--clr-surface); color: var(--clr-text-primary); cursor: pointer; }
.btn.primary { background: var(--clr-accent); color: #fff; border-color: var(--clr-accent); }
.btn.danger:hover { border-color: var(--clr-negative); color: var(--clr-negative); }
.btn:disabled { opacity: .55; cursor: default; }
.state { font-size: 13.5px; margin: 0 0 12px; } .state.err { color: var(--clr-negative); } .state.ok { color: var(--clr-accent); }
.card { background: var(--clr-surface); border: 1px solid var(--clr-stroke); border-radius: var(--radius-lg, 14px); box-shadow: var(--shadow-sm); padding: 18px 20px; margin-bottom: 16px; }
.rev-head { display: flex; align-items: center; gap: 12px; margin-bottom: 10px; }
.rev-head h2 { font-size: 1rem; font-weight: 800; margin: 0; }
.rev-actions { margin-left: auto; display: flex; gap: 8px; }
.rev-actions .btn { margin-left: 0; }
.sub2 { font-size: 12.5px; color: var(--clr-text-secondary); margin: 4px 0 0; }
h3 { font-size: 13px; font-weight: 800; margin: 16px 0 6px; } .mut { color: var(--clr-text-muted, #7c8b83); font-weight: 600; }
.tblwrap { overflow-x: auto; border: 1px solid var(--clr-stroke); border-radius: 10px; }
table { width: 100%; border-collapse: collapse; font-size: 13px; font-variant-numeric: tabular-nums; min-width: 520px; }
th, td { padding: 8px 11px; text-align: left; border-bottom: 1px solid var(--clr-stroke-soft, var(--clr-stroke)); white-space: nowrap; }
th { font-size: 10.5px; text-transform: uppercase; letter-spacing: .03em; color: var(--clr-text-muted, #7c8b83); font-weight: 700; background: var(--clr-surface-alt, transparent); }
th.r, td.r { text-align: right; } tr:last-child td { border-bottom: none; }
.ds { white-space: normal; } .dt { color: var(--clr-text-muted, #7c8b83); }
.neg { color: var(--clr-negative); font-weight: 700; } .pos { color: var(--clr-accent); font-weight: 700; }
.strike { text-decoration: line-through; opacity: .7; }
.tot td { font-weight: 800; background: var(--clr-surface-alt, transparent); }
.bdg { font-size: 10px; font-weight: 800; text-transform: uppercase; padding: 1px 7px; border-radius: 999px; }
.bdg.cat { background: var(--clr-accent-light, #d3ebe4); color: var(--clr-accent); }
.bdg.expense { background: var(--clr-red-soft, #f6d9d9); color: var(--clr-negative); }
.bdg.income { background: var(--clr-accent-light, #d3ebe4); color: var(--clr-accent); }
.bdg.fatura { background: #e9e3ff; color: #6d4aff; }
.bdg.salario { background: var(--clr-amber-soft, #f7e6cf); color: var(--clr-amber); }
.bdg.interno { background: var(--clr-surface-alt, #eef1f0); color: var(--clr-text-secondary); }
.x { font-family: inherit; font-size: 12px; border: 1px solid var(--clr-stroke); background: var(--clr-surface); color: var(--clr-text-secondary); border-radius: 6px; padding: 3px 8px; cursor: pointer; }
.x:hover { border-color: var(--clr-negative); color: var(--clr-negative); }
</style>
