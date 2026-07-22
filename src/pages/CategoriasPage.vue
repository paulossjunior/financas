<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { useSettingsStore } from "@/stores/settings.store";
import { useInvoiceStore } from "@/stores/invoice.store";
import {
  listRecurringCategories,
  setCategoryRecurring,
  recurringSuggestions,
  dismissRecurringSuggestion,
} from "@/services/tauri.service";
import type { RecurringCategoryInfo, RecurringSuggestion } from "@/types/api.types";
import MappingPage from "@/pages/MappingPage.vue";

const settings = useSettingsStore();
const store = useInvoiceStore();

type Tab = "regras" | "mapeamento";
const activeTab = ref<Tab>("regras");

const recInfos = ref<RecurringCategoryInfo[]>([]);
const suggestions = ref<RecurringSuggestion[]>([]);

// filters
const search = ref("");
const onlyRecurring = ref(false);

// inline keyword add: which category is currently accepting a new keyword
const addingFor = ref<string | null>(null);
const newKw = ref("");

// ── recurring info keyed by category name ──
const recMap = computed(() => {
  const m = new Map<string, RecurringCategoryInfo>();
  for (const r of recInfos.value) m.set(r.category, r);
  return m;
});

function isRecurring(name: string): boolean {
  return recMap.value.has(name);
}

// ── filtered rows ──
const filteredGroups = computed(() => {
  const q = search.value.trim().toLowerCase();
  return settings.categoryGroups.filter((g) => {
    if (q && !g.name.toLowerCase().includes(q)) return false;
    if (onlyRecurring.value && !isRecurring(g.name)) return false;
    return true;
  });
});

// ── Tab 2 badge: number of distinct "Outros" merchants in the queue ──
// Mirrors MappingPage's derivation so the count matches the triage list.
function deriveKeyword(desc: string): string {
  let s = desc.replace(/\(\d+\/\d+\)/g, " ");
  s = s.replace(/[*#]+/g, " ");
  s = s.replace(/\s+/g, " ").trim().toUpperCase();
  return s || desc.trim().toUpperCase();
}
const queueCount = computed(() => {
  const keys = new Set<string>();
  for (const t of store.allTransactions) {
    if (t.is_reversal || t.category !== "Outros") continue;
    const k = deriveKeyword(t.description);
    if (k) keys.add(k);
  }
  return keys.size;
});

// ── formatting ──
function fmtBRL(v: string | number): string {
  const n = typeof v === "string" ? parseFloat(v) || 0 : v;
  return n.toLocaleString("pt-BR", { style: "currency", currency: "BRL", minimumFractionDigits: 2 });
}

const MONTHS_ABBR = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];
function ymShort(ym: string): string {
  const [y, m] = ym.split("-");
  return `${MONTHS_ABBR[parseInt(m, 10) - 1] ?? m}/${(y ?? "").slice(2)}`;
}
/** Human vigência range, e.g. "jan–mar/26" or "jan/25–mar/26". "" when open-ended. */
function vigRange(info: RecurringCategoryInfo): string {
  const s = info.start_month;
  const e = info.end_month;
  if (s && e) {
    const [sy, sm] = s.split("-");
    const [ey, em] = e.split("-");
    if (sy === ey) {
      return `${MONTHS_ABBR[parseInt(sm, 10) - 1] ?? sm}–${MONTHS_ABBR[parseInt(em, 10) - 1] ?? em}/${sy.slice(2)}`;
    }
    return `${ymShort(s)}–${ymShort(e)}`;
  }
  if (s) return `desde ${ymShort(s)}`;
  if (e) return `até ${ymShort(e)}`;
  return "";
}
function isFinite_(info: RecurringCategoryInfo): boolean {
  return !!(info.start_month || info.end_month);
}

function baselineNote(info: RecurringCategoryInfo): string {
  return info.varies ? "média 3m · varia" : "valor fixo";
}

const ORIGIN_LABEL: Record<string, string> = {
  extrato: "Extrato",
  fatura: "Fatura",
  manual: "Manual",
  baseline: "Baseline",
};
function originClass(origin: string | null): string {
  if (origin === "extrato") return "ext";
  if (origin === "fatura") return "fat";
  return "base";
}

// ── data loading ──
async function loadRecurring(): Promise<void> {
  try {
    const [infos, sugg] = await Promise.all([listRecurringCategories(), recurringSuggestions()]);
    recInfos.value = infos;
    suggestions.value = sugg;
  } catch {
    // Backend may have no data yet — leave lists empty rather than surfacing an error.
  }
}

/** Persist local category/keyword edits, then refresh derived state. */
async function persist(): Promise<void> {
  await settings.saveCategories();
  await Promise.all([loadRecurring(), store.loadAllTransactions().catch(() => {})]);
}

onMounted(async () => {
  await settings.loadConfig();
  await Promise.all([loadRecurring(), store.loadAllTransactions().catch(() => {})]);
});

// ── recurring toggle + vigência ──
async function toggleRecurring(name: string): Promise<void> {
  const on = isRecurring(name);
  await setCategoryRecurring(name, !on);
  await loadRecurring();
}

async function onVigChange(name: string, which: "start" | "end", e: Event): Promise<void> {
  const val = (e.target as HTMLInputElement).value || null;
  const info = recMap.value.get(name);
  const start = which === "start" ? val : info?.start_month ?? null;
  const end = which === "end" ? val : info?.end_month ?? null;
  await setCategoryRecurring(name, true, start, end);
  await loadRecurring();
}

// ── suggestions ──
async function markSuggestion(s: RecurringSuggestion): Promise<void> {
  await setCategoryRecurring(s.category, true);
  await loadRecurring();
}
async function ignoreSuggestion(s: RecurringSuggestion): Promise<void> {
  await dismissRecurringSuggestion(s.category);
  await loadRecurring();
}

// ── keywords ──
function startAdd(name: string): void {
  addingFor.value = name;
  newKw.value = "";
}
async function commitAdd(name: string): Promise<void> {
  const kw = newKw.value.trim();
  addingFor.value = null;
  newKw.value = "";
  if (!kw) return;
  settings.addKeyword(name, kw);
  await persist();
}
function cancelAdd(): void {
  addingFor.value = null;
  newKw.value = "";
}
async function removeKw(name: string, kw: string): Promise<void> {
  settings.removeKeyword(name, kw);
  await persist();
}
function focusEl(vnode: { el?: HTMLElement | null }): void {
  vnode.el?.focus();
}

// ── category name (rename / delete) ──
async function onRename(oldName: string, e: Event): Promise<void> {
  const val = (e.target as HTMLInputElement).value.trim();
  if (!val || val === oldName) return;
  settings.renameCategory(oldName, val);
  await persist();
}
async function onNewCategory(): Promise<void> {
  settings.addCategory("Nova Categoria");
  await persist();
}
async function onDelete(name: string): Promise<void> {
  settings.deleteCategory(name);
  await persist();
}
</script>

<template>
  <div class="cats">
    <header class="top">
      <p class="eyebrow">Regras & recorrência</p>
      <h1>Categorias</h1>
      <p class="sub">Defina como os lançamentos são classificados e o que se repete todo mês.</p>
    </header>

    <!-- TABS -->
    <div class="tabs" role="tablist">
      <button
        class="tab"
        :class="{ on: activeTab === 'regras' }"
        role="tab"
        @click="activeTab = 'regras'"
      >
        Categorias & Regras <span class="c">{{ settings.categoryGroups.length }}</span>
      </button>
      <button
        class="tab"
        :class="{ on: activeTab === 'mapeamento' }"
        role="tab"
        @click="activeTab = 'mapeamento'"
      >
        Mapeamento de despesas <span class="c">{{ queueCount }} na fila</span>
      </button>
    </div>

    <!-- ── TAB 1: Categorias & Regras ── -->
    <div v-show="activeTab === 'regras'" class="tabpane">
      <div v-if="settings.error" class="msg err">⚠ {{ settings.error }}</div>

      <!-- suggestion banners -->
      <div v-for="s in suggestions" :key="s.category" class="detect">
        <span class="ic">💡</span>
        <span class="tx">
          <b>«{{ s.category }}»</b> apareceu em {{ s.months_seen }}
          {{ s.months_seen > 1 ? "meses" : "mês" }} (~<b>{{ fmtBRL(s.avg) }}</b>/mês).
          Marcar como recorrente?
        </span>
        <span class="acts">
          <button class="mini" @click="markSuggestion(s)">Marcar recorrente</button>
          <button class="mini ghost" @click="ignoreSuggestion(s)">Ignorar</button>
        </span>
      </div>

      <!-- toolbar -->
      <div class="toolbar">
        <input v-model="search" class="search" placeholder="🔎 Buscar categoria…" />
        <span class="grow" />
        <button
          class="btn ghost"
          :class="{ active: onlyRecurring }"
          @click="onlyRecurring = !onlyRecurring"
        >
          ↕ Só recorrentes
        </button>
        <button class="btn" @click="onNewCategory">+ Nova categoria</button>
      </div>

      <!-- table -->
      <div class="tablewrap">
        <table>
          <thead>
            <tr>
              <th>Categoria</th>
              <th class="hide">Palavras-chave</th>
              <th class="c">Recorrente</th>
              <th class="r">Baseline /mês</th>
              <th>Origem</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="!filteredGroups.length">
              <td colspan="5" class="empty">Nenhuma categoria encontrada.</td>
            </tr>
            <tr v-for="g in filteredGroups" :key="g.name">
              <!-- Categoria -->
              <td class="cat">
                <div class="cat-cell">
                  <input
                    class="cat-name"
                    :value="g.name"
                    title="Renomear categoria"
                    @change="onRename(g.name, $event)"
                  />
                  <button class="del" title="Excluir categoria" @click="onDelete(g.name)">✕</button>
                </div>
              </td>

              <!-- Palavras-chave -->
              <td class="hide">
                <div class="kw">
                  <span v-for="kw in g.keywords" :key="kw" class="chip-kw">
                    {{ kw }}
                    <button class="kw-x" :title="`Remover '${kw}'`" @click="removeKw(g.name, kw)">×</button>
                  </span>
                  <input
                    v-if="addingFor === g.name"
                    v-model="newKw"
                    class="kw-add-input"
                    placeholder="palavra-chave…"
                    @vue:mounted="focusEl"
                    @keyup.enter="commitAdd(g.name)"
                    @keyup.escape="cancelAdd"
                    @blur="commitAdd(g.name)"
                  />
                  <button v-else class="chip-kw add" title="Adicionar palavra-chave" @click="startAdd(g.name)">
                    +
                  </button>
                </div>
              </td>

              <!-- Recorrente -->
              <td class="c">
                <div class="rec">
                  <button
                    class="track"
                    :class="{ on: isRecurring(g.name) }"
                    role="switch"
                    :aria-checked="isRecurring(g.name)"
                    :title="isRecurring(g.name) ? 'Recorrente — clique para desativar' : 'Marcar como recorrente'"
                    @click="toggleRecurring(g.name)"
                  >
                    <span class="knob" />
                  </button>
                </div>
                <template v-if="recMap.get(g.name) as RecurringCategoryInfo | undefined">
                  <div class="vig">
                    <template v-if="isFinite_(recMap.get(g.name)!)">
                      <b>{{ vigRange(recMap.get(g.name)!) }}</b> · finita
                    </template>
                    <template v-else>contínua</template>
                  </div>
                  <div class="vig-edit">
                    <label>
                      <span>Início</span>
                      <input
                        type="month"
                        :value="recMap.get(g.name)!.start_month ?? ''"
                        @change="onVigChange(g.name, 'start', $event)"
                      />
                    </label>
                    <label>
                      <span>Fim</span>
                      <input
                        type="month"
                        :value="recMap.get(g.name)!.end_month ?? ''"
                        @change="onVigChange(g.name, 'end', $event)"
                      />
                    </label>
                  </div>
                </template>
              </td>

              <!-- Baseline /mês -->
              <td class="r">
                <template v-if="isRecurring(g.name) && recMap.get(g.name)?.baseline">
                  <span class="val">{{ fmtBRL(recMap.get(g.name)!.baseline!) }}</span>
                  <div class="note">{{ baselineNote(recMap.get(g.name)!) }}</div>
                </template>
                <span v-else class="muted">—</span>
              </td>

              <!-- Origem -->
              <td>
                <span
                  v-if="isRecurring(g.name) && recMap.get(g.name)?.origin"
                  class="tag"
                  :class="originClass(recMap.get(g.name)!.origin)"
                >
                  {{ ORIGIN_LABEL[recMap.get(g.name)!.origin as string] ?? recMap.get(g.name)!.origin }}
                </span>
                <span v-else class="muted">{{ isRecurring(g.name) ? "—" : "variável" }}</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <p class="foot">
        <b>Recorrente</b> e <b>vigência</b> são editáveis. <b>Baseline</b> e <b>Origem</b> são
        derivados do histórico real (somente leitura).
      </p>
    </div>

    <!-- ── TAB 2: Mapeamento de despesas ── -->
    <div v-if="activeTab === 'mapeamento'" class="tabpane">
      <MappingPage />
    </div>
  </div>
</template>

<style scoped>
.cats {
  --ink: var(--clr-text-primary);
  --ink-2: var(--clr-text-secondary);
  --ink-3: var(--clr-text-muted);
  --stroke: var(--clr-stroke);
  --stroke-soft: var(--clr-stroke-soft);
  --surface: var(--clr-surface);
  --surface-2: var(--clr-surface-alt);
  --accent: var(--clr-accent);
  --accent-hover: var(--clr-accent-hover);
  --accent-soft: var(--clr-accent-light);
  --amber: var(--clr-amber);
  --amber-soft: var(--clr-amber-soft);
  --red: var(--clr-negative);
  --red-soft: var(--clr-red-soft);
  --radius: var(--radius-lg);
  --radius-sm: var(--radius-md);
  padding: 1.75rem 2rem 4rem;
  max-width: 1320px;
  margin: 0 auto;
  color: var(--ink);
  font-variant-numeric: tabular-nums;
}

.top { margin-bottom: 0.5rem; }
.eyebrow { font-size: 11px; letter-spacing: .12em; text-transform: uppercase; color: var(--accent); font-weight: 700; margin-bottom: 6px; }
h1 { font-size: 28px; font-weight: 800; letter-spacing: -.02em; margin-bottom: 6px; }
.sub { color: var(--ink-2); font-size: 14px; max-width: 72ch; }

/* TABS */
.tabs { display: flex; gap: 6px; margin: 18px 0 6px; border-bottom: 1px solid var(--stroke); flex-wrap: wrap; }
.tab { position: relative; font-size: 14px; font-weight: 700; color: var(--ink-2); background: none; border: none; padding: 10px 4px; margin-right: 18px; cursor: pointer; font-family: inherit; }
.tab .c { margin-left: 6px; font-size: 11px; font-weight: 700; color: var(--ink-3); background: var(--surface-2); padding: 1px 7px; border-radius: 100px; }
.tab.on { color: var(--ink); }
.tab.on::after { content: ""; position: absolute; left: 0; right: 0; bottom: -1px; height: 2.5px; background: var(--accent); border-radius: 2px; }

.tabpane { padding-top: 6px; }

.msg { padding: 10px 14px; border-radius: 8px; font-size: 13px; margin: 12px 0; }
.msg.err { background: var(--red-soft); color: var(--red); border: 1px solid var(--red); }

/* suggestion banner */
.detect { display: flex; align-items: center; gap: 12px; background: var(--accent-soft); border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent); border-radius: var(--radius); padding: 11px 14px; margin: 14px 0; flex-wrap: wrap; }
.detect .ic { font-size: 18px; flex: none; }
.detect .tx { font-size: 13px; flex: 1; min-width: 200px; }
.detect .tx b { font-weight: 700; }
.detect .acts { display: flex; gap: 7px; flex: none; }
.mini { font-size: 12px; font-weight: 700; border-radius: 7px; padding: 6px 11px; cursor: pointer; border: 1px solid var(--accent); background: var(--accent); color: #fff; font-family: inherit; }
.mini.ghost { background: transparent; color: var(--accent); }

/* toolbar */
.toolbar { display: flex; align-items: center; gap: 10px; margin: 16px 0 10px; flex-wrap: wrap; }
.toolbar .grow { flex: 1; }
.btn { font-family: inherit; font-size: 13px; font-weight: 700; border-radius: 9px; padding: 8px 14px; cursor: pointer; border: 1px solid var(--accent); background: var(--accent); color: #fff; }
.btn.ghost { background: transparent; color: var(--accent); }
.btn.ghost.active { background: var(--accent-soft); }
.search { font-size: 13px; padding: 8px 12px; border: 1px solid var(--stroke); border-radius: 9px; background: var(--surface); color: var(--ink); min-width: 200px; outline: none; font-family: inherit; }
.search:focus { border-color: var(--accent); }

/* table */
.tablewrap { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; font-size: 13.5px; }
th { text-align: left; font-size: 10.5px; letter-spacing: .06em; text-transform: uppercase; color: var(--ink-3); font-weight: 700; padding: 0 10px 9px; border-bottom: 1px solid var(--stroke-soft); }
th.r, td.r { text-align: right; }
th.c, td.c { text-align: center; }
td { padding: 12px 10px; border-bottom: 1px solid var(--stroke-soft); vertical-align: middle; }
tr:last-child td { border-bottom: none; }
.empty { text-align: center; color: var(--ink-3); padding: 24px 10px; }

/* category name cell */
.cat { font-weight: 700; }
.cat-cell { display: flex; align-items: center; gap: 4px; }
.cat-name { flex: 1; min-width: 0; border: 1px solid transparent; background: transparent; font-size: 13.5px; font-weight: 700; color: var(--ink); padding: 3px 6px; border-radius: 6px; font-family: inherit; }
.cat-name:hover { border-color: var(--stroke-soft); }
.cat-name:focus { outline: none; border-color: var(--accent); background: var(--surface); }
.del { opacity: 0; background: transparent; border: none; color: var(--red); cursor: pointer; font-size: 12px; padding: 3px 6px; border-radius: 6px; transition: opacity .12s; }
tr:hover .del { opacity: 1; }
.del:hover { background: var(--red-soft); }

/* keyword chips */
.kw { display: flex; flex-wrap: wrap; gap: 4px; align-items: center; }
.chip-kw { display: inline-flex; align-items: center; gap: 3px; font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace; font-size: 11px; color: var(--ink-2); background: var(--surface-2); border: 1px solid var(--stroke-soft); border-radius: 6px; padding: 1px 7px; }
.kw-x { background: transparent; border: none; color: var(--ink-3); cursor: pointer; font-size: 12px; line-height: 1; padding: 0; }
.kw-x:hover { color: var(--red); }
.chip-kw.add { color: var(--accent); border-style: dashed; border-color: color-mix(in srgb, var(--accent) 40%, transparent); cursor: pointer; background: transparent; font-family: inherit; }
.kw-add-input { font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace; font-size: 11px; padding: 2px 7px; border: 1px solid var(--accent); border-radius: 6px; background: var(--surface); color: var(--ink); outline: none; width: 130px; }

/* baseline */
.val { font-weight: 800; letter-spacing: -.01em; }
.muted { color: var(--ink-3); }
.note { font-size: 11px; color: var(--ink-3); }

/* origin tag */
.tag { display: inline-flex; align-items: center; gap: 4px; font-size: 11px; font-weight: 700; padding: 2px 9px; border-radius: 100px; border: 1px solid var(--stroke); color: var(--ink-2); background: var(--surface-2); }
.tag.ext { color: var(--accent); border-color: color-mix(in srgb, var(--accent) 35%, transparent); background: var(--accent-soft); }
.tag.fat { color: var(--amber); border-color: color-mix(in srgb, var(--amber) 35%, transparent); background: var(--amber-soft); }
.tag.base { color: var(--ink-3); border-style: dashed; }

/* switch + vigência */
.rec { display: flex; align-items: center; gap: 9px; justify-content: center; }
.track { width: 36px; height: 21px; border-radius: 100px; background: var(--stroke); position: relative; flex: none; border: none; cursor: pointer; padding: 0; }
.track.on { background: var(--accent); }
.knob { position: absolute; top: 2px; left: 2px; width: 17px; height: 17px; border-radius: 50%; background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,.25); transition: left .12s; }
.track.on .knob { left: 17px; }
.vig { font-size: 10.5px; color: var(--ink-3); font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace; margin-top: 6px; }
.vig b { color: var(--amber); font-weight: 700; }
.vig-edit { display: flex; gap: 8px; justify-content: center; margin-top: 6px; }
.vig-edit label { display: flex; flex-direction: column; gap: 2px; font-size: 9px; color: var(--ink-3); text-transform: uppercase; letter-spacing: .04em; }
.vig-edit input { font-size: 11px; padding: 2px 4px; border: 1px solid var(--stroke); border-radius: 6px; background: var(--surface); color: var(--ink); font-family: inherit; outline: none; }
.vig-edit input:focus { border-color: var(--accent); }

.foot { margin-top: 1.75rem; font-size: 12px; color: var(--ink-3); border-top: 1px solid var(--stroke-soft); padding-top: 14px; }
.foot b { color: var(--ink-2); }

@media (max-width: 720px) {
  th.hide, td.hide { display: none; }
  .cats { padding: 1.25rem 1rem 3rem; }
}
</style>
